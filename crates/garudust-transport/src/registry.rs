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
        "together" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.together.xyz/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "fireworks" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.fireworks.ai/inference/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "cerebras" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.cerebras.ai/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "perplexity" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.perplexity.ai".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "cohere" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.cohere.com/compatibility/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "nvidia" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://integrate.api.nvidia.com/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "alibaba" => Arc::new(
            ChatCompletionsTransport::new(
                base_url
                    .unwrap_or_else(|| "https://dashscope.aliyuncs.com/compatible-mode/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "doubao" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://ark.cn-beijing.volces.com/api/v3".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "zhipu" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://open.bigmodel.cn/api/paas/v4".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "moonshot" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://api.moonshot.cn/v1".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        "baidu" => Arc::new(
            ChatCompletionsTransport::new(
                base_url.unwrap_or_else(|| "https://qianfan.baidubce.com/v2".into()),
                api_key,
            )
            .with_tokens_param("max_tokens"),
        ),
        _ => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
            api_key,
        )),
    }
}

const KNOWN_PROVIDERS: &[&str] = &[
    "anthropic",
    "openai",
    "gemini",
    "groq",
    "mistral",
    "deepseek",
    "xai",
    "openrouter",
    "vllm",
    "thaillm",
    "ollama",
    "bedrock",
    "codex",
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
];

fn api_key_for_provider(provider: &str) -> String {
    let var = match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "xai" => "XAI_API_KEY",
        "thaillm" => "THAILLM_API_KEY",
        "vllm" => "VLLM_API_KEY",
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
        _ => "OPENROUTER_API_KEY",
    };
    garudust_core::config::get_secret(var).unwrap_or_default()
}

/// Resolve a routing target string into a `(transport, model)` pair.
///
/// Format: `"<provider>/<model>"` — the part before the first `/` selects the
/// provider and its endpoint; everything after is the model name sent to the API.
///
/// Returns `None` when the prefix is not a recognized provider (caller should
/// use the default transport and treat the whole string as the model name).
pub fn resolve_hint(target: &str) -> Option<(Arc<dyn ProviderTransport>, String)> {
    let (provider, model) = target.split_once('/')?;
    if !KNOWN_PROVIDERS.contains(&provider) {
        return None;
    }
    let api_key = api_key_for_provider(provider);
    let transport = build_base_transport(provider, None, api_key);
    Some((transport, model.to_string()))
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
