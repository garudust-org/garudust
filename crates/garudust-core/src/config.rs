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

/// Metadata for a built-in OpenAI-compatible provider.
/// This is the single source of truth used by both the config loader and the
/// transport layer — no more hardcoded duplicates in multiple match arms.
#[derive(Debug, Clone, Copy)]
pub struct BuiltinProvider {
    pub name: &'static str,
    pub base_url: &'static str,
    pub api_key_env: &'static str,
    /// JSON field name for the token limit sent to the API.
    pub tokens_param: &'static str,
}

/// All built-in OpenAI-compatible providers in detection-priority order.
/// Special transports (anthropic-native, bedrock, ollama, codex) are handled
/// separately in the transport layer.
pub const BUILTIN_PROVIDERS: &[BuiltinProvider] = &[
    BuiltinProvider {
        name: "openai",
        base_url: "https://api.openai.com/v1",
        api_key_env: "OPENAI_API_KEY",
        tokens_param: "max_completion_tokens",
    },
    BuiltinProvider {
        name: "gemini",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        api_key_env: "GEMINI_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "groq",
        base_url: "https://api.groq.com/openai/v1",
        api_key_env: "GROQ_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "mistral",
        base_url: "https://api.mistral.ai/v1",
        api_key_env: "MISTRAL_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "deepseek",
        base_url: "https://api.deepseek.com/v1",
        api_key_env: "DEEPSEEK_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "xai",
        base_url: "https://api.x.ai/v1",
        api_key_env: "XAI_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "together",
        base_url: "https://api.together.xyz/v1",
        api_key_env: "TOGETHER_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "fireworks",
        base_url: "https://api.fireworks.ai/inference/v1",
        api_key_env: "FIREWORKS_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "cerebras",
        base_url: "https://api.cerebras.ai/v1",
        api_key_env: "CEREBRAS_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "perplexity",
        base_url: "https://api.perplexity.ai",
        api_key_env: "PERPLEXITY_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "cohere",
        base_url: "https://api.cohere.com/compatibility/v1",
        api_key_env: "COHERE_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "nvidia",
        base_url: "https://integrate.api.nvidia.com/v1",
        api_key_env: "NVIDIA_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "alibaba",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        api_key_env: "DASHSCOPE_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "doubao",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        api_key_env: "ARK_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "zhipu",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        api_key_env: "ZHIPU_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "moonshot",
        base_url: "https://api.moonshot.cn/v1",
        api_key_env: "MOONSHOT_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "baidu",
        base_url: "https://qianfan.baidubce.com/v2",
        api_key_env: "QIANFAN_API_KEY",
        tokens_param: "max_tokens",
    },
    BuiltinProvider {
        name: "thaillm",
        base_url: "http://thaillm.or.th/api/v1",
        api_key_env: "THAILLM_API_KEY",
        tokens_param: "max_completion_tokens",
    },
    BuiltinProvider {
        name: "vllm",
        base_url: "http://localhost:8000/v1",
        api_key_env: "VLLM_API_KEY",
        tokens_param: "max_completion_tokens",
    },
    BuiltinProvider {
        name: "openrouter",
        base_url: "https://openrouter.ai/api/v1",
        api_key_env: "OPENROUTER_API_KEY",
        tokens_param: "max_completion_tokens",
    },
];

/// User-defined provider profile declared in `config.yaml` under `providers:`.
///
/// Example:
/// ```yaml
/// providers:
///   default:
///     name: groq
///     key: ${GROQ_API_KEY}
///     model: llama-3.3-70b
///   groq-backup:
///     name: groq
///     key: ${GROQ_API_KEY_2}
///   local:
///     url: http://192.168.1.10:8000/v1
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderProfile {
    /// Builtin provider name — inherits `base_url` and `tokens_param`.
    /// Optional when `url` is set directly.
    #[serde(default)]
    pub name: Option<String>,
    /// Custom base URL. Overrides the builtin default for `name`.
    #[serde(default)]
    pub url: Option<String>,
    /// API key literal or `${ENV_VAR}` reference.
    #[serde(default)]
    pub key: Option<String>,
    /// Default model — meaningful only for the `default` profile.
    #[serde(default)]
    pub model: Option<String>,
}

impl ProviderProfile {
    /// Resolve the `key` field: `${ENV_VAR}` → environment value, literal → itself.
    pub fn resolved_key(&self) -> Option<String> {
        let k = self.key.as_deref()?;
        if let Some(var) = k.strip_prefix("${").and_then(|s| s.strip_suffix('}')) {
            get_secret(var)
        } else {
            Some(k.to_string())
        }
    }

    /// Effective base URL for this profile: an explicit `url:`, otherwise the
    /// built-in default for `name:`. Returns `None` for special transports
    /// (anthropic / ollama / bedrock) that have no entry in `BUILTIN_PROVIDERS`
    /// and for profiles with neither a `url` nor a recognised `name`.
    pub fn resolved_base_url(&self) -> Option<String> {
        if let Some(url) = &self.url {
            return Some(url.clone());
        }
        let name = self.name.as_deref()?;
        BUILTIN_PROVIDERS
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.base_url.to_string())
    }
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
    /// Named provider profiles. The special name `default` acts as the main LLM
    /// provider, overriding the top-level `provider:` / `model:` fields.
    /// Routing hints reference profiles by name (`profile-name/model`).
    ///
    /// Example:
    /// ```yaml
    /// providers:
    ///   default:
    ///     name: groq
    ///     key: ${GROQ_API_KEY}
    ///     model: llama-3.3-70b
    ///   groq-backup:
    ///     name: groq
    ///     key: ${GROQ_API_KEY_2}
    ///   local:
    ///     url: http://192.168.1.10:8000/v1
    /// ```
    #[serde(default)]
    pub providers: std::collections::HashMap<String, ProviderProfile>,
    /// Provider routing table: hint name → "provider/model" or "profile/model" string.
    /// Example: `cheap: groq/llama-3.1-8b-instant`
    /// When a hint is passed to agent.run(), the agent looks up the target here,
    /// builds an appropriate transport, and overrides the model for that task only.
    #[serde(default)]
    pub routing: std::collections::HashMap<String, String>,
    /// Per-tool model configuration, keyed by tool name → named provider slot.
    /// Each slot is a `ProviderProfile` (name/url/key/model). Slots whose name
    /// contains `"fallback"` are injected as `GARUDUST_FALLBACK_*` env vars;
    /// all others as `GARUDUST_*` (primary).
    ///
    /// Example:
    /// ```yaml
    /// tools:
    ///   view_image:
    ///     vision:
    ///       name: google
    ///       key: ${GOOGLE_AI_API_KEY}
    ///       model: gemini-flash-latest
    ///     vision-fallback:
    ///       name: openrouter
    ///       key: ${OPENROUTER_API_KEY}
    ///       model: nvidia/nemotron-nano-12b-v2-vl:free
    ///   generate_image:
    ///     gen:
    ///       url: https://router.huggingface.co/hf-inference/models
    ///       key: ${HF_TOKEN}
    ///       model: black-forest-labs/FLUX.1-schnell
    /// ```
    #[serde(default)]
    pub tools: std::collections::HashMap<String, std::collections::HashMap<String, ProviderProfile>>,
    /// Per-skill model configuration — same named-slot format as `tools`.
    #[serde(default)]
    pub skills: std::collections::HashMap<String, std::collections::HashMap<String, ProviderProfile>>,
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
    ///   openrouter, vllm, ollama, bedrock, codex, thaillm,
    ///   together, fireworks, cerebras, perplexity, cohere, nvidia,
    ///   alibaba, doubao, zhipu, moonshot, baidu
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
            providers: std::collections::HashMap::new(),
            routing: std::collections::HashMap::new(),
            tools: std::collections::HashMap::new(),
            skills: std::collections::HashMap::new(),
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
    if matches!(provider, "ollama" | "bedrock" | "codex") {
        return None;
    }
    if provider == "anthropic" {
        return env_or_dotenv("ANTHROPIC_API_KEY", dotenv);
    }
    if let Some(p) = BUILTIN_PROVIDERS.iter().find(|p| p.name == provider) {
        return env_or_dotenv(p.api_key_env, dotenv);
    }
    env_or_dotenv("OPENROUTER_API_KEY", dotenv)
}

/// Detect provider and API key from environment when no config.yaml exists.
/// Priority order follows BUILTIN_PROVIDERS, with anthropic first (special
/// transport), then ollama/vllm (URL-based), thaillm, and openrouter last.
pub(crate) fn detect_provider_from_env(config: &mut AgentConfig, dotenv: &HashMap<String, String>) {
    // anthropic: special transport, highest priority
    if let Some(k) = env_or_dotenv("ANTHROPIC_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "anthropic".into();
        return;
    }
    // All BUILTIN_PROVIDERS in table order; skip the URL-based and fallback ones
    for p in BUILTIN_PROVIDERS {
        if matches!(p.name, "thaillm" | "vllm" | "openrouter") {
            continue;
        }
        if let Some(k) = env_or_dotenv(p.api_key_env, dotenv) {
            config.api_key = Some(k);
            config.provider = p.name.into();
            return;
        }
    }
    // ollama and vllm: detected by base_url, not API key
    if let Some(url) = env_or_dotenv("OLLAMA_BASE_URL", dotenv) {
        config.provider = "ollama".into();
        config.base_url = Some(url);
        return;
    }
    if let Some(url) = env_or_dotenv("VLLM_BASE_URL", dotenv) {
        config.provider = "vllm".into();
        config.base_url = Some(url);
        config.api_key = env_or_dotenv("VLLM_API_KEY", dotenv);
        return;
    }
    if let Some(k) = env_or_dotenv("THAILLM_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "thaillm".into();
        return;
    }
    if let Some(k) = env_or_dotenv("OPENROUTER_API_KEY", dotenv) {
        config.api_key = Some(k);
        config.provider = "openrouter".into();
    }
}

impl AgentConfig {
    /// Effective transport base URL, honouring the new `providers.default`
    /// profile first (its `url:` or the built-in default for its `name:`),
    /// then falling back to the legacy top-level `base_url:` field.
    /// `None` means "use the provider's built-in default" — callers that need
    /// a concrete URL (doctor, `config show`) supply their own provider match.
    pub fn effective_base_url(&self) -> Option<String> {
        if let Some(p) = self.providers.get("default") {
            if let Some(url) = p.resolved_base_url() {
                return Some(url);
            }
        }
        self.base_url.clone()
    }

    /// Effective API key, honouring the `providers.default` profile's resolved
    /// `key:` first, then the legacy `api_key` field populated by `load()`.
    pub fn effective_api_key(&self) -> Option<String> {
        if let Some(p) = self.providers.get("default") {
            if let Some(k) = p.resolved_key() {
                return Some(k);
            }
        }
        self.api_key.clone()
    }

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

        // Apply `providers.default` overrides: name → provider, model → model.
        if let Some(default_profile) = config.providers.get("default") {
            if let Some(name) = &default_profile.name {
                if !name.is_empty() {
                    config.provider = name.clone();
                }
            }
            if let Some(model) = &default_profile.model {
                if !model.is_empty() {
                    config.model = model.clone();
                }
            }
        }

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

    // ── resolve_key — new providers ───────────────────────────────────────────

    #[test]
    fn resolve_together_key() {
        let map = dotenv(&[("TOGETHER_API_KEY", "tog-test")]);
        assert_eq!(
            resolve_key_for_provider("together", &map),
            Some("tog-test".into())
        );
    }

    #[test]
    fn resolve_fireworks_key() {
        let map = dotenv(&[("FIREWORKS_API_KEY", "fw-test")]);
        assert_eq!(
            resolve_key_for_provider("fireworks", &map),
            Some("fw-test".into())
        );
    }

    #[test]
    fn resolve_cerebras_key() {
        let map = dotenv(&[("CEREBRAS_API_KEY", "cb-test")]);
        assert_eq!(
            resolve_key_for_provider("cerebras", &map),
            Some("cb-test".into())
        );
    }

    #[test]
    fn resolve_perplexity_key() {
        let map = dotenv(&[("PERPLEXITY_API_KEY", "pplx-test")]);
        assert_eq!(
            resolve_key_for_provider("perplexity", &map),
            Some("pplx-test".into())
        );
    }

    #[test]
    fn resolve_cohere_key() {
        let map = dotenv(&[("COHERE_API_KEY", "co-test")]);
        assert_eq!(
            resolve_key_for_provider("cohere", &map),
            Some("co-test".into())
        );
    }

    #[test]
    fn resolve_nvidia_key() {
        let map = dotenv(&[("NVIDIA_API_KEY", "nvapi-test")]);
        assert_eq!(
            resolve_key_for_provider("nvidia", &map),
            Some("nvapi-test".into())
        );
    }

    #[test]
    fn resolve_alibaba_key() {
        let map = dotenv(&[("DASHSCOPE_API_KEY", "sk-ds-test")]);
        assert_eq!(
            resolve_key_for_provider("alibaba", &map),
            Some("sk-ds-test".into())
        );
    }

    #[test]
    fn resolve_doubao_key() {
        let map = dotenv(&[("ARK_API_KEY", "ark-test")]);
        assert_eq!(
            resolve_key_for_provider("doubao", &map),
            Some("ark-test".into())
        );
    }

    #[test]
    fn resolve_zhipu_key() {
        let map = dotenv(&[("ZHIPU_API_KEY", "zp-test")]);
        assert_eq!(
            resolve_key_for_provider("zhipu", &map),
            Some("zp-test".into())
        );
    }

    #[test]
    fn resolve_moonshot_key() {
        let map = dotenv(&[("MOONSHOT_API_KEY", "ms-kimi-test")]);
        assert_eq!(
            resolve_key_for_provider("moonshot", &map),
            Some("ms-kimi-test".into())
        );
    }

    #[test]
    fn resolve_baidu_key() {
        let map = dotenv(&[("QIANFAN_API_KEY", "qf-test")]);
        assert_eq!(
            resolve_key_for_provider("baidu", &map),
            Some("qf-test".into())
        );
    }

    // ── detect_provider_from_env — new providers ──────────────────────────────

    #[test]
    fn detect_together_only() {
        let cfg = detect(&[("TOGETHER_API_KEY", "tog-test")]);
        assert_eq!(cfg.provider, "together");
        assert_eq!(cfg.api_key.as_deref(), Some("tog-test"));
    }

    #[test]
    fn detect_fireworks_only() {
        let cfg = detect(&[("FIREWORKS_API_KEY", "fw-test")]);
        assert_eq!(cfg.provider, "fireworks");
        assert_eq!(cfg.api_key.as_deref(), Some("fw-test"));
    }

    #[test]
    fn detect_cerebras_only() {
        let cfg = detect(&[("CEREBRAS_API_KEY", "cb-test")]);
        assert_eq!(cfg.provider, "cerebras");
        assert_eq!(cfg.api_key.as_deref(), Some("cb-test"));
    }

    #[test]
    fn detect_perplexity_only() {
        let cfg = detect(&[("PERPLEXITY_API_KEY", "pplx-test")]);
        assert_eq!(cfg.provider, "perplexity");
        assert_eq!(cfg.api_key.as_deref(), Some("pplx-test"));
    }

    #[test]
    fn detect_cohere_only() {
        let cfg = detect(&[("COHERE_API_KEY", "co-test")]);
        assert_eq!(cfg.provider, "cohere");
        assert_eq!(cfg.api_key.as_deref(), Some("co-test"));
    }

    #[test]
    fn detect_nvidia_only() {
        let cfg = detect(&[("NVIDIA_API_KEY", "nvapi-test")]);
        assert_eq!(cfg.provider, "nvidia");
        assert_eq!(cfg.api_key.as_deref(), Some("nvapi-test"));
    }

    #[test]
    fn detect_alibaba_only() {
        let cfg = detect(&[("DASHSCOPE_API_KEY", "sk-ds-test")]);
        assert_eq!(cfg.provider, "alibaba");
        assert_eq!(cfg.api_key.as_deref(), Some("sk-ds-test"));
    }

    #[test]
    fn detect_doubao_only() {
        let cfg = detect(&[("ARK_API_KEY", "ark-test")]);
        assert_eq!(cfg.provider, "doubao");
        assert_eq!(cfg.api_key.as_deref(), Some("ark-test"));
    }

    #[test]
    fn detect_zhipu_only() {
        let cfg = detect(&[("ZHIPU_API_KEY", "zp-test")]);
        assert_eq!(cfg.provider, "zhipu");
        assert_eq!(cfg.api_key.as_deref(), Some("zp-test"));
    }

    #[test]
    fn detect_moonshot_only() {
        let cfg = detect(&[("MOONSHOT_API_KEY", "ms-kimi-test")]);
        assert_eq!(cfg.provider, "moonshot");
        assert_eq!(cfg.api_key.as_deref(), Some("ms-kimi-test"));
    }

    #[test]
    fn detect_baidu_only() {
        let cfg = detect(&[("QIANFAN_API_KEY", "qf-test")]);
        assert_eq!(cfg.provider, "baidu");
        assert_eq!(cfg.api_key.as_deref(), Some("qf-test"));
    }

    // ── ProviderProfile::resolved_key ─────────────────────────────────────────

    #[test]
    fn profile_resolved_key_literal() {
        let p = super::ProviderProfile {
            key: Some("sk-literal".into()),
            ..Default::default()
        };
        assert_eq!(p.resolved_key(), Some("sk-literal".into()));
    }

    #[test]
    fn profile_resolved_key_none_when_absent() {
        let p = super::ProviderProfile::default();
        assert!(p.resolved_key().is_none());
    }

    #[test]
    fn profile_resolved_key_env_var_interpolation() {
        // Set a unique env var just for this test.
        std::env::set_var("GARUDUST_TEST_KEY_INTERP", "env-value-123");
        let p = super::ProviderProfile {
            key: Some("${GARUDUST_TEST_KEY_INTERP}".into()),
            ..Default::default()
        };
        assert_eq!(p.resolved_key(), Some("env-value-123".into()));
        std::env::remove_var("GARUDUST_TEST_KEY_INTERP");
    }

    #[test]
    fn profile_resolved_key_missing_env_var_returns_none() {
        std::env::remove_var("GARUDUST_TEST_KEY_MISSING");
        let p = super::ProviderProfile {
            key: Some("${GARUDUST_TEST_KEY_MISSING}".into()),
            ..Default::default()
        };
        assert!(p.resolved_key().is_none());
    }

    // ── providers.default → config.provider / model ───────────────────────────

    #[test]
    fn providers_default_overrides_provider_and_model() {
        let yaml = "
providers:
  default:
    name: groq
    model: llama-3.3-70b-versatile
";
        let mut cfg: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        // Simulate the load() post-processing step.
        if let Some(default_profile) = cfg.providers.get("default") {
            if let Some(name) = &default_profile.name.clone() {
                if !name.is_empty() {
                    cfg.provider = name.clone();
                }
            }
            if let Some(model) = &default_profile.model.clone() {
                if !model.is_empty() {
                    cfg.model = model.clone();
                }
            }
        }
        assert_eq!(cfg.provider, "groq");
        assert_eq!(cfg.model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn providers_map_deserializes_correctly() {
        let yaml = r#"
providers:
  groq-backup:
    name: groq
    key: "${GROQ_API_KEY_2}"
  local:
    url: "http://192.168.1.10:8000/v1"
"#;
        let cfg: AgentConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(cfg.providers.contains_key("groq-backup"));
        assert!(cfg.providers.contains_key("local"));
        let backup = &cfg.providers["groq-backup"];
        assert_eq!(backup.name.as_deref(), Some("groq"));
        assert_eq!(backup.key.as_deref(), Some("${GROQ_API_KEY_2}"));
        let local = &cfg.providers["local"];
        assert_eq!(local.url.as_deref(), Some("http://192.168.1.10:8000/v1"));
    }
}
