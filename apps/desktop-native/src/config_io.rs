//! Direct config.yaml / .env access (embedded mode — no HTTP gateway needed).

use std::path::PathBuf;

use garudust_core::config::{AgentConfig, TerminalSandbox};

pub const PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "gemini",
    "groq",
    "mistral",
    "deepseek",
    "ollama",
    "openrouter",
    "vllm",
    "bedrock",
    "xai",
    "together",
    "fireworks",
    "cerebras",
    "perplexity",
    "cohere",
    "nvidia",
    "alibaba",
    "doubao",
    "zhipu",
    "moonshot",
    "baidu",
    "thaillm",
    "codex",
];
pub const APPROVAL_MODES: &[&str] = &["auto", "smart", "deny", "interactive"];
pub const SANDBOX_MODES: &[&str] = &["none", "docker", "ssh"];

pub fn default_model(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "claude-sonnet-4-6",
        "openai" => "gpt-4o",
        "gemini" => "gemini-2.0-flash",
        "groq" => "llama-3.3-70b-versatile",
        "mistral" => "mistral-large-latest",
        "deepseek" => "deepseek-chat",
        "ollama" => "llama3.2",
        "openrouter" => "anthropic/claude-sonnet-4-6",
        "xai" => "grok-2-latest",
        "together" => "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        "perplexity" => "sonar",
        "cohere" => "command-r-plus",
        "nvidia" => "meta/llama-3.3-70b-instruct",
        "cerebras" => "llama-3.3-70b",
        _ => "",
    }
}

pub fn provider_key_env(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "xai" => "XAI_API_KEY",
        "together" => "TOGETHER_API_KEY",
        "fireworks" => "FIREWORKS_API_KEY",
        "cerebras" => "CEREBRAS_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "cohere" => "COHERE_API_KEY",
        "nvidia" => "NVIDIA_API_KEY",
        "alibaba" => "DASHSCOPE_API_KEY",
        "doubao" => "ARK_API_KEY",
        "zhipu" => "ZHIPU_API_KEY",
        "moonshot" => "MOONSHOT_API_KEY",
        "baidu" => "QIANFAN_API_KEY",
        "thaillm" => "THAILLM_API_KEY",
        "vllm" => "VLLM_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        _ => "",
    }
}

/// Editable view of the config fields the UI exposes. Other fields are
/// preserved on save (we load the full config, mutate, then write it back).
#[derive(Default, Clone)]
pub struct ConfigForm {
    pub model: String,
    pub provider: String,
    pub base_url: String,
    pub reflection_model: String,
    pub max_iterations: u32,
    pub nudge_interval: u32,
    pub auto_skill_threshold: u32,
    pub max_history_pairs: usize,
    pub approval_mode: String,
    pub terminal_sandbox: String,
    pub routing: Vec<(String, String)>,
}

impl ConfigForm {
    pub fn load() -> Self {
        let c = AgentConfig::load();
        let mut routing: Vec<(String, String)> = c
            .routing
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        routing.sort();
        Self {
            model: c.model.clone(),
            provider: c.provider.clone(),
            base_url: c.base_url.clone().unwrap_or_default(),
            reflection_model: c.reflection_model.clone().unwrap_or_default(),
            max_iterations: c.max_iterations,
            nudge_interval: c.nudge_interval,
            auto_skill_threshold: c.auto_skill_threshold,
            max_history_pairs: c.max_history_pairs,
            approval_mode: c.security.approval_mode.clone(),
            terminal_sandbox: match c.security.terminal_sandbox {
                TerminalSandbox::Docker => "docker",
                TerminalSandbox::Ssh => "ssh",
                TerminalSandbox::None => "none",
            }
            .to_string(),
            routing,
        }
    }

    /// Write back: load the full config, apply exposed fields, save atomically.
    pub fn save(&self) -> std::io::Result<()> {
        let mut c = AgentConfig::load();
        c.model = self.model.clone();
        c.provider = self.provider.clone();
        c.base_url = opt(&self.base_url);
        c.reflection_model = opt(&self.reflection_model);
        c.max_iterations = self.max_iterations;
        c.nudge_interval = self.nudge_interval;
        c.auto_skill_threshold = self.auto_skill_threshold;
        c.max_history_pairs = self.max_history_pairs;
        c.security.approval_mode = self.approval_mode.clone();
        c.security.terminal_sandbox = match self.terminal_sandbox.as_str() {
            "docker" => TerminalSandbox::Docker,
            "ssh" => TerminalSandbox::Ssh,
            _ => TerminalSandbox::None,
        };
        c.routing = self
            .routing
            .iter()
            .filter(|(h, _)| !h.trim().is_empty())
            .map(|(h, t)| (h.trim().to_string(), t.trim().to_string()))
            .collect();
        c.save_yaml()
    }
}

fn opt(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

pub fn home_dir() -> PathBuf {
    AgentConfig::load().home_dir
}

// ── Secrets (.env) ──────────────────────────────────────────────────────────

pub struct EnvEntry {
    pub key: String,
    pub masked: String,
}

fn mask(v: &str) -> String {
    let n = v.chars().count();
    if n <= 4 {
        "••••••".to_string()
    } else {
        let last4: String = v.chars().skip(n - 4).collect();
        format!("••••••{last4}")
    }
}

pub fn list_env() -> Vec<EnvEntry> {
    let path = home_dir().join(".env");
    let Ok(contents) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (k, v) = line.split_once('=')?;
            let k = k.trim();
            if k.is_empty() {
                return None;
            }
            Some(EnvEntry {
                key: k.to_string(),
                masked: mask(v.trim()),
            })
        })
        .collect()
}

pub fn valid_env_key(k: &str) -> bool {
    let mut ch = k.chars();
    match ch.next() {
        Some(c) if c.is_ascii_uppercase() || c == '_' => {}
        _ => return false,
    }
    ch.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

pub fn set_env(key: &str, value: &str) -> std::io::Result<()> {
    AgentConfig::set_env_var(&home_dir(), key, value)
}

pub fn delete_env(key: &str) -> std::io::Result<bool> {
    AgentConfig::delete_env_var(&home_dir(), key)
}
