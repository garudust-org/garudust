use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde_json::json;

pub struct MemoryTool;

pub struct UserProfileTool;

#[async_trait]
impl Tool for MemoryTool {
    fn name(&self) -> &'static str {
        "memory"
    }
    fn description(&self) -> &'static str {
        "Manage persistent memory. Actions: add, read, remove, replace"
    }
    fn toolset(&self) -> &'static str {
        "memory"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action":  { "type": "string", "enum": ["add", "read", "remove", "replace"] },
                "content": { "type": "string", "description": "Entry content (for add/replace)" },
                "match":   { "type": "string", "description": "Substring to match (for remove/replace)" }
            },
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("action required".into()))?;

        let mut mem = ctx
            .memory
            .read_memory()
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        match action {
            "read" => {
                let output = mem.serialize();
                Ok(ToolResult::ok(
                    "",
                    if output.is_empty() {
                        "(empty)".into()
                    } else {
                        output
                    },
                ))
            }
            "add" => {
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;
                mem.entries.push(content.trim().to_string());
                ctx.memory
                    .write_memory(&mem)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("", "Entry added"))
            }
            "remove" => {
                let m = params["match"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("match required".into()))?;
                let before = mem.entries.len();
                mem.entries.retain(|e| !e.contains(m));
                let removed = before - mem.entries.len();
                ctx.memory
                    .write_memory(&mem)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("", format!("Removed {removed} entries")))
            }
            "replace" => {
                let m = params["match"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("match required".into()))?;
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;
                let mut replaced = 0;
                for entry in &mut mem.entries {
                    if entry.contains(m) {
                        *entry = content.trim().to_string();
                        replaced += 1;
                    }
                }
                ctx.memory
                    .write_memory(&mem)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("", format!("Replaced {replaced} entries")))
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action: {other}"))),
        }
    }
}

#[async_trait]
impl Tool for UserProfileTool {
    fn name(&self) -> &'static str {
        "user_profile"
    }
    fn description(&self) -> &'static str {
        "Read or update the persistent user profile (USER.md). \
         Use 'write' to replace the entire profile, 'append' to add new information, \
         'read' to view the current profile."
    }
    fn toolset(&self) -> &'static str {
        "memory"
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action":  { "type": "string", "enum": ["read", "write", "append"] },
                "content": { "type": "string", "description": "Profile content (for write/append)" }
            },
            "required": ["action"]
        })
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let action = params["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("action required".into()))?;

        match action {
            "read" => {
                let profile = ctx
                    .memory
                    .read_user_profile()
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok(
                    "",
                    if profile.is_empty() {
                        "(empty)".into()
                    } else {
                        profile
                    },
                ))
            }
            "write" => {
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;
                ctx.memory
                    .write_user_profile(content.trim())
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("", "User profile updated"))
            }
            "append" => {
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("content required".into()))?;
                let existing = ctx
                    .memory
                    .read_user_profile()
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                let updated = if existing.is_empty() {
                    content.trim().to_string()
                } else {
                    format!("{}\n\n{}", existing.trim_end(), content.trim())
                };
                ctx.memory
                    .write_user_profile(&updated)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                Ok(ToolResult::ok("", "Appended to user profile"))
            }
            other => Err(ToolError::InvalidArgs(format!("unknown action: {other}"))),
        }
    }
}
