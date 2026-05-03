use std::path::Path;

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};

use super::files::is_path_allowed;

/// Maximum bytes of extracted text returned to the agent.
const MAX_OUTPUT_BYTES: usize = 200 * 1_024;

/// Maximum pages processed in a single call.
const MAX_PAGES: usize = 50;

#[derive(Deserialize)]
struct PdfReadInput {
    path: String,
    pages: Option<String>,
}

/// Parse "N" or "N-M" into an inclusive 1-based page range.
fn parse_page_range(s: &str) -> Result<(usize, usize), ToolError> {
    let s = s.trim();
    if let Some((a, b)) = s.split_once('-') {
        let start = a
            .trim()
            .parse::<usize>()
            .map_err(|_| ToolError::InvalidArgs(format!("invalid page range: {s}")))?;
        let end = b
            .trim()
            .parse::<usize>()
            .map_err(|_| ToolError::InvalidArgs(format!("invalid page range: {s}")))?;
        if start == 0 || end < start {
            return Err(ToolError::InvalidArgs(format!("invalid page range: {s}")));
        }
        Ok((start, end))
    } else {
        let page = s
            .parse::<usize>()
            .map_err(|_| ToolError::InvalidArgs(format!("invalid page: {s}")))?;
        if page == 0 {
            return Err(ToolError::InvalidArgs("page number must be >= 1".into()));
        }
        Ok((page, page))
    }
}

fn pdf_obj_to_string(obj: &lopdf::Object) -> Option<String> {
    match obj {
        lopdf::Object::String(bytes, _) => String::from_utf8(bytes.clone())
            .ok()
            .filter(|s| !s.trim().is_empty()),
        _ => None,
    }
}

fn extract_metadata(doc: &lopdf::Document) -> (Option<String>, Option<String>) {
    let info_id = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok());

    let dict = info_id
        .and_then(|id| doc.get_object(id).ok())
        .and_then(|o| o.as_dict().ok());

    let Some(dict) = dict else {
        return (None, None);
    };

    let title = dict.get(b"Title").ok().and_then(pdf_obj_to_string);
    let author = dict.get(b"Author").ok().and_then(pdf_obj_to_string);
    (title, author)
}

pub struct PdfRead;

#[async_trait]
impl Tool for PdfRead {
    fn name(&self) -> &'static str {
        "pdf_read"
    }

    fn description(&self) -> &'static str {
        "Extract plain text from a PDF file. Returns the text content, total page count, and document metadata (title, author when available)."
    }

    fn toolset(&self) -> &'static str {
        "files"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute path to the PDF file"
                },
                "pages": {
                    "type": "string",
                    "description": "Page range to extract, e.g. \"1-5\" or \"3\" (default: all pages, capped at 50)"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: PdfReadInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        if !is_path_allowed(
            Path::new(&input.path),
            &ctx.config.security.allowed_read_paths,
        ) {
            return Err(ToolError::Execution(format!(
                "path '{}' is outside allowed read directories",
                input.path
            )));
        }

        let page_range = input.pages.as_deref().map(parse_page_range).transpose()?;
        let path = input.path.clone();

        let bytes = tokio::fs::read(&path)
            .await
            .map_err(|e| ToolError::Execution(format!("cannot read '{path}': {e}")))?;

        let result: Result<Value, String> = tokio::task::spawn_blocking(move || {
            let doc =
                lopdf::Document::load_mem(&bytes).map_err(|e| format!("PDF parse error: {e}"))?;

            let page_count = doc.get_pages().len();
            let (title, author) = extract_metadata(&doc);

            let all_text = pdf_extract::extract_text_from_mem(&bytes)
                .map_err(|e| format!("text extraction failed: {e}"))?;

            // pdf-extract separates pages with form-feed (\x0c)
            let all_pages: Vec<&str> = all_text.split('\x0c').collect();

            let (start, end) = page_range.unwrap_or((1, MAX_PAGES.min(page_count)));
            let end = end.min(page_count).min(start + MAX_PAGES - 1);

            if start > page_count {
                return Err(format!(
                    "page {start} is out of range — document has {page_count} pages"
                ));
            }

            let selected = all_pages
                .get(start.saturating_sub(1)..end.min(all_pages.len()))
                .unwrap_or(&[]);
            let mut text = selected.join("\n\n");

            let mut truncated = false;
            if text.len() > MAX_OUTPUT_BYTES {
                text.truncate(MAX_OUTPUT_BYTES);
                truncated = true;
            }

            let mut out = json!({
                "page_count": page_count,
                "pages_extracted": format!("{start}-{}", end.min(page_count)),
                "text": text,
            });
            if let Some(t) = title {
                out["title"] = json!(t);
            }
            if let Some(a) = author {
                out["author"] = json!(a);
            }
            if truncated {
                out["truncated"] = json!(true);
            }

            Ok(out)
        })
        .await
        .map_err(|e| ToolError::Execution(e.to_string()))?;

        let output = result.map_err(ToolError::Execution)?;
        Ok(ToolResult::ok("", output.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_page() {
        assert_eq!(parse_page_range("3").unwrap(), (3, 3));
    }

    #[test]
    fn parse_range() {
        assert_eq!(parse_page_range("1-5").unwrap(), (1, 5));
        assert_eq!(parse_page_range(" 2 - 10 ").unwrap(), (2, 10));
    }

    #[test]
    fn rejects_zero_page() {
        assert!(parse_page_range("0").is_err());
    }

    #[test]
    fn rejects_inverted_range() {
        assert!(parse_page_range("5-3").is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_page_range("abc").is_err());
        assert!(parse_page_range("1-abc").is_err());
    }
}
