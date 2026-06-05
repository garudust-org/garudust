//! Manage `~/.garudust/.env` secrets over HTTP for the web dashboard.
//!
//! # Trust boundary
//!
//! Garudust deliberately keeps secrets in an in-memory map and never exposes
//! them to subprocesses or to the wire. This module preserves that boundary:
//!
//! * **`GET /api/env` is masked** — it returns each key with a fixed-width
//!   placeholder plus at most the last 4 characters. It never returns a secret
//!   value, and the mask is a constant width so it does not even leak the
//!   secret's length.
//! * **`PUT /api/env` is write-only** — it accepts a `{key, value}` pair and
//!   appends/updates the `.env` file. There is no read-back of the value.
//!
//! Keys and values are validated before they touch the file: a value containing
//! a newline could otherwise inject an unrelated `OTHER_KEY=...` line into
//! `.env` (the writer joins entries with `\n`), and a malformed key could
//! produce an unparseable file.

use std::path::Path;

use axum::{extract::State, http::StatusCode, Json};
use garudust_core::config::AgentConfig;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

/// One `.env` entry, value always masked.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EnvEntry {
    pub key: String,
    /// Masked stand-in for the value, e.g. `••••••3a9f`. Never the real value.
    pub masked: String,
}

/// Body for `PUT /api/env`.
#[derive(Debug, Deserialize)]
pub struct SetEnvRequest {
    pub key: String,
    pub value: String,
}

/// Mask a secret to a fixed-width placeholder plus the last 4 chars (when long
/// enough). The bullet run is a constant 6 so the rendered width does not reveal
/// the secret's true length.
fn mask_secret(value: &str) -> String {
    const BULLETS: &str = "••••••";
    let n = value.chars().count();
    if n <= 4 {
        BULLETS.to_string()
    } else {
        let last4: String = value.chars().skip(n - 4).collect();
        format!("{BULLETS}{last4}")
    }
}

/// Env var names: an uppercase ASCII letter or underscore, followed by uppercase
/// letters, digits, or underscores. Rejects empty, lowercase, and any name with
/// `=`, whitespace, or other shell/file-breaking characters.
fn valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_uppercase() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// A value is writable only if it contains no line break — a newline/carriage
/// return would inject a second `KEY=...` line into `.env`.
fn valid_env_value(value: &str) -> bool {
    !value.contains('\n') && !value.contains('\r')
}

/// Parse `home/.env` into masked entries. Lines without `=` and comment lines
/// (`#`) are skipped. Missing file yields an empty list.
fn read_env_entries(home: &Path) -> Vec<EnvEntry> {
    let path = home.join(".env");
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some(EnvEntry {
                key: key.to_string(),
                masked: mask_secret(value.trim()),
            })
        })
        .collect()
}

/// `GET /api/env` — list configured secret keys with masked values.
pub async fn get_env(State(state): State<AppState>) -> Json<Vec<EnvEntry>> {
    Json(read_env_entries(&state.config.home_dir))
}

/// `PUT /api/env` — set or update a single secret. Write-only.
pub async fn put_env(
    State(state): State<AppState>,
    Json(req): Json<SetEnvRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !valid_env_key(&req.key) {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid key: must match [A-Z_][A-Z0-9_]*".to_string(),
        ));
    }
    if !valid_env_value(&req.value) {
        return Err((
            StatusCode::BAD_REQUEST,
            "invalid value: must not contain line breaks".to_string(),
        ));
    }
    AgentConfig::set_env_var(&state.config.home_dir, &req.key, &req.value)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Body for `DELETE /api/env`.
#[derive(Debug, Deserialize)]
pub struct DeleteEnvRequest {
    pub key: String,
}

/// `DELETE /api/env` — remove a secret from `.env`.
pub async fn delete_env(
    State(state): State<AppState>,
    Json(req): Json<DeleteEnvRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    if !valid_env_key(&req.key) {
        return Err((StatusCode::BAD_REQUEST, "invalid key".to_string()));
    }
    AgentConfig::delete_env_var(&state.config.home_dir, &req.key)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_hides_value_and_keeps_constant_width() {
        // Long secret: last 4 shown, fixed bullet run.
        assert_eq!(mask_secret("sk-ant-abcd1234ef"), "••••••34ef");
        // Two different-length secrets render the same bullet width — no length leak.
        let short = mask_secret("aaaaa");
        let long = mask_secret("a".repeat(200).as_str());
        assert_eq!(short.matches('•').count(), long.matches('•').count());
    }

    #[test]
    fn mask_never_reveals_full_short_secret() {
        assert_eq!(mask_secret("abcd"), "••••••");
        assert_eq!(mask_secret(""), "••••••");
    }

    #[test]
    fn valid_keys_accepted_invalid_rejected() {
        assert!(valid_env_key("ANTHROPIC_API_KEY"));
        assert!(valid_env_key("_PRIVATE"));
        assert!(valid_env_key("GARUDUST_PORT"));
        assert!(!valid_env_key(""));
        assert!(!valid_env_key("lowercase"));
        assert!(!valid_env_key("HAS SPACE"));
        assert!(!valid_env_key("HAS=EQUALS"));
        assert!(!valid_env_key("1STARTSNUM"));
    }

    #[test]
    fn value_with_linebreak_rejected() {
        assert!(valid_env_value("sk-normal-value"));
        assert!(!valid_env_value("sk-evil\nINJECTED=1"));
        assert!(!valid_env_value("sk-evil\rINJECTED=1"));
    }

    #[test]
    fn read_entries_masks_and_skips_comments() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".env"),
            "# comment\nANTHROPIC_API_KEY=sk-ant-secret123\n\nEMPTYLINE_OK=1\n",
        )
        .unwrap();

        let entries = read_env_entries(dir.path());
        assert_eq!(entries.len(), 2);
        let anthropic = entries
            .iter()
            .find(|e| e.key == "ANTHROPIC_API_KEY")
            .unwrap();
        assert!(!anthropic.masked.contains("secret"));
        assert!(anthropic.masked.ends_with("t123"));
    }

    #[test]
    fn read_entries_missing_file_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_env_entries(dir.path()).is_empty());
    }

    #[test]
    fn delete_removes_only_the_named_key() {
        let dir = tempfile::tempdir().unwrap();
        AgentConfig::set_env_var(dir.path(), "FOO", "bar").unwrap();
        AgentConfig::set_env_var(dir.path(), "BAZ", "qux").unwrap();

        assert!(AgentConfig::delete_env_var(dir.path(), "FOO").unwrap());
        let keys: Vec<String> = read_env_entries(dir.path())
            .into_iter()
            .map(|e| e.key)
            .collect();
        assert!(!keys.contains(&"FOO".to_string()));
        assert!(keys.contains(&"BAZ".to_string()));
        // Deleting an absent key is a no-op.
        assert!(!AgentConfig::delete_env_var(dir.path(), "FOO").unwrap());
    }
}
