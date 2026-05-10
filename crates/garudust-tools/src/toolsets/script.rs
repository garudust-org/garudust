use std::path::{Path, PathBuf};

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::process::Command;

// ── YAML definition ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ScriptToolDef {
    name: String,
    description: String,
    #[serde(default = "default_toolset")]
    toolset: String,
    /// JSON Schema for parameters. Supports {param_name} placeholders in command.
    #[serde(default = "empty_schema")]
    schema: Value,
    command: String,
    /// Whether this tool requires approval before running. Defaults to true.
    #[serde(default = "default_destructive")]
    destructive: bool,
}

fn default_toolset() -> String {
    "script".into()
}

fn default_destructive() -> bool {
    true
}

fn empty_schema() -> Value {
    json!({ "type": "object", "properties": {} })
}

// ── Runtime struct ────────────────────────────────────────────────────────────

pub struct ScriptTool {
    name: String,
    description: String,
    toolset: String,
    schema: Value,
    command: String,
    destructive: bool,
    /// The tool's folder; `sh -c` runs here so `./run.py` and sibling files
    /// resolve correctly. `$TOOL_DIR` is also set in the environment.
    tool_dir: PathBuf,
}

// ── Shell quoting ─────────────────────────────────────────────────────────────

/// Wrap `s` in POSIX single-quotes so it is safe to embed in a shell command.
/// Internal single-quotes are replaced with `'\''` (end-quote, escaped-quote,
/// re-open-quote), which is the standard POSIX technique.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Substitute `{param_name}` placeholders in `template` with shell-quoted
/// values extracted from `params`.
///
/// Placeholders that have no matching key in `params` are left unchanged so
/// the shell produces a visible error rather than silently passing empty data.
fn substitute(template: &str, params: &Value) -> String {
    let mut result = template.to_string();
    if let Some(obj) = params.as_object() {
        for (key, val) in obj {
            let placeholder = format!("{{{key}}}");
            let value = match val {
                Value::String(s) => shell_quote(s),
                // Numbers and booleans are safe without quoting, but quote
                // anyway for consistency.
                other => shell_quote(&other.to_string()),
            };
            result = result.replace(&placeholder, &value);
        }
    }
    result
}

// ── Tool impl ─────────────────────────────────────────────────────────────────

/// Parse a `.env` file into key-value pairs to forward to subprocess environments.
/// Skips blank lines, comments, and malformed entries; never panics.
fn read_dotenv(path: &std::path::Path) -> Vec<(String, String)> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

#[async_trait]
impl Tool for ScriptTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn toolset(&self) -> &str {
        &self.toolset
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    fn is_destructive(&self) -> bool {
        self.destructive
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = substitute(&self.command, &params);

        let dotenv_vars = read_dotenv(&ctx.config.home_dir.join(".env"));

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(&command)
            .current_dir(&self.tool_dir)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .env("LANG", "en_US.UTF-8")
            .env("TOOL_DIR", &self.tool_dir);
        for (k, v) in &dotenv_vars {
            cmd.env(k, v);
        }
        let out = cmd
            .output()
            .await
            .map_err(|e| ToolError::Execution(format!("script tool spawn error: {e}")))?;

        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();

        if !out.status.success() {
            let msg = if stderr.is_empty() { stdout } else { stderr };
            return Err(ToolError::Execution(format!(
                "script exited with {}: {msg}",
                out.status
            )));
        }

        Ok(ToolResult::ok("", stdout))
    }
}

// ── Loader ────────────────────────────────────────────────────────────────────

/// Load script tools from `<home_dir>/tools/`.
///
/// Each tool is a subdirectory containing a `tool.yaml` file:
///
/// ```text
/// ~/.garudust/tools/
/// └── my_tool/
///     ├── tool.yaml   ← metadata and command
///     └── run.py      ← script (referenced as ./run.py in command)
/// ```
///
/// The command runs with `current_dir` set to the tool folder so relative
/// paths like `./run.py` resolve correctly. `$TOOL_DIR` is also set.
/// Subdirectories without `tool.yaml` are silently skipped.
pub async fn load_script_tools(home_dir: &Path) -> Vec<ScriptTool> {
    let tools_dir = home_dir.join("tools");
    let mut tools = Vec::new();

    let Ok(mut entries) = tokio::fs::read_dir(&tools_dir).await else {
        return tools;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        let Ok(meta) = tokio::fs::metadata(&path).await else {
            continue;
        };

        if !meta.is_dir() {
            continue;
        }

        if let Some(tool) = load_tool_from_folder(&path).await {
            tools.push(tool);
        }
    }

    tools
}

async fn load_tool_from_folder(dir: &Path) -> Option<ScriptTool> {
    let yaml_path = dir.join("tool.yaml");

    // no tool.yaml — not a tool folder, silently skip
    let Ok(content) = tokio::fs::read_to_string(&yaml_path).await else {
        return None;
    };

    let def: ScriptToolDef = match serde_yaml::from_str(&content) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(path = %yaml_path.display(), "script tool: parse error: {e}");
            return None;
        }
    };

    if def.name.is_empty() || def.command.is_empty() {
        tracing::warn!(path = %yaml_path.display(), "script tool: name and command are required");
        return None;
    }

    tracing::info!(name = %def.name, dir = %dir.display(), "loaded script tool");
    Some(ScriptTool {
        name: def.name,
        description: def.description,
        toolset: def.toolset,
        schema: def.schema,
        command: def.command,
        destructive: def.destructive,
        tool_dir: dir.to_path_buf(),
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_quote_simple() {
        assert_eq!(shell_quote("hello"), "'hello'");
    }

    #[test]
    fn shell_quote_with_spaces() {
        assert_eq!(shell_quote("hello world"), "'hello world'");
    }

    #[test]
    fn shell_quote_escapes_single_quote() {
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn shell_quote_injection_attempt() {
        // Attacker tries to break out with: '; rm -rf /; echo '
        let malicious = "'; rm -rf /; echo '";
        let quoted = shell_quote(malicious);
        // Content is preserved but wrapped in outer single quotes; the shell
        // sees it as one literal argument. Embedded ' is escaped via '\''.
        assert!(quoted.starts_with('\''));
        assert!(quoted.ends_with('\''));
        assert!(
            quoted.contains("'\\''"),
            "embedded single-quote must be escaped"
        );
    }

    #[test]
    fn substitute_simple() {
        let cmd = "curl wttr.in/{city}?format=3";
        let params = json!({ "city": "Bangkok" });
        assert_eq!(substitute(cmd, &params), "curl wttr.in/'Bangkok'?format=3");
    }

    #[test]
    fn substitute_multiple_params() {
        let cmd = "echo {greeting} {name}";
        let params = json!({ "greeting": "hello", "name": "world" });
        assert_eq!(substitute(cmd, &params), "echo 'hello' 'world'");
    }

    #[test]
    fn substitute_missing_placeholder_unchanged() {
        let cmd = "echo {missing}";
        let params = json!({});
        assert_eq!(substitute(cmd, &params), "echo {missing}");
    }

    #[test]
    fn substitute_injection_safe() {
        let cmd = "curl wttr.in/{city}";
        let params = json!({ "city": "'; rm -rf /; echo '" });
        let result = substitute(cmd, &params);
        // The malicious city value is shell-quoted, so the shell sees it as one
        // literal argument. Embedded single-quotes are escaped via '\''.
        assert!(
            result.contains("curl wttr.in/'"),
            "path arg must open with single quote"
        );
        assert!(
            result.ends_with('\''),
            "path arg must close with single quote"
        );
        assert!(
            result.contains("'\\''"),
            "embedded single-quote must be escaped"
        );
    }

    #[test]
    fn parse_minimal_yaml() {
        let yaml = "
name: greet
description: Say hello
command: echo hello
";
        let def: ScriptToolDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.name, "greet");
        assert_eq!(def.toolset, "script");
        assert!(def.destructive);
    }

    #[test]
    fn parse_full_yaml() {
        let yaml = r#"
name: get_weather
description: Get weather
toolset: custom
destructive: false
schema:
  type: object
  properties:
    city:
      type: string
  required: [city]
command: "curl -s wttr.in/{city}?format=3"
"#;
        let def: ScriptToolDef = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(def.name, "get_weather");
        assert!(!def.destructive);
        assert_eq!(def.toolset, "custom");
    }

    #[tokio::test]
    async fn load_from_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn load_tool_from_folder_sets_tool_dir() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        let tool_folder = tools_dir.join("greet");
        tokio::fs::create_dir_all(&tool_folder).await.unwrap();
        tokio::fs::write(
            tool_folder.join("tool.yaml"),
            b"name: greet\ndescription: Say hello\ncommand: ./run.sh\n",
        )
        .await
        .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "greet");
        assert_eq!(tools[0].tool_dir, tool_folder);
    }

    #[tokio::test]
    async fn folder_without_tool_yaml_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        tokio::fs::create_dir_all(tools_dir.join("empty_folder"))
            .await
            .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn multiple_tool_folders_all_loaded() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");

        for name in ["weather", "analyze"] {
            let folder = tools_dir.join(name);
            tokio::fs::create_dir_all(&folder).await.unwrap();
            tokio::fs::write(
                folder.join("tool.yaml"),
                format!("name: {name}\ndescription: {name} tool\ncommand: ./run.sh\n").as_bytes(),
            )
            .await
            .unwrap();
        }

        let tools = load_script_tools(dir.path()).await;
        assert_eq!(tools.len(), 2);
    }

    #[tokio::test]
    async fn files_in_tools_dir_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.unwrap();
        // Flat .yaml files are no longer supported — must be ignored
        tokio::fs::write(
            tools_dir.join("greet.yaml"),
            b"name: greet\ndescription: Say hello\ncommand: echo hello\n",
        )
        .await
        .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn skip_folder_with_invalid_tool_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        let tool_folder = tools_dir.join("bad");
        tokio::fs::create_dir_all(&tool_folder).await.unwrap();
        tokio::fs::write(tool_folder.join("tool.yaml"), b"not: valid: yaml: [[[")
            .await
            .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }

    // ── execute() integration tests ───────────────────────────────────────────

    fn make_ctx() -> garudust_core::tool::ToolContext {
        use garudust_core::{
            budget::IterationBudget,
            config::AgentConfig,
            error::AgentError,
            memory::{MemoryContent, MemoryStore},
            tool::{ApprovalDecision, CommandApprover, SkillPermissions},
        };
        use std::sync::Arc;

        struct NopMemory;
        #[async_trait::async_trait]
        impl MemoryStore for NopMemory {
            async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
                Ok(MemoryContent::default())
            }
            async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
                Ok(())
            }
            async fn read_user_profile(&self) -> Result<String, AgentError> {
                Ok(String::new())
            }
            async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
                Ok(())
            }
        }

        struct AutoApprove;
        #[async_trait::async_trait]
        impl CommandApprover for AutoApprove {
            async fn approve(&self, _: &str, _: &str) -> ApprovalDecision {
                ApprovalDecision::Approved
            }
        }

        garudust_core::tool::ToolContext {
            session_id: "test".into(),
            agent_id: "test".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(AgentConfig::default()),
            approver: Arc::new(AutoApprove),
            sub_agent: None,
            skill_permissions: Arc::new(tokio::sync::RwLock::new(SkillPermissions::default())),
        }
    }

    fn make_tool(tool_dir: std::path::PathBuf, command: &str) -> ScriptTool {
        ScriptTool {
            name: "test_tool".into(),
            description: "test".into(),
            toolset: "script".into(),
            schema: json!({ "type": "object", "properties": {} }),
            command: command.into(),
            destructive: false,
            tool_dir,
        }
    }

    #[tokio::test]
    async fn execute_returns_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path().to_path_buf(), "echo hello");
        let ctx = make_ctx();
        let result = tool.execute(json!({}), &ctx).await.unwrap();
        assert_eq!(result.content.trim(), "hello");
    }

    #[tokio::test]
    async fn execute_substitutes_params() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path().to_path_buf(), "echo {msg}");
        let ctx = make_ctx();
        let result = tool.execute(json!({ "msg": "world" }), &ctx).await.unwrap();
        assert_eq!(result.content.trim(), "world");
    }

    #[tokio::test]
    async fn execute_runs_in_tool_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Write a file in the tool folder and verify the command can see it via ./
        tokio::fs::write(dir.path().join("sentinel.txt"), b"ok")
            .await
            .unwrap();
        let tool = make_tool(dir.path().to_path_buf(), "cat ./sentinel.txt");
        let ctx = make_ctx();
        let result = tool.execute(json!({}), &ctx).await.unwrap();
        assert_eq!(result.content.trim(), "ok");
    }

    #[tokio::test]
    async fn execute_sets_tool_dir_env() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path().to_path_buf(), "echo $TOOL_DIR");
        let ctx = make_ctx();
        let result = tool.execute(json!({}), &ctx).await.unwrap();
        assert_eq!(
            result.content.trim(),
            dir.path().to_str().unwrap(),
            "TOOL_DIR must equal the tool folder path"
        );
    }

    #[tokio::test]
    async fn execute_failed_command_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let tool = make_tool(dir.path().to_path_buf(), "exit 1");
        let ctx = make_ctx();
        let err = tool.execute(json!({}), &ctx).await.unwrap_err();
        assert!(
            matches!(err, garudust_core::error::ToolError::Execution(_)),
            "non-zero exit must produce ToolError::Execution"
        );
    }
}
