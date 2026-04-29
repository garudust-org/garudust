use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

static DOTENV_VARS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Load ~/.garudust/.env once per process into an in-memory map.
/// Never writes to process environment, so secrets are not visible to subprocesses.
fn load_dotenv_once(path: &Path) -> &'static HashMap<String, String> {
    DOTENV_VARS.get_or_init(|| {
        let mut map = HashMap::new();
        let Ok(content) = std::fs::read_to_string(path) else {
            return map;
        };
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let k = k.trim().to_string();
                let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
                map.insert(k, v);
            }
        }
        map
    })
}

/// Read an env var: real environment takes priority, dotenv map is fallback.
fn env_or_dotenv(key: &str, dotenv: &HashMap<String, String>) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| dotenv.get(key).filter(|v| !v.is_empty()).cloned())
}

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
    #[serde(default)]
    pub max_concurrent_requests: Option<usize>,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub memory_expiry: MemoryExpiryConfig,
    /// Inject a memory-save reminder every N tool-use iterations within a task.
    /// 0 = disabled. Default: 5.
    #[serde(default = "default_nudge_interval")]
    pub nudge_interval: u32,
}

fn default_nudge_interval() -> u32 {
    5
}

/// Per-category retention policy for memory entries.
/// `None` means the category never expires.
/// `preference` and `skill` default to `None` — they represent durable knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryExpiryConfig {
    /// Max age in days for `fact` entries. Default: 90.
    #[serde(default = "default_fact_days")]
    pub fact_days: Option<u32>,
    /// Max age in days for `project` entries. Default: 30.
    #[serde(default = "default_project_days")]
    pub project_days: Option<u32>,
    /// Max age in days for `other` entries. Default: 60.
    #[serde(default = "default_other_days")]
    pub other_days: Option<u32>,
    /// `preference` entries never expire by default.
    #[serde(default)]
    pub preference_days: Option<u32>,
    /// `skill` entries never expire by default.
    #[serde(default)]
    pub skill_days: Option<u32>,
}

#[allow(clippy::unnecessary_wraps)]
fn default_fact_days() -> Option<u32> {
    Some(90)
}
#[allow(clippy::unnecessary_wraps)]
fn default_project_days() -> Option<u32> {
    Some(30)
}
#[allow(clippy::unnecessary_wraps)]
fn default_other_days() -> Option<u32> {
    Some(60)
}

impl Default for MemoryExpiryConfig {
    fn default() -> Self {
        Self {
            fact_days: default_fact_days(),
            project_days: default_project_days(),
            other_days: default_other_days(),
            preference_days: None,
            skill_days: None,
        }
    }
}

/// Security-related settings grouped together (mirrors CompressionConfig / NetworkConfig pattern).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Bearer token required on /chat* endpoints. None = open (warn at startup).
    #[serde(skip)]
    pub gateway_api_key: Option<String>,

    /// Allowed root paths for read_file tool. Defaults to cwd + home.
    #[serde(default)]
    pub allowed_read_paths: Vec<PathBuf>,

    /// Allowed root paths for write_file tool. Defaults to cwd only.
    #[serde(default)]
    pub allowed_write_paths: Vec<PathBuf>,

    /// Command approval mode: "auto" | "smart" | "deny". Default "smart".
    #[serde(default = "default_approval_mode")]
    pub approval_mode: String,

    /// Per-IP rate limit in requests/minute. None = disabled.
    #[serde(default)]
    pub rate_limit_rpm: Option<u32>,
}

fn default_approval_mode() -> String {
    "smart".to_string()
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
        let cwd = std::env::current_dir().unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_default();
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
            max_concurrent_requests: None,
            security: SecurityConfig {
                gateway_api_key: None,
                allowed_read_paths: vec![cwd.clone(), home],
                allowed_write_paths: vec![cwd],
                approval_mode: default_approval_mode(),
                rate_limit_rpm: None,
            },
            memory_expiry: MemoryExpiryConfig::default(),
            nudge_interval: default_nudge_interval(),
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

        // Load dotenv values into memory (never calls set_var — secrets stay out of process env)
        let env_file = home_dir.join(".env");
        let dotenv = load_dotenv_once(&env_file);

        // Load config.yaml (non-secret settings)
        let yaml_path = home_dir.join("config.yaml");
        let mut config: AgentConfig = if yaml_path.exists() {
            let src = std::fs::read_to_string(&yaml_path).unwrap_or_default();
            serde_yaml::from_str(&src).unwrap_or_default()
        } else {
            AgentConfig::default()
        };

        config.home_dir = home_dir;

        // Populate default security paths if they came back empty from YAML
        if config.security.allowed_read_paths.is_empty() {
            let cwd = std::env::current_dir().unwrap_or_default();
            let home = dirs::home_dir().unwrap_or_default();
            config.security.allowed_read_paths = vec![cwd.clone(), home];
            config.security.allowed_write_paths = vec![cwd];
        }

        // Apply env/dotenv overrides (real env takes priority over dotenv)
        if let Some(k) = env_or_dotenv("ANTHROPIC_API_KEY", dotenv) {
            config.api_key = Some(k);
            config.provider = "anthropic".into();
        } else if let Some(k) = env_or_dotenv("OPENROUTER_API_KEY", dotenv) {
            config.api_key = Some(k);
        } else if let Some(url) = env_or_dotenv("OLLAMA_BASE_URL", dotenv) {
            config.provider = "ollama".into();
            config.base_url = Some(url);
        } else if let Some(url) = env_or_dotenv("VLLM_BASE_URL", dotenv) {
            config.provider = "vllm".into();
            config.base_url = Some(url);
            if let Some(k) = env_or_dotenv("VLLM_API_KEY", dotenv) {
                config.api_key = Some(k);
            }
        }
        if let Some(m) = env_or_dotenv("GARUDUST_MODEL", dotenv) {
            config.model = m;
        }
        if let Some(u) = env_or_dotenv("GARUDUST_BASE_URL", dotenv) {
            config.base_url = Some(u);
        }
        if let Some(k) = env_or_dotenv("GARUDUST_API_KEY", dotenv) {
            config.security.gateway_api_key = Some(k);
        }
        if let Some(v) = env_or_dotenv("GARUDUST_RATE_LIMIT", dotenv) {
            if let Ok(n) = v.parse::<u32>() {
                config.security.rate_limit_rpm = Some(n);
            }
        }
        if let Some(mode) = env_or_dotenv("GARUDUST_APPROVAL_MODE", dotenv) {
            config.security.approval_mode = mode;
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
