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
    pub temperature: Option<f32>,
    pub reasoning_effort: Option<ReasoningEffort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    None,
    Low,
    Medium,
    High,
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

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub channel: ChannelId,
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub session_key: String,
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
