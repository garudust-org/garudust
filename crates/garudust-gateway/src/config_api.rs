//! Read/write the agent's `config.yaml` over HTTP for the web dashboard.
//!
//! Both handlers operate on the on-disk `config.yaml` (via `AppState.home_dir`)
//! rather than the in-memory [`AppState::config`] snapshot — the latter is taken
//! at startup and is *not* swapped on hot-reload, so reading it would return a
//! stale view after the first config change. Writing goes through
//! [`AgentConfig::save_yaml`] (atomic write+rename); the running server's
//! config-file watcher then picks up the change and hot-reloads the agent.
//!
//! Secrets never travel through here: `api_key`, `fallback_api_keys` and
//! `home_dir` are `#[serde(skip)]` on [`AgentConfig`], so they are absent from
//! both the GET response and any accepted PUT body. Secret management is the
//! job of [`crate::env_api`].

use std::path::Path;

use axum::{extract::State, http::StatusCode, Json};
use garudust_core::config::AgentConfig;

use crate::state::AppState;

/// Load `config.yaml` from `home`, falling back to defaults when it is missing
/// or unparseable. The returned value has `home_dir` left at its default — that
/// field is `#[serde(skip)]` and is irrelevant to callers.
fn read_config(home: &Path) -> AgentConfig {
    let path = home.join("config.yaml");
    if path.exists() {
        let src = std::fs::read_to_string(&path).unwrap_or_default();
        serde_yaml::from_str(&src).unwrap_or_default()
    } else {
        AgentConfig::default()
    }
}

/// Persist `cfg` to `home/config.yaml`. `home_dir` is forced to `home` so a
/// client cannot redirect the write elsewhere by setting that field (it is
/// `#[serde(skip)]` and so always arrives as default, but we set it defensively).
fn write_config(home: &Path, mut cfg: AgentConfig) -> std::io::Result<()> {
    cfg.home_dir = home.to_path_buf();
    cfg.save_yaml()
}

/// `GET /api/config` — current non-secret configuration as JSON.
pub async fn get_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    let cfg = read_config(&state.config.home_dir);
    // Serialization mirrors `save_yaml`, so secret `#[serde(skip)]` fields are
    // omitted — the response can never contain an API key.
    Json(serde_json::to_value(&cfg).unwrap_or(serde_json::Value::Null))
}

/// `PUT /api/config` — replace `config.yaml` with the posted configuration.
/// The config-file watcher hot-reloads the agent after the write lands.
pub async fn put_config(
    State(state): State<AppState>,
    Json(cfg): Json<AgentConfig>,
) -> Result<StatusCode, (StatusCode, String)> {
    if cfg.model.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "`model` must not be empty".to_string(),
        ));
    }
    write_config(&state.config.home_dir, cfg)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_config_missing_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = read_config(dir.path());
        assert_eq!(cfg.model, AgentConfig::default().model);
    }

    #[test]
    fn write_then_read_round_trips_model() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = AgentConfig::default();
        cfg.model = "claude-opus-4-8".to_string();
        write_config(dir.path(), cfg).unwrap();

        let reloaded = read_config(dir.path());
        assert_eq!(reloaded.model, "claude-opus-4-8");
        assert!(dir.path().join("config.yaml").exists());
    }

    #[test]
    fn written_config_never_contains_secret_fields() {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = AgentConfig::default();
        cfg.api_key = Some("sk-super-secret".to_string());
        cfg.fallback_api_keys = vec!["sk-backup".to_string()];
        write_config(dir.path(), cfg).unwrap();

        let yaml = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
        assert!(!yaml.contains("sk-super-secret"));
        assert!(!yaml.contains("sk-backup"));
        assert!(!yaml.contains("api_key"));
    }
}
