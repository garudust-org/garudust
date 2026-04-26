use std::path::{Path, PathBuf};

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

    Some(Skill {
        name,
        description,
        version,
        platforms,
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
        "# Skills\nUse `skill_view` to load a skill's full instructions.\n\n{}",
        entries.join("\n")
    )
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

        Ok(ToolResult::ok(
            "",
            format!("# {}\n\n{}", skill.name, skill.body),
        ))
    }
}
