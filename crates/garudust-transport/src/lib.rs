//! LLM provider transport layer for Garudust agents.
//!
//! Implements [`garudust_core::transport::ProviderTransport`] for every
//! supported backend.  All providers expose a unified streaming interface so
//! the agent run-loop is completely decoupled from the underlying API.
//!
//! # Supported providers
//!
//! | Module | Provider | Notes |
//! |---|---|---|
//! | [`anthropic`] | Anthropic Claude | Default; streaming + tool-use |
//! | [`ollama`] | Ollama | Local models via HTTP |
//! | [`chat_completions`] | OpenAI-compatible | OpenRouter, vLLM, LM Studio … |
//! | [`bedrock`] | AWS Bedrock | Claude via AWS credentials |
//! | [`codex`] | OpenAI Codex | Legacy completions endpoint |
//!
//! # Selecting a transport
//!
//! Use [`build_transport`] which reads `ANTHROPIC_API_KEY`, `OPENROUTER_API_KEY`,
//! or `OLLAMA_HOST` from the environment and returns the appropriate transport
//! wrapped in a [`RetryTransport`] with exponential back-off.
//!
//! ```no_run
//! use garudust_transport::build_transport;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let transport = build_transport()?;
//!     Ok(())
//! }
//! ```

pub mod anthropic;
pub mod bedrock;
pub mod chat_completions;
pub mod codex;
pub mod ollama;
pub mod registry;
pub mod retry;

pub use registry::build_transport;
pub use retry::RetryTransport;
