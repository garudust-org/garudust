//! Core traits, types, and error definitions for the Garudust AI agent framework.
//!
//! This crate is the foundation that all other `garudust-*` crates build on.
//! It defines the shared interfaces — tools, transports, memory stores, and
//! platform adapters — so that every layer of the stack can be swapped or
//! extended without touching unrelated code.
//!
//! # Key abstractions
//!
//! | Trait | Purpose |
//! |---|---|
//! | [`tool::Tool`] | A single callable capability the agent can invoke |
//! | [`transport::ProviderTransport`] | LLM backend (Anthropic, Ollama, OpenRouter …) |
//! | [`memory::MemoryStore`] | Persistent facts and user profile storage |
//! | [`platform::PlatformAdapter`] | Chat platform (Telegram, Discord, Slack …) |
//!
//! # Feature flags
//!
//! This crate has no optional features; everything here is always available.

pub mod budget;
pub mod config;
pub mod error;
pub mod memory;
pub mod net_guard;
pub mod platform;
pub mod tool;
pub mod transport;
pub mod types;

pub use error::{AgentError, PlatformError, ToolError, TransportError};
pub use types::*;
