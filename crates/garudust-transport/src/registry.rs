use std::sync::Arc;

use garudust_core::config::AgentConfig;
use garudust_core::transport::ProviderTransport;

use crate::anthropic::AnthropicTransport;
use crate::bedrock::BedrockTransport;
use crate::chat_completions::ChatCompletionsTransport;
use crate::codex::CodexTransport;

pub fn build_transport(config: &AgentConfig) -> Arc<dyn ProviderTransport> {
    let base_url = config.base_url.clone();
    let api_key = config.api_key.clone().unwrap_or_default();

    match config.provider.as_str() {
        "anthropic" => Arc::new(AnthropicTransport::new(api_key)),
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
        _ => Arc::new(ChatCompletionsTransport::new(
            base_url.unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
            api_key,
        )),
    }
}
