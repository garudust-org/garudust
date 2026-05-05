//! Built-in tool suite for Garudust agents.
//!
//! Every tool implements [`garudust_core::tool::Tool`] and is registered in a
//! [`ToolRegistry`].  The agent calls tools by name; the registry validates
//! arguments against a JSON Schema, checks permissions, runs the approval gate
//! for destructive operations, and dispatches to the implementation.
//!
//! # Available toolsets
//!
//! | Toolset | Tools |
//! |---|---|
//! | `files` | `read_file`, `write_file`, `edit_file`, `list_dir`, `delete_file`, `move_file` |
//! | `terminal` | `run_command` — shell execution with timeout and approval gate |
//! | `web` | `web_fetch`, `web_search` — HTTP fetch and DuckDuckGo search |
//! | `browser` | `browser_*` — Chrome/Chromium CDP automation |
//! | `memory` | `read_memory`, `save_memory`, `read_user_profile`, `save_user_profile` |
//! | `skills` | `skills_list`, `skill_view`, `write_skill` — reusable instruction sets |
//! | `mcp` | Dynamically proxied tools from connected MCP servers |
//! | `pdf` | `read_pdf` — extract text from PDF files |
//! | `search` | `search_files`, `search_code` — glob and content search |
//! | `delegate` | `delegate_task` — spawn a sub-agent for a sub-task |

pub mod registry;
pub mod security;
pub mod toolsets;

pub use registry::ToolRegistry;
