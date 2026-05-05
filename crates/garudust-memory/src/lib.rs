//! SQLite-backed persistent memory and user profiles for Garudust AI agents.
//!
//! Provides two storage layers:
//!
//! * **[`FileMemoryStore`]** — Markdown files under `~/.garudust/` for long-term
//!   facts and user profile. Readable and editable by humans.
//! * **[`SessionDb`]** — SQLite database for conversation history, tool call
//!   logs, and session metadata.
//!
//! # Example
//!
//! ```no_run
//! use garudust_memory::FileMemoryStore;
//! use garudust_core::memory::MemoryStore;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let store  = FileMemoryStore::new(dirs::home_dir().unwrap().join(".garudust"));
//!     let memory = store.read_memory().await?;
//!     println!("Facts: {:?}", memory.facts);
//!     Ok(())
//! }
//! ```

pub mod file_store;
pub mod migrations;
pub mod session_db;

pub use file_store::FileMemoryStore;
pub use session_db::SessionDb;
