//! Serve the embedded web dashboard SPA (feature `web-ui`).
//!
//! The contents of `web/dist` are baked into the binary via `rust-embed`, so a
//! single `garudust-server` ships the UI with no separate static directory —
//! keeping the "one self-contained binary" property. Unknown paths fall back to
//! `index.html` so client-side routing (e.g. `/config`, `/env`) works on reload.

use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

// Path is relative to this crate's Cargo.toml (crates/garudust-gateway), so
// `../../web/dist` resolves to the workspace-root `web/dist`. rust-embed joins
// it onto CARGO_MANIFEST_DIR at compile time.
#[derive(RustEmbed)]
#[folder = "../../web/dist"]
struct Assets;

/// Guess a content type from a file extension. Covers the asset kinds a Vite
/// build emits; anything else is served as `application/octet-stream`.
fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("map") => "application/json",
        _ => "application/octet-stream",
    }
}

fn serve(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    Some(
        (
            [(header::CONTENT_TYPE, content_type(path))],
            file.data.into_owned(),
        )
            .into_response(),
    )
}

/// Axum fallback handler: serve the requested asset, or `index.html` for any
/// unmatched route (SPA client-side routing), or 404 if the bundle is empty.
pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(resp) = serve(path) {
        return resp;
    }
    if let Some(resp) = serve("index.html") {
        return resp;
    }
    (StatusCode::NOT_FOUND, "web dashboard not built").into_response()
}
