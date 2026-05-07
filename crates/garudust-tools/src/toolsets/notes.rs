use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};

// Notes live at ~/.garudust/notes/<key>.md — separate from persistent memory
// so the agent can keep short-lived session reminders without polluting memory.

fn notes_dir(ctx: &ToolContext) -> std::path::PathBuf {
    ctx.config.home_dir.join("notes")
}

fn sanitize_key(key: &str) -> Result<String, ToolError> {
    let clean: String = key
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if clean.is_empty() {
        return Err(ToolError::InvalidArgs(
            "note key must contain alphanumeric characters".into(),
        ));
    }
    Ok(clean)
}

// ── write_note ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct WriteNoteInput {
    key: String,
    content: String,
}

pub struct WriteNote;

#[async_trait]
impl Tool for WriteNote {
    fn name(&self) -> &'static str {
        "write_note"
    }

    fn description(&self) -> &'static str {
        "Save a short note or todo item under a key. Overwrites any existing note with the same key. Notes persist across sessions."
    }

    fn toolset(&self) -> &'static str {
        "notes"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "Short identifier for the note (alphanumeric, hyphens, underscores)."
                },
                "content": {
                    "type": "string",
                    "description": "Note content (markdown supported)."
                }
            },
            "required": ["key", "content"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: WriteNoteInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let key = sanitize_key(&input.key)?;
        let dir = notes_dir(ctx);
        tokio::fs::create_dir_all(&dir)
            .await
            .map_err(|e| ToolError::Execution(format!("create notes dir: {e}")))?;

        let path = dir.join(format!("{key}.md"));
        tokio::fs::write(&path, &input.content)
            .await
            .map_err(|e| ToolError::Execution(format!("write note: {e}")))?;

        Ok(ToolResult::ok("", format!("Note '{key}' saved.")))
    }
}

// ── read_note ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ReadNoteInput {
    key: String,
}

pub struct ReadNote;

#[async_trait]
impl Tool for ReadNote {
    fn name(&self) -> &'static str {
        "read_note"
    }

    fn description(&self) -> &'static str {
        "Read a saved note by key."
    }

    fn toolset(&self) -> &'static str {
        "notes"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "The note key to read."
                }
            },
            "required": ["key"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: ReadNoteInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let key = sanitize_key(&input.key)?;
        let path = notes_dir(ctx).join(format!("{key}.md"));

        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|_| ToolError::Execution(format!("note '{key}' not found")))?;

        Ok(ToolResult::ok("", content))
    }
}

// ── list_notes ────────────────────────────────────────────────────────────────

pub struct ListNotes;

#[async_trait]
impl Tool for ListNotes {
    fn name(&self) -> &'static str {
        "list_notes"
    }

    fn description(&self) -> &'static str {
        "List all saved note keys."
    }

    fn toolset(&self) -> &'static str {
        "notes"
    }

    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let dir = notes_dir(ctx);
        let Ok(mut entries) = tokio::fs::read_dir(&dir).await else {
            return Ok(ToolResult::ok("", "No notes found."));
        };

        let mut keys = Vec::new();
        loop {
            match entries.next_entry().await {
                Ok(Some(entry)) => {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.ends_with(".md") {
                        keys.push(name.trim_end_matches(".md").to_string());
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    return Err(ToolError::Execution(format!("list notes: {e}")));
                }
            }
        }

        if keys.is_empty() {
            return Ok(ToolResult::ok("", "No notes found."));
        }

        keys.sort();
        Ok(ToolResult::ok("", keys.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_keeps_hyphens_and_underscores() {
        assert_eq!(sanitize_key("my-note_1").unwrap(), "my-note_1");
    }

    #[test]
    fn sanitize_empty_key_errors() {
        assert!(sanitize_key("").is_err());
        assert!(sanitize_key("!@#$").is_err());
    }

    #[test]
    fn sanitize_strips_path_traversal() {
        // ../etc/passwd → only alphanumeric kept → "etcpasswd"
        let key = sanitize_key("../etc/passwd").unwrap();
        assert!(!key.contains('.'));
        assert!(!key.contains('/'));
    }
}
