use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("budget exhausted after {0} iterations")]
    BudgetExhausted(u32),
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),
    #[error("context compression failed: {0}")]
    Compression(String),
    #[error("interrupted")]
    Interrupted,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("HTTP error {status}: {body}")]
    Http { status: u16, body: String },
    #[error("rate limited, retry after {retry_after_secs}s")]
    RateLimit { retry_after_secs: u64 },
    #[error("network error: {0}")]
    Network(String),
    #[error("authentication failed")]
    Auth,
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("context length exceeded")]
    ContextLengthExceeded,
    #[error("stream error: {0}")]
    Stream(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("approval denied")]
    ApprovalDenied,
    #[error("tool timed out after {0}s")]
    Timeout(u64),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("connection error: {0}")]
    Connection(String),
    #[error("send failed: {0}")]
    Send(String),
    #[error("authentication failed")]
    Auth,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
