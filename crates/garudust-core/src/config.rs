use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(skip)]
    pub home_dir: PathBuf,
    pub model: String,
    pub max_iterations: u32,
    pub tool_delay_ms: u64,
    pub provider: String,
    pub base_url: Option<String>,
    #[serde(skip)]
    pub api_key: Option<String>,
    pub compression: CompressionConfig,
    pub network: NetworkConfig,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            home_dir: Self::garudust_dir(),
            model: "anthropic/claude-sonnet-4-6".into(),
            max_iterations: 90,
            tool_delay_ms: 0,
            provider: "openrouter".into(),
            base_url: None,
            api_key: None,
            compression: CompressionConfig::default(),
            network: NetworkConfig::default(),
            mcp_servers: Vec::new(),
        }
    }
}

impl AgentConfig {
    /// Canonical ~/.garudust directory.
    pub fn garudust_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".garudust")
    }

    /// Load config from ~/.garudust/config.yaml + ~/.garudust/.env + environment.
    ///
    /// Priority (highest first):
    ///   1. Environment variables already set in the shell
    ///   2. ~/.garudust/.env  (set if not already present in env)
    ///   3. ~/.garudust/config.yaml
    ///   4. Built-in defaults
    pub fn load() -> Self {
        let home_dir = Self::garudust_dir();

        // Apply ~/.garudust/.env (does not override already-set vars)
        let env_file = home_dir.join(".env");
        apply_dotenv(&env_file);

        // Load config.yaml (non-secret settings)
        let yaml_path = home_dir.join("config.yaml");
        let mut config: AgentConfig = if yaml_path.exists() {
            let src = std::fs::read_to_string(&yaml_path).unwrap_or_default();
            serde_yaml::from_str(&src).unwrap_or_default()
        } else {
            AgentConfig::default()
        };

        config.home_dir = home_dir;

        // Apply env var overrides for secrets / per-session settings
        if let Ok(k) = std::env::var("ANTHROPIC_API_KEY") {
            if !k.is_empty() {
                config.api_key = Some(k);
                config.provider = "anthropic".into();
            }
        } else if let Ok(k) = std::env::var("OPENROUTER_API_KEY") {
            if !k.is_empty() {
                config.api_key = Some(k);
            }
        }
        if let Ok(m) = std::env::var("GARUDUST_MODEL") {
            if !m.is_empty() {
                config.model = m;
            }
        }
        if let Ok(u) = std::env::var("GARUDUST_BASE_URL") {
            if !u.is_empty() {
                config.base_url = Some(u);
            }
        }

        config
    }

    /// Save non-secret settings to ~/.garudust/config.yaml.
    pub fn save_yaml(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.home_dir)?;
        let yaml = serde_yaml::to_string(self).map_err(std::io::Error::other)?;
        std::fs::write(self.home_dir.join("config.yaml"), yaml)
    }

    /// Write or update a KEY=VALUE line in ~/.garudust/.env.
    pub fn set_env_var(home_dir: &Path, key: &str, value: &str) -> std::io::Result<()> {
        std::fs::create_dir_all(home_dir)?;
        let env_path = home_dir.join(".env");
        let existing = if env_path.exists() {
            std::fs::read_to_string(&env_path)?
        } else {
            String::new()
        };

        let prefix = format!("{key}=");
        let mut lines: Vec<String> = existing
            .lines()
            .filter(|l| !l.starts_with(&prefix))
            .map(String::from)
            .collect();
        lines.push(format!("{key}={value}"));

        std::fs::write(&env_path, lines.join("\n") + "\n")
    }
}

/// Parse KEY=VALUE pairs from a .env file and set them as env vars
/// only if the key is not already present in the environment.
fn apply_dotenv(path: &Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            let v = v.trim().trim_matches('"').trim_matches('\'');
            if std::env::var(k).is_err() {
                // SAFETY: single-threaded at startup
                unsafe {
                    std::env::set_var(k, v);
                }
            }
        }
    }
}

// ── Sub-configs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub threshold_fraction: f32,
    pub model: Option<String>,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            threshold_fraction: 0.8,
            model: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkConfig {
    pub force_ipv4: bool,
    pub proxy: Option<String>,
}
