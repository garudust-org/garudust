use std::sync::Arc;

use garudust_core::{
    error::AgentError,
    transport::ProviderTransport,
    types::{ContentPart, InferenceConfig, Message, Role, TokenUsage},
};
use tracing::info;

/// Compress old conversation turns when approaching context limit.
///
/// Strategy (mirrors Hermes):
/// 1. Keep the system prompt and last N turns intact (tail)
/// 2. Summarize everything in the middle via a separate LLM call
/// 3. Replace the middle with a single assistant message containing the summary
pub struct ContextCompressor {
    transport: Arc<dyn ProviderTransport>,
    model: String,
    threshold_fraction: f32,
    context_limit: usize,
    tail_turns: usize,
}

impl ContextCompressor {
    pub fn new(transport: Arc<dyn ProviderTransport>, model: String) -> Self {
        Self {
            transport,
            model,
            threshold_fraction: 0.80,
            context_limit: 128_000,
            tail_turns: 6,
        }
    }

    pub fn with_context_limit(mut self, limit: usize) -> Self {
        self.context_limit = limit;
        self
    }

    fn estimate_tokens(messages: &[Message]) -> usize {
        messages
            .iter()
            .map(|m| {
                m.content
                    .iter()
                    .map(|p| match p {
                        ContentPart::Text(t) => t.len() / 3,
                        ContentPart::ToolResult { content, .. } => content.len() / 3,
                        _ => 50,
                    })
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn should_compress(&self, messages: &[Message]) -> bool {
        let estimated = Self::estimate_tokens(messages);
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let threshold = (self.context_limit as f32 * self.threshold_fraction) as usize;
        estimated > threshold
    }

    pub async fn compress(
        &self,
        messages: Vec<Message>,
    ) -> Result<(Vec<Message>, TokenUsage), AgentError> {
        // Separate system prompt from conversation.
        let (system_msgs, conv_msgs): (Vec<_>, Vec<_>) =
            messages.into_iter().partition(|m| m.role == Role::System);

        // Need more than head (1) + tail (tail_turns*2) messages to have a middle to compress.
        if conv_msgs.len() <= 1 + self.tail_turns * 2 {
            let all: Vec<_> = system_msgs.into_iter().chain(conv_msgs).collect();
            return Ok((all, TokenUsage::default()));
        }

        // Three-region split (mirrors Hermes selective compression):
        //   head   — first message (original task) — always preserved
        //   middle — turns between head and tail — summarized via LLM
        //   tail   — last tail_turns*2 messages — always preserved
        let (head, rest) = conv_msgs.split_at(1);
        let split = rest.len().saturating_sub(self.tail_turns * 2);
        let (to_compress, tail) = rest.split_at(split);

        info!(
            head = head.len(),
            middle = to_compress.len(),
            tail = tail.len(),
            "compressing context"
        );

        let (summary_text, usage) = self.summarize(to_compress).await?;

        let summary_msg = Message {
            role: Role::Assistant,
            content: vec![ContentPart::Text(format!(
                "[Context summary — earlier conversation compressed]\n\n{summary_text}"
            ))],
        };

        let mut result = system_msgs;
        result.extend_from_slice(head);
        result.push(summary_msg);
        result.extend_from_slice(tail);

        Ok((result, usage))
    }

    async fn summarize(&self, turns: &[Message]) -> Result<(String, TokenUsage), AgentError> {
        let serialized: Vec<String> = turns
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                    Role::Tool => "Tool",
                    Role::System => "System",
                };
                let text = m
                    .content
                    .iter()
                    .find_map(|p| {
                        if let ContentPart::Text(t) = p {
                            Some(t.as_str())
                        } else {
                            None
                        }
                    })
                    .unwrap_or("[tool call/result]");
                format!("{role}: {text}")
            })
            .collect();

        let prompt = format!(
            "Summarize the following conversation turns concisely. \
             Preserve key facts, decisions, tool results, and any important context \
             that the agent may need to continue the task.\n\n{}",
            serialized.join("\n\n")
        );

        let config = InferenceConfig {
            model: self.model.clone(),
            max_tokens: Some(2048),
            context_limit: None,
            temperature: Some(0.0),
            reasoning_effort: None,
        };

        let resp = self
            .transport
            .chat(&[Message::user(prompt)], &config, &[])
            .await
            .map_err(AgentError::Transport)?;

        let summary = resp
            .content
            .iter()
            .find_map(|p| {
                if let ContentPart::Text(t) = p {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();

        Ok((summary, resp.usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use garudust_core::{
        error::TransportError,
        transport::{ApiMode, ProviderTransport, StreamResult},
        types::{ContentPart, InferenceConfig, Message, Role, ToolSchema, TransportResponse},
    };

    struct NullTransport;

    #[async_trait]
    impl ProviderTransport for NullTransport {
        fn api_mode(&self) -> ApiMode {
            ApiMode::ChatCompletions
        }
        async fn chat(
            &self,
            _messages: &[Message],
            _config: &InferenceConfig,
            _tools: &[ToolSchema],
        ) -> Result<TransportResponse, TransportError> {
            unimplemented!()
        }
        async fn chat_stream(
            &self,
            _messages: &[Message],
            _config: &InferenceConfig,
            _tools: &[ToolSchema],
        ) -> Result<StreamResult, TransportError> {
            unimplemented!()
        }
    }

    fn compressor(context_limit: usize) -> ContextCompressor {
        ContextCompressor::new(Arc::new(NullTransport), "null".into())
            .with_context_limit(context_limit)
    }

    fn msg(text: &str) -> Message {
        Message {
            role: Role::User,
            content: vec![ContentPart::Text(text.to_string())],
        }
    }

    // ── should_compress ───────────────────────────────────────────────────────

    #[test]
    fn should_compress_empty_messages() {
        // 0 estimated tokens — never triggers compression
        assert!(!compressor(1_000).should_compress(&[]));
    }

    #[test]
    fn should_compress_small_history() {
        // 300 chars ÷ 3 ≈ 100 tokens; threshold = 1000 × 0.80 = 800 → no compress
        let msgs = vec![msg(&"x".repeat(300))];
        assert!(!compressor(1_000).should_compress(&msgs));
    }

    #[test]
    fn should_compress_large_history() {
        // 3000 chars ÷ 3 = 1000 tokens; threshold = 1000 × 0.80 = 800 → compress
        let msgs = vec![msg(&"x".repeat(3_000))];
        assert!(compressor(1_000).should_compress(&msgs));
    }

    #[test]
    fn should_compress_exactly_at_threshold_does_not_trigger() {
        // 2400 chars ÷ 3 = 800 tokens == threshold (not strictly >) → no compress
        let msgs = vec![msg(&"x".repeat(2_400))];
        assert!(!compressor(1_000).should_compress(&msgs));
    }

    #[test]
    fn should_compress_one_over_threshold_triggers() {
        // 2403 chars ÷ 3 = 801 tokens > 800 → compress
        let msgs = vec![msg(&"x".repeat(2_403))];
        assert!(compressor(1_000).should_compress(&msgs));
    }
}
