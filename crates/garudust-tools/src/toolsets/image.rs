use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use serde::Deserialize;
use serde_json::{json, Value};

/// Maximum image size passed to the model (bytes).
/// Larger files are rejected with a clear error rather than flooding the context.
const MAX_IMAGE_BYTES: usize = 5 * 1_024 * 1_024; // 5 MB

fn detect_mime(path: &str, header: &[u8]) -> &'static str {
    if header.starts_with(b"\x89PNG\r\n\x1a\n") {
        return "image/png";
    }
    if header.starts_with(b"\xff\xd8\xff") {
        return "image/jpeg";
    }
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return "image/gif";
    }
    if header.starts_with(b"RIFF") && header.get(8..12) == Some(b"WEBP") {
        return "image/webp";
    }
    match path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

#[derive(Deserialize)]
struct ImageReadInput {
    path: String,
}

pub struct ImageRead;

#[async_trait]
impl Tool for ImageRead {
    fn name(&self) -> &'static str {
        "image_read"
    }

    fn description(&self) -> &'static str {
        "Read an image file and return its base64-encoded content and MIME type so a multimodal model can analyse it."
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
                    "description": "Absolute or relative path to the image file (PNG, JPEG, GIF, WEBP)."
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let input: ImageReadInput =
            serde_json::from_value(params).map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

        let path = std::path::Path::new(&input.path);
        let canonical = std::fs::canonicalize(path)
            .map_err(|_| ToolError::Execution(format!("image not found: {}", input.path)))?;

        // Respect the same allowed-roots rules as read_file.
        if !ctx
            .config
            .security
            .allowed_read_paths
            .iter()
            .any(|root| std::fs::canonicalize(root).is_ok_and(|r| canonical.starts_with(&r)))
        {
            return Err(ToolError::Execution(
                "image path is outside allowed read paths".into(),
            ));
        }

        let file_len = tokio::fs::metadata(&canonical)
            .await
            .map_err(|e| ToolError::Execution(format!("metadata error: {e}")))?
            .len();
        let file_size = usize::try_from(file_len)
            .map_err(|_| ToolError::Execution(format!("file too large: {file_len} bytes")))?;
        if file_size > MAX_IMAGE_BYTES {
            return Err(ToolError::Execution(format!(
                "image too large: {file_size} bytes (max {MAX_IMAGE_BYTES} bytes)"
            )));
        }

        let bytes = tokio::fs::read(&canonical)
            .await
            .map_err(|e| ToolError::Execution(format!("read error: {e}")))?;

        let mime = detect_mime(&input.path, &bytes[..bytes.len().min(12)]);
        let b64 = B64.encode(&bytes);
        let size_kb = bytes.len() / 1_024;

        let output =
            format!("mime_type: {mime}\nsize: {size_kb} KB\ndata: data:{mime};base64,{b64}");

        Ok(ToolResult::ok("", output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_png_by_magic() {
        let header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\r";
        assert_eq!(detect_mime("file.png", header), "image/png");
    }

    #[test]
    fn detect_jpeg_by_magic() {
        let header = b"\xff\xd8\xff\xe0\x00\x10JFIF";
        assert_eq!(detect_mime("file.jpg", header), "image/jpeg");
    }

    #[test]
    fn detect_gif_by_magic() {
        let header = b"GIF89a\x01\x00\x01\x00\x00\xff\x00";
        assert_eq!(detect_mime("file.gif", header), "image/gif");
    }

    #[test]
    fn fallback_to_extension_for_unknown_header() {
        let header = b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        assert_eq!(detect_mime("photo.png", header), "image/png");
        assert_eq!(detect_mime("photo.webp", header), "image/webp");
    }
}
