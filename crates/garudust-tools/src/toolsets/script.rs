use std::path::Path;

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
}

// ── Shell quoting ─────────────────────────────────────────────────────────────

/// Wrap `s` in POSIX single-quotes so it is safe to embed in a shell command.
/// Internal single-quotes are replaced with `'\''` (end-quote, escaped-quote,
/// re-open-quote), which is the standard POSIX technique.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
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

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let command = substitute(&self.command, &params);

        let out = Command::new("sh")
            .arg("-c")
            .arg(&command)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", std::env::var("HOME").unwrap_or_default())
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

/// Load all `*.yaml` files from `<home_dir>/tools/` as script tools.
/// Files that fail to parse are skipped with a warning.
pub async fn load_script_tools(home_dir: &Path) -> Vec<ScriptTool> {
    let tools_dir = home_dir.join("tools");
    let mut tools = Vec::new();

    let Ok(mut entries) = tokio::fs::read_dir(&tools_dir).await else {
        return tools;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
            continue;
        }

        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            tracing::warn!(path = %path.display(), "script tool: failed to read file");
            continue;
        };

        let def: ScriptToolDef = match serde_yaml::from_str(&content) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!(path = %path.display(), "script tool: parse error: {e}");
                continue;
            }
        };

        if def.name.is_empty() || def.command.is_empty() {
            tracing::warn!(path = %path.display(), "script tool: name and command are required");
            continue;
        }

        tracing::info!(name = %def.name, "loaded script tool");
        tools.push(ScriptTool {
            name: def.name,
            description: def.description,
            toolset: def.toolset,
            schema: def.schema,
            command: def.command,
            destructive: def.destructive,
        });
    }

    tools
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
        assert!(quoted.contains(r"'\''"), "embedded single-quote must be escaped");
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
        assert!(result.contains("curl wttr.in/'"), "path arg must open with single quote");
        assert!(result.ends_with('\''), "path arg must close with single quote");
        assert!(result.contains(r"'\''"), "embedded single-quote must be escaped");
    }

    #[test]
    fn parse_minimal_yaml() {
        let yaml = r#"
name: greet
description: Say hello
command: echo hello
"#;
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
    async fn load_valid_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.unwrap();
        tokio::fs::write(
            tools_dir.join("greet.yaml"),
            b"name: greet\ndescription: Say hello\ncommand: echo hello\n",
        )
        .await
        .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name(), "greet");
    }

    #[tokio::test]
    async fn skip_invalid_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.unwrap();
        tokio::fs::write(tools_dir.join("bad.yaml"), b"not: valid: yaml: [[[")
            .await
            .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn skip_non_yaml_files() {
        let dir = tempfile::tempdir().unwrap();
        let tools_dir = dir.path().join("tools");
        tokio::fs::create_dir_all(&tools_dir).await.unwrap();
        tokio::fs::write(tools_dir.join("greet.toml"), b"name = 'greet'")
            .await
            .unwrap();

        let tools = load_script_tools(dir.path()).await;
        assert!(tools.is_empty());
    }
}
