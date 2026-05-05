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
//! use std::path::PathBuf;
//! use garudust_memory::FileMemoryStore;
//! use garudust_core::memory::MemoryStore;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let home  = PathBuf::from(std::env::var("HOME").unwrap_or_default());
//!     let store = FileMemoryStore::new(&home);
//!     let profile = store.read_user_profile().await?;
//!     println!("Profile: {profile}");
//!     Ok(())
//! }
//! ```

pub mod file_store;
pub mod migrations;
pub mod session_db;

pub use file_store::FileMemoryStore;
pub use session_db::SessionDb;
