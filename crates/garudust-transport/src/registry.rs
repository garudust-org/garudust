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
                builtin.map_or_else(
                    || "https://openrouter.ai/api/v1".into(),
                    |p| p.base_url.to_string(),
                )
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
        builtin.map_or_else(
            || "https://openrouter.ai/api/v1".into(),
            |p| p.base_url.to_string(),
        )
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
pub fn resolve_hint<S: std::hash::BuildHasher>(
    target: &str,
    profiles: &HashMap<String, ProviderProfile, S>,
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
pub fn resolve_to_env_vars<S: std::hash::BuildHasher>(
    target: &str,
    profiles: &HashMap<String, ProviderProfile, S>,
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use garudust_core::config::ProviderProfile;

    use super::{resolve_hint, resolve_to_env_vars};

    fn profile(name: Option<&str>, url: Option<&str>, key: Option<&str>) -> ProviderProfile {
        ProviderProfile {
            name: name.map(str::to_string),
            url: url.map(str::to_string),
            key: key.map(str::to_string),
            model: None,
        }
    }

    fn profiles(pairs: &[(&str, ProviderProfile)]) -> HashMap<String, ProviderProfile> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect()
    }

    // ── resolve_hint ─────────────────────────────────────────────────────────

    #[test]
    fn hint_unknown_prefix_is_none() {
        assert!(resolve_hint("totally-unknown/some-model", &profiles(&[])).is_none());
    }

    #[test]
    fn hint_builtin_provider_returns_some() {
        let result = resolve_hint("groq/llama-3.1-8b-instant", &profiles(&[]));
        assert!(result.is_some());
        let (_, model) = result.unwrap();
        assert_eq!(model, "llama-3.1-8b-instant");
    }

    #[test]
    fn hint_user_profile_wins_over_builtin() {
        let map = profiles(&[("groq", profile(Some("openai"), None, Some("sk-custom")))]);
        let result = resolve_hint("groq/gpt-4o", &map);
        assert!(result.is_some());
        let (_, model) = result.unwrap();
        assert_eq!(model, "gpt-4o");
    }

    #[test]
    fn hint_user_profile_arbitrary_alias() {
        let map = profiles(&[(
            "my-local",
            profile(None, Some("http://localhost:8000/v1"), Some("token")),
        )]);
        let result = resolve_hint("my-local/llama3", &map);
        assert!(result.is_some());
        let (_, model) = result.unwrap();
        assert_eq!(model, "llama3");
    }

    #[test]
    fn hint_no_slash_returns_none() {
        assert!(resolve_hint("groq", &profiles(&[])).is_none());
    }

    #[test]
    fn hint_anthropic_builtin_returns_some() {
        let result = resolve_hint("anthropic/claude-sonnet-4-6", &profiles(&[]));
        assert!(result.is_some());
        let (_, model) = result.unwrap();
        assert_eq!(model, "claude-sonnet-4-6");
    }

    // ── resolve_to_env_vars ───────────────────────────────────────────────────

    #[test]
    fn env_vars_unknown_returns_none() {
        assert!(resolve_to_env_vars("totally-unknown/model", &profiles(&[])).is_none());
    }

    #[test]
    fn env_vars_no_slash_returns_none() {
        assert!(resolve_to_env_vars("groq", &profiles(&[])).is_none());
    }

    #[test]
    fn env_vars_builtin_provider_returns_base_url() {
        let (base_url, _key, model) =
            resolve_to_env_vars("groq/llama-3.1-8b-instant", &profiles(&[])).unwrap();
        assert_eq!(base_url, "https://api.groq.com/openai/v1");
        assert_eq!(model, "llama-3.1-8b-instant");
    }

    #[test]
    fn env_vars_profile_with_url() {
        let map = profiles(&[(
            "local",
            profile(None, Some("http://192.168.1.10:8000/v1"), Some("tok")),
        )]);
        let (base_url, key, model) = resolve_to_env_vars("local/llama3", &map).unwrap();
        assert_eq!(base_url, "http://192.168.1.10:8000/v1");
        assert_eq!(key, "tok");
        assert_eq!(model, "llama3");
    }

    #[test]
    fn env_vars_profile_with_builtin_name_uses_builtin_url() {
        let map = profiles(&[("backup", profile(Some("groq"), None, Some("gsk-2")))]);
        let (base_url, key, model) =
            resolve_to_env_vars("backup/llama-3.3-70b-versatile", &map).unwrap();
        assert_eq!(base_url, "https://api.groq.com/openai/v1");
        assert_eq!(key, "gsk-2");
        assert_eq!(model, "llama-3.3-70b-versatile");
    }

    #[test]
    fn env_vars_profile_no_url_no_name_returns_none() {
        // No url and no builtin name → can't determine base_url.
        let map = profiles(&[("mystery", profile(None, None, Some("tok")))]);
        assert!(resolve_to_env_vars("mystery/model", &map).is_none());
    }

    #[test]
    fn env_vars_profile_key_env_var_resolved() {
        std::env::set_var("GARUDUST_TEST_REGISTRY_KEY", "resolved-key");
        let map = profiles(&[(
            "p",
            profile(
                None,
                Some("http://localhost:8000/v1"),
                Some("${GARUDUST_TEST_REGISTRY_KEY}"),
            ),
        )]);
        let (_, key, _) = resolve_to_env_vars("p/model", &map).unwrap();
        assert_eq!(key, "resolved-key");
        std::env::remove_var("GARUDUST_TEST_REGISTRY_KEY");
    }

    // ── BUILTIN_PROVIDERS schema validation ───────────────────────────────────

    #[test]
    fn all_builtin_providers_have_unique_names() {
        let names: Vec<&str> = garudust_core::config::BUILTIN_PROVIDERS
            .iter()
            .map(|p| p.name)
            .collect();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(*name), "duplicate provider name: {name}");
        }
    }

    #[test]
    fn all_builtin_providers_have_non_empty_base_url() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            assert!(
                !p.base_url.is_empty(),
                "provider '{}' has empty base_url",
                p.name
            );
        }
    }

    #[test]
    fn all_builtin_providers_have_non_empty_api_key_env() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            assert!(
                !p.api_key_env.is_empty(),
                "provider '{}' has empty api_key_env",
                p.name
            );
        }
    }

    #[test]
    fn all_builtin_providers_tokens_param_is_known_value() {
        const VALID: &[&str] = &["max_tokens", "max_completion_tokens"];
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            assert!(
                VALID.contains(&p.tokens_param),
                "provider '{}' has unexpected tokens_param: '{}'",
                p.name,
                p.tokens_param
            );
        }
    }

    #[test]
    fn all_builtin_providers_base_url_has_valid_scheme() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            assert!(
                p.base_url.starts_with("https://") || p.base_url.starts_with("http://"),
                "provider '{}' base_url has unexpected scheme: '{}'",
                p.name,
                p.base_url
            );
        }
    }

    #[test]
    fn all_builtin_providers_api_key_env_is_screaming_snake_case() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            let valid = p
                .api_key_env
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            assert!(
                valid,
                "provider '{}' api_key_env '{}' is not SCREAMING_SNAKE_CASE",
                p.name, p.api_key_env
            );
        }
    }

    #[test]
    fn resolve_to_env_vars_returns_correct_base_url_for_every_builtin() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            let target = format!("{}/some-model", p.name);
            let result = resolve_to_env_vars(&target, &profiles(&[]));
            assert!(
                result.is_some(),
                "resolve_to_env_vars returned None for builtin provider '{}'",
                p.name
            );
            let (base_url, _key, model) = result.unwrap();
            assert_eq!(
                base_url, p.base_url,
                "provider '{}' resolved to wrong base_url",
                p.name
            );
            assert_eq!(
                model, "some-model",
                "provider '{}' model was mangled",
                p.name
            );
        }
    }

    #[test]
    fn resolve_hint_returns_transport_for_every_builtin() {
        for p in garudust_core::config::BUILTIN_PROVIDERS {
            let target = format!("{}/some-model", p.name);
            let result = resolve_hint(&target, &profiles(&[]));
            assert!(
                result.is_some(),
                "resolve_hint returned None for builtin provider '{}'",
                p.name
            );
            let (_, model) = result.unwrap();
            assert_eq!(model, "some-model");
        }
    }
}
