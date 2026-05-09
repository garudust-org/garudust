use std::{
    collections::HashMap,
    fmt::Write as _,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;

// ─── SKILL.md parser ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: Option<String>,
    pub platforms: Option<Vec<String>>,
    /// Optional per-skill tool permissions. Map of tool name → allowed.
    /// Absent tools are not restricted. Union semantics when multiple skills loaded.
    pub permissions: Option<HashMap<String, bool>>,
    pub body: String,
    pub path: PathBuf,
}

impl Skill {
    pub fn matches_platform(&self, platform: &str) -> bool {
        match &self.platforms {
            None => true,
            Some(list) => list.iter().any(|p| p == platform || p == "all"),
        }
    }
}

pub fn parse_skill_md(content: &str, path: PathBuf) -> Option<Skill> {
    let content = content.trim();
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];
    let body = rest[end + 4..].trim().to_string();

    let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter).ok()?;

    let name = yaml["name"].as_str()?.to_string();
    let description = yaml["description"].as_str().unwrap_or("").to_string();
    let version = yaml["version"].as_str().map(str::to_string);
    let platforms = yaml["platforms"].as_sequence().map(|seq| {
        seq.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect()
    });
    // Garudust-native permissions map: { terminal: true, web_fetch: false }
    let mut permissions: Option<HashMap<String, bool>> =
        yaml["permissions"].as_mapping().map(|map| {
            map.iter()
                .filter_map(|(k, v)| Some((k.as_str()?.to_string(), v.as_bool()?)))
                .collect()
        });

    // agentskills.io compatible: allowed-tools is a space-separated allowlist.
    // Merge into permissions with allow=true so both formats work side-by-side.
    if let Some(allowed) = yaml["allowed-tools"].as_str() {
        let map = permissions.get_or_insert_with(HashMap::new);
        for tool in allowed.split_whitespace() {
            map.entry(tool.to_string()).or_insert(true);
        }
    }

    Some(Skill {
        name,
        description,
        version,
        platforms,
        permissions,
        body,
        path,
    })
}

pub async fn load_skills_from_dir(dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(mut entries) = tokio::fs::read_dir(&current).await else {
            continue;
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "SKILL.md") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Some(skill) = parse_skill_md(&content, path) {
                        skills.push(skill);
                    }
                }
            }
        }
    }

    skills
}

// ─── Skills index for system prompt ──────────────────────────────────────────

pub async fn build_skills_index(skills_dir: &Path, platform: &str) -> String {
    let skills = load_skills_from_dir(skills_dir).await;
    if skills.is_empty() {
        return String::new();
    }

    let entries: Vec<String> = skills
        .iter()
        .filter(|s| s.matches_platform(platform))
        .map(|s| {
            let ver = s
                .version
                .as_deref()
                .map(|v| format!(" v{v}"))
                .unwrap_or_default();
            format!("- **{}**{}: {}", s.name, ver, s.description)
        })
        .collect();

    if entries.is_empty() {
        return String::new();
    }

    format!(
        "# Skills\n\
         Before replying, scan this list. If a skill matches or is even partially \
         relevant to the task, you MUST call `skill_view` first to load its full \
         instructions before proceeding. Err on the side of loading — missing a skill \
         means missing critical steps or established workflows. Only skip if genuinely \
         none are relevant.\n\n{}",
        entries.join("\n")
    )
}

// ─── Name sanitizer ──────────────────────────────────────────────────────────

/// Allow only alphanumeric, hyphens, and underscores to prevent path traversal.
/// Validate a skill name against the agentskills.io spec:
/// lowercase letters, digits, and hyphens only; max 64 chars;
/// must not start/end with `-` or contain `--`.
fn sanitize_skill_name(name: &str) -> Option<&str> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('-')
        || name.ends_with('-')
        || name.contains("--")
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        None
    } else {
        Some(name)
    }
}

// ─── Tools ───────────────────────────────────────────────────────────────────

pub struct SkillsList;

#[async_trait]
impl Tool for SkillsList {
    fn name(&self) -> &'static str {
        "skills_list"
    }
    fn description(&self) -> &'static str {
        "List all available skills with name and description"
    }
    fn toolset(&self) -> &'static str {
        "skills"
    }

    fn schema(&self) -> serde_json::Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(
        &self,
        _params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let skills_dir = ctx.config.home_dir.join("skills");
        let skills = load_skills_from_dir(&skills_dir).await;

        if skills.is_empty() {
            return Ok(ToolResult::ok("", "No skills found."));
        }

        let list: Vec<String> = skills
            .iter()
            .map(|s| {
                let ver = s
                    .version
                    .as_deref()
                    .map(|v| format!(" v{v}"))
                    .unwrap_or_default();
                format!("**{}**{}\n  {}", s.name, ver, s.description)
            })
            .collect();

        Ok(ToolResult::ok("", list.join("\n\n")))
    }
}

pub struct SkillView;

#[async_trait]
impl Tool for SkillView {
    fn name(&self) -> &'static str {
        "skill_view"
    }
    fn description(&self) -> &'static str {
        "Load the full instructions of a skill by name"
    }
    fn toolset(&self) -> &'static str {
        "skills"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Skill name to load" }
            },
            "required": ["name"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name required".into()))?;

        let skills_dir = ctx.config.home_dir.join("skills");
        let skills = load_skills_from_dir(&skills_dir).await;

        let skill = skills
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| ToolError::NotFound(format!("skill '{name}' not found")))?;

        if let Some(perms) = &skill.permissions {
            ctx.skill_permissions.write().await.merge(perms);
        }

        Ok(ToolResult::ok(
            "",
            format!("# {}\n\n{}", skill.name, skill.body),
        ))
    }
}

pub struct WriteSkill;

#[async_trait]
impl Tool for WriteSkill {
    fn name(&self) -> &'static str {
        "write_skill"
    }
    fn description(&self) -> &'static str {
        "Create or update a skill in ~/.garudust/skills/<name>/SKILL.md. \
         Use this to save reusable instruction sets the agent should be able to invoke later."
    }
    fn toolset(&self) -> &'static str {
        "skills"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name":        { "type": "string", "description": "Skill identifier (alphanumeric, hyphens, underscores only)" },
                "description": { "type": "string", "description": "One-line description shown in skills_list" },
                "body":        { "type": "string", "description": "Full Markdown instructions for the skill" },
                "version":     { "type": "string", "description": "Optional semver version string (e.g. '1.0.0')" },
                "permissions": {
                    "type": "object",
                    "description": "Optional per-skill tool allowlist. Map of tool name to true (allow) or false (deny). Tools not listed are unrestricted.",
                    "additionalProperties": { "type": "boolean" }
                }
            },
            "required": ["name", "description", "body"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("name required".into()))?;
        let name = sanitize_skill_name(name).ok_or_else(|| {
            ToolError::InvalidArgs(
                "name must use lowercase letters, digits, and hyphens only (agentskills.io compatible); max 64 chars; cannot start/end with '-' or contain '--'".into(),
            )
        })?;

        let description = params["description"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("description required".into()))?;
        let body = params["body"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("body required".into()))?;
        let version = params["version"].as_str().unwrap_or("1.0.0");

        let permissions_block = match params["permissions"].as_object() {
            Some(map) if !map.is_empty() => {
                let mut entries = String::new();
                for (k, v) in map {
                    let _ = writeln!(entries, "  {k}: {}", v.as_bool().unwrap_or(false));
                }
                format!("permissions:\n{entries}")
            }
            _ => String::new(),
        };

        let skill_dir = ctx.config.home_dir.join("skills").join(name);
        tokio::fs::create_dir_all(&skill_dir)
            .await
            .map_err(|e| ToolError::Execution(format!("failed to create skill dir: {e}")))?;

        let content = format!(
            "---\nname: {name}\ndescription: {description}\nversion: {version}\n{permissions_block}---\n\n{body}\n"
        );

        let skill_path = skill_dir.join("SKILL.md");
        tokio::fs::write(&skill_path, &content)
            .await
            .map_err(|e| ToolError::Execution(format!("failed to write skill: {e}")))?;

        Ok(ToolResult::ok(
            "",
            format!("Skill '{name}' saved to {}", skill_path.display()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use garudust_core::tool::SkillPermissions;

    use super::*;

    fn skill_md(name: &str, extra: &str, body: &str) -> String {
        format!("---\nname: {name}\ndescription: test\nversion: 1.0.0\n{extra}---\n\n{body}\n")
    }

    #[test]
    fn parse_minimal_skill() {
        let md = skill_md("my-skill", "", "Do something.");
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert_eq!(skill.body, "Do something.");
        assert!(skill.permissions.is_none());
    }

    #[test]
    fn parse_skill_with_permissions() {
        let md = skill_md(
            "git-workflow",
            "permissions:\n  terminal: true\n  web_fetch: false\n",
            "Always write conventional commits.",
        );
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        let perms = skill.permissions.unwrap();
        assert!(perms["terminal"]);
        assert!(!perms["web_fetch"]);
    }

    #[test]
    fn parse_invalid_frontmatter_returns_none() {
        let md = "no frontmatter at all";
        assert!(parse_skill_md(md, PathBuf::from("SKILL.md")).is_none());
    }

    #[test]
    fn skill_permissions_merge_union_semantics() {
        let mut sp = SkillPermissions::default();
        sp.merge(&HashMap::from([
            ("terminal".into(), false),
            ("read_file".into(), true),
        ]));
        sp.merge(&HashMap::from([("terminal".into(), true)]));
        // allow wins over deny
        assert_eq!(sp.check("terminal"), Some(true));
        assert_eq!(sp.check("read_file"), Some(true));
        // unlisted tool is unrestricted
        assert_eq!(sp.check("web_fetch"), None);
    }

    #[test]
    fn skill_permissions_deny_when_no_allow() {
        let mut sp = SkillPermissions::default();
        sp.merge(&HashMap::from([("write_file".into(), false)]));
        assert_eq!(sp.check("write_file"), Some(false));
    }

    #[test]
    fn permissions_block_written_to_frontmatter() {
        // Verify the generated SKILL.md can be round-tripped through the parser.
        let perms_yaml = "permissions:\n  terminal: true\n  web_fetch: false\n";
        let md = skill_md("deploy", perms_yaml, "Deploy instructions.");
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        let perms = skill.permissions.unwrap();
        assert!(perms["terminal"]);
        assert!(!perms["web_fetch"]);
    }

    #[test]
    fn platform_filter_matches_all_when_no_platforms() {
        let md = skill_md("any", "", "body");
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        assert!(skill.matches_platform("telegram"));
        assert!(skill.matches_platform("cli"));
    }

    #[test]
    fn platform_filter_restricts_to_listed_platforms() {
        let md = skill_md("telegram-only", "platforms:\n  - telegram\n", "body");
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        assert!(skill.matches_platform("telegram"));
        assert!(!skill.matches_platform("cli"));
    }

    // ── agentskills.io compatibility ─────────────────────────────────────────

    #[test]
    fn parse_allowed_tools_field() {
        let md = skill_md("git-ops", "allowed-tools: terminal read_file\n", "body");
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        let perms = skill.permissions.unwrap();
        assert!(perms["terminal"]);
        assert!(perms["read_file"]);
    }

    #[test]
    fn allowed_tools_does_not_override_explicit_deny() {
        // explicit permissions deny wins — allowed-tools only inserts with or_insert
        let md = skill_md(
            "mixed",
            "permissions:\n  terminal: false\nallowed-tools: terminal read_file\n",
            "body",
        );
        let skill = parse_skill_md(&md, PathBuf::from("SKILL.md")).unwrap();
        let perms = skill.permissions.unwrap();
        // permissions map set terminal=false first; allowed-tools uses or_insert so it stays false
        assert!(!perms["terminal"]);
        assert!(perms["read_file"]);
    }

    #[test]
    fn sanitize_rejects_uppercase() {
        assert!(sanitize_skill_name("MySkill").is_none());
    }

    #[test]
    fn sanitize_rejects_underscore() {
        assert!(sanitize_skill_name("my_skill").is_none());
    }

    #[test]
    fn sanitize_rejects_leading_hyphen() {
        assert!(sanitize_skill_name("-skill").is_none());
    }

    #[test]
    fn sanitize_rejects_trailing_hyphen() {
        assert!(sanitize_skill_name("skill-").is_none());
    }

    #[test]
    fn sanitize_rejects_double_hyphen() {
        assert!(sanitize_skill_name("my--skill").is_none());
    }

    #[test]
    fn sanitize_accepts_valid_name() {
        assert!(sanitize_skill_name("git-workflow").is_some());
        assert!(sanitize_skill_name("pdf2json").is_some());
    }
}
