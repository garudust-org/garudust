use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::types::ReasoningEffort;

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

/// Read a secret from real env or ~/.garudust/.env (whichever is set first).
/// Useful for Rust tools that don't go through script.rs env forwarding.
pub fn get_secret(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            DOTENV_VARS
                .get()?
                .get(key)
                .filter(|v| !v.is_empty())
                .cloned()
        })
}

/// Per-tool or per-skill configuration overrides.
/// All fields are optional; unset fields leave the tool's own defaults intact.
/// This struct is intentionally open-ended — new fields can be added here in
/// future releases without breaking existing config files (serde default).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolOverrideConfig {
    /// Primary LLM model the tool's subprocess should use.
    /// Forwarded as `GARUDUST_MODEL` env var. Empty string = tool's own default.
    #[serde(default)]
    pub model: String,
    /// Fallback model tried when the primary model fails or is unavailable.
    /// Forwarded as `GARUDUST_FALLBACK_MODEL` env var. Empty string = tool's own default.
    #[serde(default)]
    pub fallback_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(skip)]
    pub home_dir: PathBuf,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Maximum iterations for sub-agents spawned via delegate_task / delegate_tasks.
    /// Defaults to `max_iterations` when unset, letting you cap sub-agents lower than
    /// the parent (e.g. `sub_agent_max_iterations: 10`) to limit runaway delegation chains.
    #[serde(default)]
    pub sub_agent_max_iterations: Option<u32>,
    /// Maximum nesting depth for delegate_task / delegate_tasks.
    /// Depth 0 = parent agent, depth 1 = first sub-agent, etc.
    /// Sub-agents at or beyond this depth cannot call delegate_task again.
    /// Default: 1 (sub-agents may not re-delegate).
    #[serde(default = "default_max_delegation_depth")]
    pub max_delegation_depth: u32,
    #[serde(default)]
    pub tool_delay_ms: u64,
    #[serde(default = "default_provider")]
    pub provider: String,
    pub base_url: Option<String>,
    /// Provider routing table: hint name → "provider/model" string.
    /// Example: `cheap: groq/llama-3.1-8b-instant`
    /// When a hint is passed to agent.run(), the agent looks up the target here,
    /// builds an appropriate transport, and overrides the model for that task only.
    #[serde(default)]
    pub routing: std::collections::HashMap<String, String>,
    /// Per-tool configuration overrides, keyed by tool name.
    /// Example:
    /// ```yaml
    /// tools:
    ///   view_image:
    ///     model: "openrouter/google/gemini-flash-1.5"
    ///     fallback_model: "google/gemini-1.5-flash"
    /// ```
    /// Values are forwarded as `GARUDUST_MODEL` / `GARUDUST_FALLBACK_MODEL` env vars
    /// to the tool's subprocess. Tools that do not read these vars are unaffected.
    #[serde(default)]
    pub tools: std::collections::HashMap<String, ToolOverrideConfig>,
    #[serde(skip)]
    pub api_key: Option<String>,
    /// Fallback API keys tried in order when the primary key returns 401/403.
    /// Set via `LLM_FALLBACK_API_KEYS` env var or .env file (comma-separated values).
    #[serde(skip)]
    pub fallback_api_keys: Vec<String>,
    #[serde(default)]
    pub compression: CompressionConfig,
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
    /// Max retry attempts on transient LLM API errors (429, 5xx, network). 0 = disabled.
    #[serde(default = "default_llm_max_retries")]
    pub llm_max_retries: u32,
    /// Base delay in milliseconds for exponential backoff between retries.
    #[serde(default = "default_llm_retry_base_ms")]
    pub llm_retry_base_ms: u64,
    /// Platform-level access controls (whitelist, mention gate, session isolation).
    #[serde(default)]
    pub platform: PlatformConfig,
    /// Minimum tool-use iterations that trigger an automatic skill-reflection pass after a task.
    /// The agent reviews the conversation and calls write_skill if the workflow is reusable.
    /// Set to 0 to disable. Default: 5.
    #[serde(default = "default_auto_skill_threshold")]
    pub auto_skill_threshold: u32,
    /// Timeout in seconds for a single LLM API call (chat or stream). 0 = no timeout. Default: 120.
    #[serde(default = "default_llm_timeout_secs")]
    pub llm_timeout_secs: u64,
    /// Timeout in seconds applied to every non-terminal tool dispatch. 0 = no timeout. Default: 60.
    #[serde(default = "default_tool_timeout_secs")]
    pub tool_timeout_secs: u64,
    /// Drain window in seconds for graceful shutdown — server waits this long for in-flight
    /// requests to complete before forcing exit. Default: 30.
    #[serde(default = "default_shutdown_timeout_secs")]
    pub shutdown_timeout_secs: u64,
    /// Hard cap on total tokens (input + output) consumed by a single task.
    /// When exceeded the agent stops and returns what it has with a budget notice.
    /// `None` means no limit.
    #[serde(default)]
    pub max_tokens_per_task: Option<u32>,
    /// Maximum output tokens per LLM request. Default: 8192.
    /// Lower this for models with small context windows (e.g. 4096 for a 27k-ctx model).
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    /// Reasoning effort for supported models (Claude extended thinking, OpenAI o1/o3/o4).
    /// Set via config.yaml: `reasoning_effort: medium`
    #[serde(default)]
    pub reasoning_effort: Option<ReasoningEffort>,
    /// Maximum context window of the model in tokens.
    /// Used by the context compressor to decide when to summarise history.
    /// Defaults to 128 000. Set this to the actual limit for small-context models
    /// (e.g. `context_window: 27168` for Qwen3-14B-AWQ on vLLM).
    #[serde(default)]
    pub context_window: Option<usize>,
    /// Toolsets to disable. Removes all tools in the named toolset from every
    /// request, reducing context usage for small-context models.
    /// Available toolsets: web, files, terminal, memory, skills, agent,
    ///   browser, git, notes, json, mcp, rag
    /// Providers: anthropic, openai, gemini, groq, mistral, deepseek, xai,
    ///   openrouter, vllm, ollama, bedrock, codex, thaillm
    /// Example: `disabled_toolsets: [browser, git, notes, json, agent, rag]`
    #[serde(default = "default_disabled_toolsets")]
    pub disabled_toolsets: Vec<String>,
    /// Individual tools to disable by exact name. Useful when only some tools
    /// in a toolset need to be removed (e.g. disable `image_read` without
    /// removing the entire `files` toolset).
    /// Example: `disabled_tools: [image_read, pdf_read, session_search]`
    #[serde(default)]
    pub disabled_tools: Vec<String>,
    /// Append a usage footer (`[N iter | Xin Yout Ztok @ model]`) to every
    /// agent response. Useful for debugging; usually unwanted on chat platforms
    /// where end users see the output. Default: false.
    #[serde(default)]
    pub show_usage_footer: bool,
    /// Maximum number of tokens (rough estimate: chars / 4) injected from
    /// persistent memory into the system prompt. Oldest entries are dropped
    /// first when the cap is exceeded. `None` = no cap. Default: None.
    #[serde(default)]
    pub max_memory_tokens: Option<u32>,
    /// Per-platform webhook server settings (LINE, WhatsApp, generic webhook).
    /// Each entry sets enabled flag, listening port, and HTTP path. Tokens and
    /// secrets continue to be read from `~/.garudust/.env` — never from yaml.
    #[serde(default)]
    pub platforms: WebhookPlatformsConfig,
    /// HTTP gateway server settings (port, …). Overridden by `--port` and
    /// `GARUDUST_PORT` env var.
    #[serde(default)]
    pub server: ServerConfig,
    /// Cron scheduler — recurring agent tasks plus the memory consolidation /
    /// expiry sweeps. CLI flags (`--cron-jobs`, `--memory-cron`,
    /// `--memory-expiry-cron`) and the corresponding env vars take precedence.
    #[serde(default)]
    pub cron: CronConfig,
}

/// Default model used when no `config.yaml`, env override, or routing hint applies.
pub const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-6";
/// Default provider used when none is configured or auto-detected.
pub const DEFAULT_PROVIDER: &str = "openrouter";

fn default_model() -> String {
    DEFAULT_MODEL.into()
}
fn default_provider() -> String {
    DEFAULT_PROVIDER.into()
}
fn default_max_iterations() -> u32 {
    90
}
fn default_max_delegation_depth() -> u32 {
    1
}
fn default_nudge_interval() -> u32 {
    5
}
fn default_auto_skill_threshold() -> u32 {
    5
}
fn default_llm_max_retries() -> u32 {
    3
}
fn default_llm_retry_base_ms() -> u64 {
    1000
}
fn default_llm_timeout_secs() -> u64 {
    120
}
fn default_tool_timeout_secs() -> u64 {
    60
}
fn default_shutdown_timeout_secs() -> u64 {
    30
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

/// Terminal execution sandbox mode.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TerminalSandbox {
    /// Direct host execution (default). Hardline blocks still apply.
    #[default]
    None,
    /// Wrap every command in `docker run --rm` with hardened flags.
    Docker,
}

/// Security-related settings grouped together (mirrors CompressionConfig pattern).
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Terminal execution sandbox. Default "none" (direct host execution).
    #[serde(default)]
    pub terminal_sandbox: TerminalSandbox,

    /// Docker image used when `terminal_sandbox = docker`. Default "ubuntu:24.04".
    #[serde(default = "default_sandbox_image")]
    pub terminal_sandbox_image: String,

    /// Extra `docker run` flags appended after the hardened defaults.
    /// Example: `["--network=none", "--memory=512m", "--cpus=0.5"]`
    #[serde(default)]
    pub terminal_sandbox_opts: Vec<String>,
}

fn default_approval_mode() -> String {
    "smart".to_string()
}

fn default_sandbox_image() -> String {
    "ubuntu:24.04".to_string()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            gateway_api_key: None,
            allowed_read_paths: Vec::new(),
            allowed_write_paths: Vec::new(),
            approval_mode: default_approval_mode(),
            rate_limit_rpm: None,
            terminal_sandbox: TerminalSandbox::None,
            terminal_sandbox_image: default_sandbox_image(),
            terminal_sandbox_opts: Vec::new(),
        }
    }
}

/// Platform-level access and behaviour controls (whitelist, mention gate, session isolation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// User IDs allowed to send messages to the agent.
    /// Empty list means everyone is allowed.
    #[serde(default)]
    pub allowed_user_ids: Vec<String>,

    /// Only respond in group chats when the bot is @mentioned.
    /// Private / DM chats always get a response regardless of this flag.
    #[serde(default)]
    pub require_mention: bool,

    /// Bot username used for @mention detection (without the @).
    /// Example: set to "mybot" so @mybot triggers a response.
    #[serde(default)]
    pub bot_username: String,

    /// Give each user their own conversation session (default: true).
    /// Set to false only when you want all users in a channel to share one session.
    /// Not applied to the webhook platform — webhook callers control session routing via payload.
    #[serde(default = "default_true")]
    pub session_per_user: bool,
}

fn default_true() -> bool {
    true
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            allowed_user_ids: Vec::new(),
            require_mention: false,
            bot_username: String::new(),
            session_per_user: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Per-platform webhook server settings. A `WebhookPlatformConfig` with
/// `enabled = false` means the adapter is not started even if its secret is
/// present in the environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPlatformConfig {
    /// Whether to start this adapter at boot.
    #[serde(default)]
    pub enabled: bool,
    /// TCP port to bind on `0.0.0.0`.
    pub port: u16,
    /// HTTP path the adapter listens on (e.g. `/webhooks/line`).
    pub webhook_path: String,
}

/// Container for all webhook-based platform settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookPlatformsConfig {
    #[serde(default)]
    pub line: Option<WebhookPlatformConfig>,
    #[serde(default)]
    pub whatsapp: Option<WebhookPlatformConfig>,
    #[serde(default)]
    pub webhook: Option<WebhookPlatformConfig>,
}

/// HTTP gateway server settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// TCP port for the HTTP gateway. Default `3000`.
    #[serde(default = "default_server_port")]
    pub port: u16,
}

fn default_server_port() -> u16 {
    3000
}

fn default_disabled_toolsets() -> Vec<String> {
    vec![]
}

/// Parse a comma-separated `"cron_expr=task"` env var into structured [`CronJob`]s.
/// Mirrors `garudust_cron::parse_job_pairs` (kept inline to avoid a core→cron dep cycle).
fn parse_cron_jobs_str(s: &str) -> Vec<CronJob> {
    s.split(',')
        .filter_map(|entry| {
            let (expr, task) = entry.trim().split_once('=')?;
            Some(CronJob {
                schedule: expr.trim().to_string(),
                task: task.trim().to_string(),
            })
        })
        .collect()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: default_server_port(),
        }
    }
}

/// A single scheduled agent task — cron expression plus the prompt to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJob {
    /// Standard 5-field cron expression (e.g. `0 9 * * *`).
    pub schedule: String,
    /// The task prompt handed to the agent when the cron fires.
    pub task: String,
}

/// Cron scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronConfig {
    /// Recurring agent tasks.
    #[serde(default)]
    pub jobs: Vec<CronJob>,
    /// Cron expression for automatic memory consolidation. `None` = disabled.
    #[serde(default)]
    pub memory_consolidation: Option<String>,
    /// Cron expression for automatic memory expiry sweeps. `None` = disabled.
    #[serde(default)]
    pub memory_expiry: Option<String>,
}

impl WebhookPlatformConfig {
    /// Defaults for the generic webhook adapter. Used when no explicit
    /// `platforms.webhook` block is present so existing setups keep working.
    pub fn default_webhook() -> Self {
        Self {
            enabled: true,
            port: 3001,
            webhook_path: "/webhook".to_string(),
        }
    }

    /// Defaults for LINE. Constructed by the setup wizard when the user opts
    /// in, so `enabled = true`; for manual yaml authors, `enabled` itself
    /// defaults to `false` via serde, keeping the adapter opt-in.
    pub fn default_line() -> Self {
        Self {
            enabled: true,
            port: 3002,
            webhook_path: "/line".to_string(),
        }
    }

    /// Defaults for WhatsApp — same semantics as `default_line`.
    pub fn default_whatsapp() -> Self {
        Self {
            enabled: true,
            port: 3003,
            webhook_path: "/whatsapp".to_string(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let home = dirs::home_dir().unwrap_or_default();
        Self {
            home_dir: Self::garudust_dir(),
            model: DEFAULT_MODEL.into(),
            max_iterations: 90,
            sub_agent_max_iterations: None,
            max_delegation_depth: 1,
            tool_delay_ms: 0,
            provider: DEFAULT_PROVIDER.into(),
            base_url: None,
            routing: std::collections::HashMap::new(),
            tools: std::collections::HashMap::new(),
            api_key: None,
            fallback_api_keys: Vec::new(),
            compression: CompressionConfig::default(),
            mcp_servers: Vec::new(),
            max_concurrent_requests: None,
            security: SecurityConfig {
                gateway_api_key: None,
                allowed_read_paths: vec![cwd.clone(), home],
                allowed_write_paths: vec![cwd],
                approval_mode: default_approval_mode(),
                rate_limit_rpm: None,
                terminal_sandbox: TerminalSandbox::None,
                terminal_sandbox_image: default_sandbox_image(),
                terminal_sandbox_opts: Vec::new(),
            },
            memory_expiry: MemoryExpiryConfig::default(),
            nudge_interval: default_nudge_interval(),
            llm_max_retries: default_llm_max_retries(),
            llm_retry_base_ms: default_llm_retry_base_ms(),
            platform: PlatformConfig::default(),
            auto_skill_threshold: default_auto_skill_threshold(),
            llm_timeout_secs: default_llm_timeout_secs(),
            tool_timeout_secs: default_tool_timeout_secs(),
            shutdown_timeout_secs: default_shutdown_timeout_secs(),
            max_tokens_per_task: None,
            max_output_tokens: None,
            reasoning_effort: None,
            context_window: None,
            disabled_toolsets: default_disabled_toolsets(),
            disabled_tools: Vec::new(),
            show_usage_footer: false,
            max_memory_tokens: None,
            platforms: WebhookPlatformsConfig {
                webhook: Some(WebhookPlatformConfig::default_webhook()),
                line: None,
                whatsapp: None,
            },
            server: ServerConfig::default(),
            cron: CronConfig::default(),
        }
    }
}

/// Map a provider name to its API-key env var and return the value.
/// Used when config.yaml is authoritative (provider is already known).
pub(crate) fn resolve_key_for_provider(
    provider: &str,
    dotenv: &HashMap<String, String>,
) -> Option<String> {
    match provider {
        "anthropic" => env_or_dotenv("ANTHROPIC_API_KEY", dotenv),
        "openai" => env_or_dotenv("OPENAI_API_KEY", dotenv),
        "gemini" => env_or_dotenv("GEMINI_API_KEY", dotenv),
        "groq" => env_or_dotenv("GROQ_API_KEY", dotenv),
        "mistral" => env_or_dotenv("MISTRAL_API_KEY", dotenv),
        "deepseek" => env_or_dotenv("DEEPSEEK_API_KEY", dotenv),
        "xai" => env_or_dotenv("XAI_API_KEY", dotenv),
        "vllm" => env_or_dotenv("VLLM_API_KEY", dotenv),
        "thaillm" => env_or_dotenv("THAILLM_API_KEY", dotenv),
        "ollama" | "bedrock" | "codex" => None,
        _ => env_or_dotenv("OPENROUTER_API_KEY", dotenv),
    }
}

/// Detect provider and API key from environment when no config.yaml exists.
/// Priority: anthropic → openai → gemini → groq → mistral → deepseek → xai
///           → ollama → vllm → thaillm → openrouter.
pub(crate) fn detect_provider_from_env(config: &mut AgentConfig, dotenv: &HashMap<String, String>) {
    if let Some(k) = env_or_dotenv("ANTHROPIC_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "anthropic".into();
    } else if let Some(k) = env_or_dotenv("OPENAI_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "openai".into();
    } else if let Some(k) = env_or_dotenv("GEMINI_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "gemini".into();
    } else if let Some(k) = env_or_dotenv("GROQ_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "groq".into();
    } else if let Some(k) = env_or_dotenv("MISTRAL_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "mistral".into();
    } else if let Some(k) = env_or_dotenv("DEEPSEEK_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "deepseek".into();
    } else if let Some(k) = env_or_dotenv("XAI_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "xai".into();
    } else if let Some(url) = env_or_dotenv("OLLAMA_BASE_URL", dotenv) {
        config.provider = "ollama".into();
        config.base_url = Some(url);
    } else if let Some(url) = env_or_dotenv("VLLM_BASE_URL", dotenv) {
        config.provider = "vllm".into();
        config.base_url = Some(url);
        config.api_key = env_or_dotenv("VLLM_API_KEY", dotenv);
    } else if let Some(k) = env_or_dotenv("THAILLM_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "thaillm".into();
    } else if let Some(k) = env_or_dotenv("OPENROUTER_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "openrouter".into();
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

        // Provider→env binding rule:
        //   1. If config.yaml explicitly set the provider, load *only* that provider's
        //      env key. Other providers' keys (e.g. OPENROUTER_API_KEY left in .env for
        //      tools like view_image) must not leak into config.api_key.
        //   2. If yaml did not exist (config.provider is still default), allow env-based
        //      auto-detection: ANTHROPIC_API_KEY → anthropic, OPENROUTER_API_KEY → openrouter,
        //      VLLM_BASE_URL → vllm, etc.
        //
        // This prevents the "tool credential leaks into LLM transport" bug while
        // preserving zero-config UX for users who only set one *_API_KEY in env.
        let yaml_authoritative = yaml_path.exists();

        if yaml_authoritative {
            if config.api_key.is_none() {
                config.api_key = resolve_key_for_provider(&config.provider, dotenv);
            }
        } else {
            detect_provider_from_env(&mut config, dotenv);
        }
        if let Some(m) = env_or_dotenv("GARUDUST_MODEL", dotenv) {
            config.model = m;
        }
        if let Some(u) = env_or_dotenv("GARUDUST_BASE_URL", dotenv) {
            config.base_url = Some(u);
        }
        if let Some(v) = env_or_dotenv("LLM_FALLBACK_API_KEYS", dotenv) {
            config.fallback_api_keys = v
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
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
        if let Some(sandbox) = env_or_dotenv("GARUDUST_TERMINAL_SANDBOX", dotenv) {
            config.security.terminal_sandbox = match sandbox.to_lowercase().as_str() {
                "docker" => TerminalSandbox::Docker,
                _ => TerminalSandbox::None,
            };
        }
        if let Some(image) = env_or_dotenv("GARUDUST_SANDBOX_IMAGE", dotenv) {
            config.security.terminal_sandbox_image = image;
        }

        // Non-secret env vars that previously reached clap via `dotenvy::from_path`.
        // Reading them here lets us drop dotenvy from main.rs without losing the
        // ability for operators to set these in ~/.garudust/.env. CLI flags still
        // override these because main.rs applies CLI > config precedence at use sites.
        if let Some(v) = env_or_dotenv("GARUDUST_PORT", dotenv) {
            if let Ok(n) = v.parse::<u16>() {
                config.server.port = n;
            }
        }
        if let Some(v) = env_or_dotenv("GARUDUST_MEMORY_CRON", dotenv) {
            config.cron.memory_consolidation = Some(v);
        }
        if let Some(v) = env_or_dotenv("GARUDUST_MEMORY_EXPIRY_CRON", dotenv) {
            config.cron.memory_expiry = Some(v);
        }
        if let Some(v) = env_or_dotenv("GARUDUST_CRON_JOBS", dotenv) {
            config.cron.jobs = parse_cron_jobs_str(&v);
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{detect_provider_from_env, resolve_key_for_provider, AgentConfig};

    fn dotenv(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // ── resolve_key_for_provider ──────────────────────────────────────────────

    #[test]
    fn resolve_openai_key() {
        let map = dotenv(&[("OPENAI_API_KEY", "sk-test-openai")]);
        assert_eq!(
            resolve_key_for_provider("openai", &map),
            Some("sk-test-openai".into())
        );
    }

    #[test]
    fn resolve_gemini_key() {
        let map = dotenv(&[("GEMINI_API_KEY", "AIza-test")]);
        assert_eq!(
            resolve_key_for_provider("gemini", &map),
            Some("AIza-test".into())
        );
    }

    #[test]
    fn resolve_groq_key() {
        let map = dotenv(&[("GROQ_API_KEY", "gsk-test")]);
        assert_eq!(
            resolve_key_for_provider("groq", &map),
            Some("gsk-test".into())
        );
    }

    #[test]
    fn resolve_mistral_key() {
        let map = dotenv(&[("MISTRAL_API_KEY", "ms-test")]);
        assert_eq!(
            resolve_key_for_provider("mistral", &map),
            Some("ms-test".into())
        );
    }

    #[test]
    fn resolve_deepseek_key() {
        let map = dotenv(&[("DEEPSEEK_API_KEY", "ds-test")]);
        assert_eq!(
            resolve_key_for_provider("deepseek", &map),
            Some("ds-test".into())
        );
    }

    #[test]
    fn resolve_xai_key() {
        let map = dotenv(&[("XAI_API_KEY", "xai-test")]);
        assert_eq!(
            resolve_key_for_provider("xai", &map),
            Some("xai-test".into())
        );
    }

    #[test]
    fn resolve_ollama_returns_none() {
        let map = dotenv(&[("OPENROUTER_API_KEY", "or-test")]);
        assert_eq!(resolve_key_for_provider("ollama", &map), None);
    }

    #[test]
    fn resolve_unknown_provider_falls_back_to_openrouter() {
        let map = dotenv(&[("OPENROUTER_API_KEY", "or-test")]);
        assert_eq!(
            resolve_key_for_provider("custom-provider", &map),
            Some("or-test".into())
        );
    }

    // ── detect_provider_from_env ──────────────────────────────────────────────

    fn detect(pairs: &[(&str, &str)]) -> AgentConfig {
        let mut cfg = AgentConfig::default();
        detect_provider_from_env(&mut cfg, &dotenv(pairs));
        cfg
    }

    #[test]
    fn detect_openai_only() {
        let cfg = detect(&[("OPENAI_API_KEY", "sk-test-openai")]);
        assert_eq!(cfg.provider, "openai");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-test-openai"));
    }

    #[test]
    fn detect_gemini_only() {
        let cfg = detect(&[("GEMINI_API_KEY", "AIza-test")]);
        assert_eq!(cfg.provider, "gemini");
        assert_eq!(cfg.api_key.as_deref(), Some("AIza-test"));
    }

    #[test]
    fn detect_groq_only() {
        let cfg = detect(&[("GROQ_API_KEY", "gsk-test")]);
        assert_eq!(cfg.provider, "groq");
        assert_eq!(cfg.api_key.as_deref(), Some("gsk-test"));
    }

    #[test]
    fn detect_mistral_only() {
        let cfg = detect(&[("MISTRAL_API_KEY", "ms-test")]);
        assert_eq!(cfg.provider, "mistral");
        assert_eq!(cfg.api_key.as_deref(), Some("ms-test"));
    }

    #[test]
    fn detect_deepseek_only() {
        let cfg = detect(&[("DEEPSEEK_API_KEY", "ds-test")]);
        assert_eq!(cfg.provider, "deepseek");
        assert_eq!(cfg.api_key.as_deref(), Some("ds-test"));
    }

    #[test]
    fn detect_xai_only() {
        let cfg = detect(&[("XAI_API_KEY", "xai-test")]);
        assert_eq!(cfg.provider, "xai");
        assert_eq!(cfg.api_key.as_deref(), Some("xai-test"));
    }

    #[test]
    fn detect_openrouter_only() {
        let cfg = detect(&[("OPENROUTER_API_KEY", "or-test")]);
        assert_eq!(cfg.provider, "openrouter");
        assert_eq!(cfg.api_key.as_deref(), Some("or-test"));
    }

    #[test]
    fn detect_ollama_sets_base_url_not_key() {
        let cfg = detect(&[("OLLAMA_BASE_URL", "http://localhost:11434")]);
        assert_eq!(cfg.provider, "ollama");
        assert_eq!(cfg.base_url.as_deref(), Some("http://localhost:11434"));
        assert!(cfg.api_key.is_none());
    }

    #[test]
    fn detect_vllm_sets_base_url_and_key() {
        let cfg = detect(&[
            ("VLLM_BASE_URL", "http://localhost:8000/v1"),
            ("VLLM_API_KEY", "vllm-test"),
        ]);
        assert_eq!(cfg.provider, "vllm");
        assert_eq!(cfg.base_url.as_deref(), Some("http://localhost:8000/v1"));
        assert_eq!(cfg.api_key.as_deref(), Some("vllm-test"));
    }

    #[test]
    fn detect_empty_env_leaves_defaults() {
        let cfg = detect(&[]);
        assert_eq!(cfg.provider, "openrouter");
        assert!(cfg.api_key.is_none());
    }

    // Priority: openai loses to anthropic when both are present in the dotenv
    // map and neither is in the real process environment.
    // (This test assumes ANTHROPIC_API_KEY is not set in the test runner's env.)
    #[test]
    fn detect_anthropic_wins_over_openai_in_dotenv() {
        let cfg = detect(&[
            ("ANTHROPIC_API_KEY", "sk-ant-test"),
            ("OPENAI_API_KEY", "sk-oai-test"),
        ]);
        // anthropic is first in the priority chain, so it wins
        assert_eq!(cfg.provider, "anthropic");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-ant-test"));
    }
}
