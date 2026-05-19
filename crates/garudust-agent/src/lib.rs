//! AI agent run-loop, prompt builder, and multi-agent orchestration for Garudust.
//!
//! The centrepiece of this crate is [`Agent`], which drives the
//! **think → tool-call → observe** loop until the model signals it is done.
//!
//! # Quick start
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::path::PathBuf;
//! use garudust_agent::{Agent, AutoApprover};
//! use garudust_core::config::AgentConfig;
//! use garudust_transport::build_transport;
//! use garudust_memory::FileMemoryStore;
//! use garudust_tools::ToolRegistry;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config    = Arc::new(AgentConfig::default());
//!     let transport = build_transport(&config);
//!     let tools     = Arc::new(ToolRegistry::default());
//!     let memory    = Arc::new(FileMemoryStore::new(&PathBuf::from("/tmp")));
//!     let agent     = Agent::new(transport, tools, memory, config);
//!     let approver  = Arc::new(AutoApprover);
//!     let result    = agent.run("List files here", approver, "cli", None, None).await?;
//!     println!("{}", result.output);
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
pub use approver::{AutoApprover, ConstitutionalApprover, DenyApprover, RolesApprover};
