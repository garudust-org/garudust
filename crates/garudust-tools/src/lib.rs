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
//! | `rag` | `doc_ingest`, `doc_search`, `doc_list`, `doc_forget` — document RAG via FTS5 |
//! | `delegate` | `delegate_task` — spawn a sub-agent for a sub-task |
//! | `cron` | `cron_create`, `cron_list`, `cron_delete` — runtime cron job management |

pub mod hub;
pub mod registry;
pub mod security;
pub mod skill_hub;
pub mod toolsets;

pub use registry::ToolRegistry;
pub use toolsets::script::load_script_tools;

/// Register the full standard tool suite into `registry`.
///
/// `db` is `Some` when session-search history is available (always in the
/// server, optional in the CLI). Callers that don't need the session-search
/// tool pass `None`.
///
/// `cron` is `Some` when a runtime cron scheduler is available. The slot is
/// filled after the scheduler is started; tools check at call time.
pub fn register_standard_tools(
    registry: &mut ToolRegistry,
    db: Option<std::sync::Arc<garudust_memory::SessionDb>>,
    doc_store: Option<std::sync::Arc<garudust_memory::DocStore>>,
    cron: Option<
        std::sync::Arc<
            tokio::sync::Mutex<Option<std::sync::Arc<dyn garudust_core::cron::CronManager>>>,
        >,
    >,
) {
    use toolsets::{
        browser::BrowserTool,
        cron::{CronCreate, CronDelete, CronList},
        delegate::{DelegateTask, DelegateTasks},
        files::{ListDirectory, ReadFile, WriteFile},
        git::{GitDiff, GitLog, GitStatus},
        image::ImageRead,
        json_query::JsonQuery,
        memory::{MemoryTool, UserProfileTool},
        notes::{ListNotes, ReadNote, WriteNote},
        pdf::PdfRead,
        rag::{DocForget, DocIngest, DocList, DocSearch},
        search::SessionSearch,
        skills::{SkillView, SkillsList, WriteSkill},
        terminal::Terminal,
        web::{HttpRequest, WebFetch, WebSearch},
    };

    registry.register(WebFetch);
    registry.register(WebSearch);
    registry.register(HttpRequest);
    registry.register(ReadFile);
    registry.register(WriteFile);
    registry.register(ListDirectory);
    registry.register(PdfRead);
    registry.register(Terminal);
    registry.register(MemoryTool);
    registry.register(UserProfileTool);
    if let Some(db) = db {
        registry.register(SessionSearch::new(db));
    }
    if let Some(store) = doc_store {
        registry.register(DocIngest::new(store.clone()));
        registry.register(DocSearch::new(store.clone()));
        registry.register(DocList::new(store.clone()));
        registry.register(DocForget::new(store));
    }
    registry.register(SkillsList);
    registry.register(SkillView);
    registry.register(WriteSkill);
    registry.register(DelegateTask);
    registry.register(DelegateTasks);
    registry.register(BrowserTool::new());
    registry.register(GitStatus);
    registry.register(GitLog);
    registry.register(GitDiff);
    registry.register(ImageRead);
    registry.register(WriteNote);
    registry.register(ReadNote);
    registry.register(ListNotes);
    registry.register(JsonQuery);
    if let Some(slot) = cron {
        registry.register(CronCreate { slot: slot.clone() });
        registry.register(CronList { slot: slot.clone() });
        registry.register(CronDelete { slot });
    }
}
