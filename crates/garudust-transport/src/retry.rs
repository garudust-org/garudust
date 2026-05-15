use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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

/// Rotates through a list of transports on `Auth` failures.
///
/// Each `Auth` error advances the active index by one. Only after all
/// candidates are exhausted does the error propagate to the caller.
pub struct CredentialRotationTransport {
    candidates: Vec<Arc<dyn ProviderTransport>>,
    index: AtomicUsize,
}

impl CredentialRotationTransport {
    pub fn new(candidates: Vec<Arc<dyn ProviderTransport>>) -> Self {
        assert!(!candidates.is_empty(), "at least one transport required");
        Self {
            candidates,
            index: AtomicUsize::new(0),
        }
    }

    fn current(&self) -> &Arc<dyn ProviderTransport> {
        let i = self
            .index
            .load(Ordering::SeqCst)
            .min(self.candidates.len() - 1);
        &self.candidates[i]
    }

    fn rotate(&self) -> bool {
        let prev = self.index.fetch_add(1, Ordering::SeqCst);
        prev + 1 < self.candidates.len()
    }
}

#[async_trait]
impl ProviderTransport for CredentialRotationTransport {
    fn api_mode(&self) -> ApiMode {
        self.candidates[0].api_mode()
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        loop {
            match self.current().chat(messages, config, tools).await {
                Err(TransportError::Auth) if self.rotate() => {
                    tracing::warn!("auth failure, rotating to next API key");
                }
                other => return other,
            }
        }
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        loop {
            match self.current().chat_stream(messages, config, tools).await {
                Err(TransportError::Auth) if self.rotate() => {
                    tracing::warn!("auth failure on stream, rotating to next API key");
                }
                other => return other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures::StreamExt as _;
    use garudust_core::{
        error::TransportError,
        transport::{ApiMode, ProviderTransport, StreamResult},
        types::{
            ContentPart, InferenceConfig, Message, StopReason, TokenUsage, ToolSchema,
            TransportResponse,
        },
    };

    use super::{CredentialRotationTransport, RetryTransport};

    fn dummy_config() -> InferenceConfig {
        InferenceConfig {
            model: "test".into(),
            max_tokens: None,
            context_limit: None,
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
            let n = self.calls.fetch_add(1, Ordering::SeqCst);
            if n < self.fail_times {
                Err(TransportError::Http {
                    status: 503,
                    body: "unavailable".into(),
                })
            } else {
                Ok(Box::pin(futures::stream::empty()))
            }
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

    #[tokio::test]
    async fn stream_retries_on_503_then_succeeds() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Arc::new(CountingTransport {
            calls: calls.clone(),
            fail_times: 1,
        });
        let retry = RetryTransport::new(inner, 3, 0);
        let result = retry.chat_stream(&[], &dummy_config(), &[]).await;
        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 2); // 1 fail + 1 success
    }

    #[tokio::test]
    async fn stream_fails_after_max_retries() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Arc::new(CountingTransport {
            calls: calls.clone(),
            fail_times: 10,
        });
        let retry = RetryTransport::new(inner, 2, 0);
        let result = retry.chat_stream(&[], &dummy_config(), &[]).await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn stream_returns_empty_on_success() {
        let calls = Arc::new(AtomicU32::new(0));
        let inner = Arc::new(CountingTransport {
            calls: calls.clone(),
            fail_times: 0,
        });
        let retry = RetryTransport::new(inner, 1, 0);
        let mut stream = retry.chat_stream(&[], &dummy_config(), &[]).await.unwrap();
        // CountingTransport returns an empty stream on success
        assert!(stream.next().await.is_none());
    }

    struct AuthFailTransport {
        fail_auth: bool,
    }

    #[async_trait]
    impl ProviderTransport for AuthFailTransport {
        fn api_mode(&self) -> ApiMode {
            ApiMode::ChatCompletions
        }
        async fn chat(
            &self,
            _messages: &[Message],
            _config: &InferenceConfig,
            _tools: &[ToolSchema],
        ) -> Result<TransportResponse, TransportError> {
            if self.fail_auth {
                Err(TransportError::Auth)
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
            if self.fail_auth {
                Err(TransportError::Auth)
            } else {
                Ok(Box::pin(futures::stream::empty()))
            }
        }
    }

    #[tokio::test]
    async fn rotation_skips_bad_key_and_succeeds() {
        let bad = Arc::new(AuthFailTransport { fail_auth: true });
        let good = Arc::new(AuthFailTransport { fail_auth: false });
        let rot = CredentialRotationTransport::new(vec![bad, good]);
        let result = rot.chat(&[], &dummy_config(), &[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn rotation_fails_when_all_keys_exhausted() {
        let bad1 = Arc::new(AuthFailTransport { fail_auth: true });
        let bad2 = Arc::new(AuthFailTransport { fail_auth: true });
        let rot = CredentialRotationTransport::new(vec![bad1, bad2]);
        let result = rot.chat(&[], &dummy_config(), &[]).await;
        assert!(matches!(result, Err(TransportError::Auth)));
    }
}
