use std::sync::Arc;

use garudust_core::config::AgentConfig;
use garudust_core::transport::ProviderTransport;

use crate::anthropic::AnthropicTransport;
use crate::bedrock::BedrockTransport;
use crate::chat_completions::ChatCompletionsTransport;
use crate::codex::CodexTransport;
use crate::ollama;
use crate::retry::{CredentialRotationTransport, RetryTransport};

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
        "openai" => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "https://api.openai.com/v1".into()),
            api_key,
        )),
        "gemini" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| {
                    "https://generativelanguage.googleapis.com/v1beta/openai".into()
                }),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "groq" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.groq.com/openai/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "mistral" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.mistral.ai/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "deepseek" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.deepseek.com/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "xai" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.x.ai/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "vllm" => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "http://localhost:8000/v1".into()),
            api_key,
        )),
        "thaillm" => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "http://thaillm.or.th/api/v1".into()),
            api_key,
        )),
        _ => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
            api_key,
        )),
    }
}

pub fn build_transport(config: &AgentConfig) -> Arc<dyn ProviderTransport> {
    let base_url = config.base_url.clone();
    let primary_key = config.api_key.clone().unwrap_or_default();

    let base: Arc<dyn ProviderTransport> = if config.fallback_api_keys.is_empty() {
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
