use crate::chat_completions::ChatCompletionsTransport;

pub const DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";

/// Build a transport for a local Ollama server.
///
/// Ollama's OpenAI-compatible endpoint uses `max_tokens` (not `max_completion_tokens`).
/// No API key is required — the auth header is omitted when the key is empty.
pub fn new(base_url: String) -> ChatCompletionsTransport {
    ChatCompletionsTransport::new(base_url, String::new()).with_tokens_param("max_tokens")
}
