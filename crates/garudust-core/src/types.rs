use serde::{Deserialize, Serialize};

// ─── Message types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentPart {
    Text(String),
    Image {
        mime_type: String,
        data: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Vec<ContentPart>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: vec![ContentPart::Text(text.into())],
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: vec![ContentPart::Text(text.into())],
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: vec![ContentPart::Text(text.into())],
        }
    }

    pub fn text(&self) -> Option<&str> {
        self.content.iter().find_map(|p| {
            if let ContentPart::Text(t) = p {
                Some(t.as_str())
            } else {
                None
            }
        })
    }
}

// ─── Tool call / result ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn ok(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
            is_error: false,
        }
    }

    pub fn err(tool_call_id: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: msg.into(),
            is_error: true,
        }
    }
}

// ─── Tool schema (JSON Schema) ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

// ─── Inference config ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub model: String,
    pub max_tokens: Option<u32>,
    /// Total context window of the model in tokens. When set and `max_tokens`
    /// is None the transport caps output tokens to avoid exceeding the window.
    pub context_limit: Option<u32>,
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// Disable reasoning / extended thinking entirely.
    None,
    /// Very light thinking pass — minimal token budget.
    Minimal,
    Low,
    Medium,
    High,
    /// Maximum thinking budget for the most demanding tasks.
    #[serde(rename = "xhigh")]
    XHigh,
}

// ─── Transport response ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TransportResponse {
    pub content: Vec<ContentPart>,
    pub tool_calls: Vec<ToolCall>,
    pub usage: TokenUsage,
    pub stop_reason: StopReason,
}

#[derive(Debug, Clone, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Other(String),
}

// ─── Stream chunk ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum StreamChunk {
    TextDelta(String),
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        args_delta: String,
    },
    Done {
        usage: TokenUsage,
    },
}

// ─── Platform channel ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ChannelId {
    pub platform: String,
    pub chat_id: String,
    pub thread_id: Option<String>,
}

/// A downloaded image attachment from any platform, stored at a local /tmp path.
#[derive(Debug, Clone)]
pub struct ImageAttachment {
    /// Absolute path to the downloaded image file (e.g. `/tmp/garudust_line_abc.jpg`).
    pub path: String,
}

/// A non-image file attachment (PDF, TXT, CSV, DOCX, …) downloaded from a
/// platform and stored at a local /tmp path, pending user confirmation to ingest.
#[derive(Debug, Clone)]
pub struct DocAttachment {
    /// Absolute path to the downloaded file (e.g. `/tmp/garudust_line_abc.pdf`).
    pub path: String,
    /// Original file name as reported by the platform (e.g. `price_list.pdf`).
    pub file_name: String,
}

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel: ChannelId,
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub session_key: String,
    /// True when the message comes from a group/channel (not a private DM).
    /// Used by GatewayHandler to apply the require_mention gate.
    pub is_group: bool,
    /// Pre-detected mention status from the platform adapter using native
    /// data (LINE's `mention.mentionees`, Telegram entities, etc.).
    /// `Some(true/false)` = adapter knows; `None` = gateway falls back to
    /// text-contains matching against `platform.bot_username`.
    pub bot_mentioned: Option<bool>,
    /// Image files downloaded by the platform adapter, ready for analysis.
    /// Empty for text-only messages.
    pub attachments: Vec<ImageAttachment>,
    /// Non-image document files (PDF, TXT, CSV, …) awaiting user confirmation
    /// before being ingested into the RAG store.
    pub doc_attachments: Vec<DocAttachment>,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub text: String,
    pub markdown: bool,
}

impl OutboundMessage {
    pub fn text(t: impl Into<String>) -> Self {
        Self {
            text: t.into(),
            markdown: false,
        }
    }

    pub fn markdown(t: impl Into<String>) -> Self {
        Self {
            text: t.into(),
            markdown: true,
        }
    }
}

// ─── Agent result ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AgentResult {
    pub output: String,
    pub usage: TokenUsage,
    pub iterations: u32,
    pub session_id: String,
}
