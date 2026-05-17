use std::collections::HashMap;
use std::sync::Arc;

use garudust_core::config::ProviderProfile;
use garudust_core::config::{AgentConfig, BuiltinProvider, BUILTIN_PROVIDERS};
use garudust_core::transport::ProviderTransport;

use crate::anthropic::AnthropicTransport;
use crate::bedrock::BedrockTransport;
use crate::chat_completions::ChatCompletionsTransport;
use crate::codex::CodexTransport;
use crate::ollama;
use crate::retry::{CredentialRotationTransport, RetryTransport};

// ── Transport construction ────────────────────────────────────────────────────

/// Build a transport for a named builtin provider (anthropic, bedrock, ollama,
/// codex) or any OpenAI-compatible provider listed in BUILTIN_PROVIDERS.
fn build_base_transport(
    provider: &str,
    base_url: Option<String>,
    api_key: String,
) -> Arc<dyn ProviderTransport> {
    match provider {
        "anthropic" => match base_url {
            Some(url) => Arc::new(ChatCompletionsTransport::new(url, api_key)),
            None => Arc::new(AnthropicTransport::new(api_key)),
        },
        "codex" => {
            let mut t = CodexTransport::new(api_key);
            if let Some(url) = base_url {
                t = t.with_base_url(url);
            }
            Arc::new(t)
        }
        "bedrock" => match BedrockTransport::from_env() {
            Ok(t) => Arc::new(t),
            Err(e) => {
                tracing::warn!(
                    "bedrock transport init failed: {e}; falling back to chat-completions"
                );
                Arc::new(ChatCompletionsTransport::new(
                    base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
                    api_key,
                ))
            }
        },
        "ollama" => Arc::new(ollama::new(
            base_url.unwrap_or_else(|| ollama::DEFAULT_BASE_URL.into()),
        )),
        _ => {
            let builtin = BUILTIN_PROVIDERS.iter().find(|p| p.name == provider);
            let url = base_url.unwrap_or_else(|| {
                builtin
                    .map(|p| p.base_url.to_string())
                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".into())
            });
            let tokens_param = builtin.map_or("max_completion_tokens", |p| p.tokens_param);
            Arc::new(ChatCompletionsTransport::new(url, api_key).with_tokens_param(tokens_param))
        }
    }
}

/// Build a transport from a user-defined [`ProviderProfile`].
fn build_from_profile(profile: &ProviderProfile) -> Arc<dyn ProviderTransport> {
    let key = profile.resolved_key().unwrap_or_default();
    let provider_name = profile.name.as_deref().unwrap_or("");

    // Special transports only apply when no custom URL is given
    if profile.url.is_none() {
        match provider_name {
            "anthropic" => return Arc::new(AnthropicTransport::new(key)),
            "bedrock" => {
                return match BedrockTransport::from_env() {
                    Ok(t) => Arc::new(t),
                    Err(e) => {
                        tracing::warn!("bedrock init failed: {e}; falling back");
                        Arc::new(ChatCompletionsTransport::new(
                            "https://openrouter.ai/api/v1".into(),
                            key,
                        ))
                    }
                };
            }
            "ollama" => {
                return Arc::new(ollama::new(ollama::DEFAULT_BASE_URL.into()));
            }
            _ => {}
        }
    }

    let builtin: Option<&BuiltinProvider> =
        BUILTIN_PROVIDERS.iter().find(|p| p.name == provider_name);
    let url = profile.url.clone().unwrap_or_else(|| {
        builtin
            .map(|p| p.base_url.to_string())
            .unwrap_or_else(|| "https://openrouter.ai/api/v1".into())
    });
    let tokens_param = builtin.map_or("max_completion_tokens", |p| p.tokens_param);
    Arc::new(ChatCompletionsTransport::new(url, key).with_tokens_param(tokens_param))
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Resolve a routing target string into a `(transport, model)` pair.
///
/// Format: `"<profile-or-provider>/<model>"`.
///
/// Resolution order:
/// 1. Named user profile from `providers` map.
/// 2. Built-in provider name (anthropic, groq, openai, …).
///
/// Returns `None` when the prefix is not a known profile or provider.
pub fn resolve_hint(
    target: &str,
    profiles: &HashMap<String, ProviderProfile>,
) -> Option<(Arc<dyn ProviderTransport>, String)> {
    let (prefix, model) = target.split_once('/')?;

    // 1. User-defined profile takes priority
    if let Some(profile) = profiles.get(prefix) {
        return Some((build_from_profile(profile), model.to_string()));
    }

    // 2. Built-in provider
    let is_special = matches!(prefix, "anthropic" | "bedrock" | "ollama" | "codex");
    let is_builtin = is_special || BUILTIN_PROVIDERS.iter().any(|p| p.name == prefix);
    if !is_builtin {
        return None;
    }

    let api_key = if prefix == "anthropic" {
        garudust_core::config::get_secret("ANTHROPIC_API_KEY").unwrap_or_default()
    } else {
        BUILTIN_PROVIDERS
            .iter()
            .find(|p| p.name == prefix)
            .and_then(|p| garudust_core::config::get_secret(p.api_key_env))
            .unwrap_or_default()
    };
    Some((
        build_base_transport(prefix, None, api_key),
        model.to_string(),
    ))
}

/// For script tools: resolve a `"profile/model"` or `"provider/model"` string
/// to `(base_url, api_key, model)` so the subprocess can be given explicit env vars.
///
/// Returns `None` when the prefix is unrecognised.
pub fn resolve_to_env_vars(
    target: &str,
    profiles: &HashMap<String, ProviderProfile>,
) -> Option<(String, String, String)> {
    let (prefix, model) = target.split_once('/')?;

    if let Some(profile) = profiles.get(prefix) {
        let key = profile.resolved_key().unwrap_or_default();
        let provider_name = profile.name.as_deref().unwrap_or("");
        let builtin = BUILTIN_PROVIDERS.iter().find(|p| p.name == provider_name);
        let url = profile
            .url
            .clone()
            .or_else(|| builtin.map(|p| p.base_url.to_string()))?;
        return Some((url, key, model.to_string()));
    }

    if let Some(builtin) = BUILTIN_PROVIDERS.iter().find(|p| p.name == prefix) {
        let key = garudust_core::config::get_secret(builtin.api_key_env).unwrap_or_default();
        return Some((builtin.base_url.to_string(), key, model.to_string()));
    }

    None
}

/// Build the main agent transport from [`AgentConfig`].
///
/// If `providers.default` exists, it is used as the primary transport.
/// Otherwise falls back to the legacy `provider` / `base_url` / `api_key` fields.
pub fn build_transport(config: &AgentConfig) -> Arc<dyn ProviderTransport> {
    let base: Arc<dyn ProviderTransport> =
        if let Some(default_profile) = config.providers.get("default") {
            // New path: build from named profile
            let primary = build_from_profile(default_profile);
            if config.fallback_api_keys.is_empty() {
                primary
            } else {
                let mut candidates: Vec<Arc<dyn ProviderTransport>> =
                    Vec::with_capacity(1 + config.fallback_api_keys.len());
                candidates.push(primary);
                for key in &config.fallback_api_keys {
                    let mut fallback_profile = default_profile.clone();
                    fallback_profile.key = Some(key.clone());
                    candidates.push(build_from_profile(&fallback_profile));
                }
                Arc::new(CredentialRotationTransport::new(candidates))
            }
        } else {
            // Legacy path: use config.provider + base_url + api_key
            let base_url = config.base_url.clone();
            let primary_key = config.api_key.clone().unwrap_or_default();

            if config.fallback_api_keys.is_empty() {
                build_base_transport(&config.provider, base_url, primary_key)
            } else {
                let mut candidates: Vec<Arc<dyn ProviderTransport>> =
                    Vec::with_capacity(1 + config.fallback_api_keys.len());
                candidates.push(build_base_transport(
                    &config.provider,
                    base_url.clone(),
                    primary_key,
                ));
                for key in &config.fallback_api_keys {
                    candidates.push(build_base_transport(
                        &config.provider,
                        base_url.clone(),
                        key.clone(),
                    ));
                }
                Arc::new(CredentialRotationTransport::new(candidates))
            }
        };

    if config.llm_max_retries > 0 {
        Arc::new(RetryTransport::new(
            base,
            config.llm_max_retries,
            config.llm_retry_base_ms,
        ))
    } else {
        base
    }
}
