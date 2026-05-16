use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use garudust_memory::DocStore;
use serde_json::{json, Value};

use super::files::is_path_allowed;
use super::floor_char_boundary;

// ── Chunker ───────────────────────────────────────────────────────────────────

const CHUNK_MAX: usize = 800;
const CHUNK_MIN: usize = 120;

fn chunk_text(text: &str) -> Vec<String> {
    let mut raw: Vec<String> = Vec::new();

    for para in text.split("\n\n") {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        if para.len() <= CHUNK_MAX {
            raw.push(para.to_string());
        } else {
            // Long paragraph: split by single newline
            for line in para.split('\n') {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if line.len() <= CHUNK_MAX {
                    raw.push(line.to_string());
                } else {
                    // Hard split at char boundary
                    let mut start = 0;
                    while start < line.len() {
                        let end = floor_char_boundary(line, start + CHUNK_MAX);
                        let end = if end <= start { start + 1 } else { end };
                        raw.push(line[start..end].to_string());
                        start = end;
                    }
                }
            }
        }
    }

    // Merge orphan chunks that are too short
    let mut merged: Vec<String> = Vec::new();
    for chunk in raw {
        if let Some(last) = merged.last_mut() {
            if last.len() < CHUNK_MIN {
                last.push('\n');
                last.push_str(&chunk);
                continue;
            }
        }
        merged.push(chunk);
    }

    merged
}

// ── Text extraction ───────────────────────────────────────────────────────────

async fn extract_text(path: &str) -> Result<String, ToolError> {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "pdf" {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ToolError::Execution(format!("cannot read '{path}': {e}")))?;
        return tokio::task::spawn_blocking(move || {
            pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| ToolError::Execution(format!("PDF extraction failed: {e}")))
        })
        .await
        .map_err(|e| ToolError::Execution(e.to_string()))?;
    }

    // TXT, CSV, MD, JSON, and any other text format
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ToolError::Execution(format!("cannot read '{path}': {e}")))
}

// ── doc_ingest ────────────────────────────────────────────────────────────────

pub struct DocIngest {
    store: Arc<DocStore>,
}

impl DocIngest {
    pub fn new(store: Arc<DocStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DocIngest {
    fn name(&self) -> &'static str {
        "doc_ingest"
    }
    fn description(&self) -> &'static str {
        "Extract text from a file (PDF, TXT, CSV, MD, …) and index it for full-text search. \
         Re-ingesting the same path replaces the previous index."
    }
    fn toolset(&self) -> &'static str {
        "rag"
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the document file"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let path = params["path"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'path' required".into()))?;

        // Files downloaded by the platform adapters live under /tmp/garudust_
        // and are always safe to ingest regardless of allowed_read_paths.
        let is_platform_tmp = path.starts_with("/tmp/garudust_");
        if !is_platform_tmp
            && !is_path_allowed(Path::new(path), &ctx.config.security.allowed_read_paths)
        {
            return Err(ToolError::Execution(format!(
                "path '{path}' is outside allowed read directories"
            )));
        }

        let text = extract_text(path).await?;
        let chunks = chunk_text(&text);

        if chunks.is_empty() {
            return Ok(ToolResult::ok("", "Document is empty — nothing ingested."));
        }

        let chunk_count = chunks.len();
        let preview = chunks[0].chars().take(200).collect::<String>();
        let store = self.store.clone();
        let path_owned = path.to_string();
        let session_key = ctx.conv_key.clone();

        tokio::task::spawn_blocking(move || store.ingest(&session_key, &path_owned, &chunks))
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let file_name = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        Ok(ToolResult::ok(
            "",
            json!({
                "file": file_name,
                "chunks_indexed": chunk_count,
                "preview": preview
            })
            .to_string(),
        ))
    }
}

// ── doc_search ────────────────────────────────────────────────────────────────

pub struct DocSearch {
    store: Arc<DocStore>,
}

impl DocSearch {
    pub fn new(store: Arc<DocStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DocSearch {
    fn name(&self) -> &'static str {
        "doc_search"
    }
    fn description(&self) -> &'static str {
        "Full-text search across all ingested documents. Returns the most relevant chunks; \
         use the results to answer the user's question. Supports FTS5 syntax (AND, OR, NOT, \"phrase\")."
    }
    fn toolset(&self) -> &'static str {
        "rag"
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query — plain text or FTS5 syntax"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max chunks to return (default 5, max 20)",
                    "default": 5
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let query = params["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArgs("'query' required".into()))?
            .to_string();
        let limit = params["limit"].as_u64().unwrap_or(5).min(20) as usize;
        let session_key = ctx.conv_key.clone();

        let store = self.store.clone();
        let results = tokio::task::spawn_blocking(move || store.search(&session_key, &query, limit))
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        if results.is_empty() {
            return Ok(ToolResult::ok("", "No matching chunks found."));
        }

        let output = results
            .iter()
            .enumerate()
            .map(|(i, r)| {
                format!(
                    "[{}] {} (chunk {})\n{}",
                    i + 1,
                    r.file_name,
                    r.chunk_idx,
                    r.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(ToolResult::ok("", output))
    }
}

// ── doc_list ──────────────────────────────────────────────────────────────────

pub struct DocList {
    store: Arc<DocStore>,
}

impl DocList {
    pub fn new(store: Arc<DocStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DocList {
    fn name(&self) -> &'static str {
        "doc_list"
    }
    fn description(&self) -> &'static str {
        "List all documents that have been ingested and are available for search."
    }
    fn toolset(&self) -> &'static str {
        "rag"
    }
    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let session_key = ctx.conv_key.clone();
        let store = self.store.clone();
        let docs = tokio::task::spawn_blocking(move || store.list(&session_key))
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        if docs.is_empty() {
            return Ok(ToolResult::ok("", "No documents ingested yet."));
        }

        let output = docs
            .iter()
            .map(|d| {
                #[allow(clippy::cast_possible_truncation)]
                let ts = chrono::DateTime::from_timestamp(d.ingested_at as i64, 0).map_or_else(
                    || d.ingested_at.to_string(),
                    |dt| dt.format("%Y-%m-%d %H:%M").to_string(),
                );
                format!(
                    "- {} | path: {} | {} chunks | ingested {}",
                    d.file_name, d.path, d.chunk_count, ts
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ToolResult::ok("", output))
    }
}

// ── doc_forget ────────────────────────────────────────────────────────────────

pub struct DocForget {
    store: Arc<DocStore>,
}

impl DocForget {
    pub fn new(store: Arc<DocStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl Tool for DocForget {
    fn name(&self) -> &'static str {
        "doc_forget"
    }
    fn description(&self) -> &'static str {
        "Remove one or all documents from the RAG search index for the current session. \
         Provide 'file_name' to remove a specific file by name, 'path' to remove by exact path, \
         or set 'all' to true to clear every document in this session."
    }
    fn toolset(&self) -> &'static str {
        "rag"
    }
    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_name": {
                    "type": "string",
                    "description": "Name of the file to remove (e.g. 'price_list.pdf')"
                },
                "path": {
                    "type": "string",
                    "description": "Exact stored path of the document to remove"
                },
                "all": {
                    "type": "boolean",
                    "description": "Set to true to remove ALL documents in this session"
                }
            }
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let session_key = ctx.conv_key.clone();
        let store = self.store.clone();

        // Clear all documents for this session
        if params["all"].as_bool().unwrap_or(false) {
            let count =
                tokio::task::spawn_blocking(move || store.forget_all(&session_key))
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
            return Ok(ToolResult::ok(
                "",
                format!("Removed {count} document(s) from index."),
            ));
        }

        // Remove by file_name (user-friendly)
        if let Some(file_name) = params["file_name"].as_str() {
            let file_name = file_name.to_string();
            let removed =
                tokio::task::spawn_blocking(move || store.forget_by_name(&session_key, &file_name))
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
            return if removed {
                Ok(ToolResult::ok("", "Document removed from index."))
            } else {
                Ok(ToolResult::ok("", "Document not found in index."))
            };
        }

        // Remove by exact path
        if let Some(path) = params["path"].as_str() {
            let path = path.to_string();
            let removed =
                tokio::task::spawn_blocking(move || store.forget(&session_key, &path))
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
            return if removed {
                Ok(ToolResult::ok("", "Document removed from index."))
            } else {
                Ok(ToolResult::ok("", "Document not found in index."))
            };
        }

        Err(ToolError::InvalidArgs(
            "provide 'file_name', 'path', or 'all: true'".into(),
        ))
    }
}
