//! AI agent run-loop, prompt builder, and multi-agent orchestration for Garudust.
//!
//! The centrepiece of this crate is [`Agent`], which drives the
//! **think → tool-call → observe** loop until the model signals it is done.
//!
//! # Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use garudust_agent::Agent;
//! use garudust_core::config::AgentConfig;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Arc::new(AgentConfig::default());
//!     let agent  = Agent::new(config);
//!     let result = agent.run_once("List files in the current directory").await?;
//!     println!("{result}");
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────┐
//! │                  Agent                   │
//! │  build_system_prompt()                   │
//! │  ┌────────────────────────────────────┐  │
//! │  │  iteration loop                    │  │
//! │  │  1. call ProviderTransport (LLM)   │  │
//! │  │  2. dispatch tool calls            │  │
//! │  │  3. append results to history      │  │
//! │  │  4. repeat until stop_reason=end   │  │
//! │  └────────────────────────────────────┘  │
//! │  persist_session() → SessionDb           │
//! └──────────────────────────────────────────┘
//! ```
//!
//! # Skills and self-improvement
//!
//! When the agent finishes a task it may call `write_skill` to save a reusable
//! instruction set to `~/.garudust/skills/<name>/SKILL.md`.  On subsequent
//! runs the skill index is injected into the system prompt so the model can
//! load and apply the skill via `skill_view`.

pub mod agent;
pub mod approver;
pub mod compressor;
pub mod prompt_builder;
mod tests;

pub use agent::Agent;
pub use approver::{AutoApprover, ConstitutionalApprover, DenyApprover};
