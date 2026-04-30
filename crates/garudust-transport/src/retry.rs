use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    error::TransportError,
    transport::{ApiMode, ProviderTransport, StreamResult},
    types::{InferenceConfig, Message, ToolSchema, TransportResponse},
};

pub struct RetryTransport {
    inner: Arc<dyn ProviderTransport>,
    max_retries: u32,
    base_ms: u64,
}

impl RetryTransport {
    pub fn new(inner: Arc<dyn ProviderTransport>, max_retries: u32, base_ms: u64) -> Self {
        Self {
            inner,
            max_retries,
            base_ms,
        }
    }
}

fn is_retryable(err: &TransportError) -> bool {
    match err {
        TransportError::Http { status, .. } => matches!(status, 429 | 500 | 502 | 503 | 504),
        TransportError::RateLimit { .. } | TransportError::Network(_) => true,
        _ => false,
    }
}

fn delay_ms(err: &TransportError, attempt: u32, base_ms: u64) -> u64 {
    if let TransportError::RateLimit { retry_after_secs } = err {
        const MAX_RATE_LIMIT_SECS: u64 = 300;
        if *retry_after_secs > MAX_RATE_LIMIT_SECS {
            tracing::warn!(
                requested = retry_after_secs,
                capped = MAX_RATE_LIMIT_SECS,
                "Retry-After exceeds cap, clamping"
            );
        }
        return retry_after_secs
            .min(&MAX_RATE_LIMIT_SECS)
            .saturating_mul(1000);
    }
    let exp = base_ms.saturating_mul(1u64 << attempt.min(6));
    // cheap time-based jitter without external deps
    let jitter = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis(),
    ) % (exp / 4 + 1);
    exp + jitter
}

#[async_trait]
impl ProviderTransport for RetryTransport {
    fn api_mode(&self) -> ApiMode {
        self.inner.api_mode()
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        let mut attempt = 0u32;
        loop {
            match self.inner.chat(messages, config, tools).await {
                Ok(r) => return Ok(r),
                Err(e) if is_retryable(&e) && attempt < self.max_retries => {
                    let delay = delay_ms(&e, attempt, self.base_ms);
                    tracing::warn!(
                        attempt = attempt + 1,
                        max = self.max_retries,
                        delay_ms = delay,
                        error = %e,
                        "transient LLM error, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    // Streams can't be rewound after partial delivery, so we only retry the
    // initial connection — not mid-stream failures.
    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        let mut attempt = 0u32;
        loop {
            match self.inner.chat_stream(messages, config, tools).await {
                Ok(s) => return Ok(s),
                Err(e) if is_retryable(&e) && attempt < self.max_retries => {
                    let delay = delay_ms(&e, attempt, self.base_ms);
                    tracing::warn!(
                        attempt = attempt + 1,
                        max = self.max_retries,
                        delay_ms = delay,
                        error = %e,
                        "transient LLM stream error, retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        error::TransportError,
        transport::{ApiMode, ProviderTransport, StreamResult},
        types::{
            ContentPart, InferenceConfig, Message, StopReason, TokenUsage, ToolSchema,
            TransportResponse,
        },
    };

    use super::RetryTransport;

    fn dummy_config() -> InferenceConfig {
        InferenceConfig {
            model: "test".into(),
            max_tokens: None,
            temperature: None,
            reasoning_effort: None,
        }
    }

    fn ok_response() -> TransportResponse {
        TransportResponse {
            content: vec![ContentPart::Text("ok".into())],
            tool_calls: vec![],
            usage: TokenUsage::default(),
            stop_reason: StopReason::EndTurn,
        }
    }

    struct CountingTransport {
        calls: Arc<AtomicU32>,
        fail_times: u32,
    }

    #[async_trait]
    impl ProviderTransport for CountingTransport {
        fn api_mode(&self) -> ApiMode {
            ApiMode::ChatCompletions
        }
        async fn chat(
            &self,
            _messages: &[Message],
            _config: &InferenceConfig,
            _tools: &[ToolSchema],
        ) -> Result<TransportResponse, TransportError> {
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fail_times {
                Err(TransportError::Http {
                    status: 503,
                    body: "unavailable".into(),
                })
            } else {
                Ok(ok_response())
            }
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

    #[tokio::test]
    async fn retries_on_503_then_succeeds() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Arc::new(CountingTransport {
            calls: calls.clone(),
            fail_times: 2,
        });
        let retry = RetryTransport::new(inner, 3, 0);
        let result = retry.chat(&[], &dummy_config(), &[]).await;
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn fails_after_max_retries() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Arc::new(CountingTransport {
            calls: calls.clone(),
            fail_times: 10,
        });
        let retry = RetryTransport::new(inner, 2, 0);
        let result = retry.chat(&[], &dummy_config(), &[]).await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3); // initial + 2 retries
    }
}
