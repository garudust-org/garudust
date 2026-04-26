use async_trait::async_trait;
use garudust_core::{
    error::TransportError,
    transport::{ApiMode, ProviderTransport, StreamResult},
    types::{
        ContentPart, InferenceConfig, Message, Role, StopReason, TokenUsage, ToolCall, ToolSchema,
        TransportResponse,
    },
};
use serde_json::json;

pub struct AnthropicTransport {
    client: reqwest::Client,
    api_key: String,
}

impl AnthropicTransport {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl ProviderTransport for AnthropicTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::AnthropicMessages
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        let system = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .filter_map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");

        let msgs: Vec<_> =
            messages
                .iter()
                .filter(|m| m.role != Role::System)
                .map(|m| {
                    let role = match m.role {
                        Role::User | Role::Tool => "user",
                        Role::Assistant => "assistant",
                        Role::System => unreachable!(),
                    };
                    let content: Vec<_> = m.content.iter().map(|p| match p {
                    ContentPart::Text(t) => json!({ "type": "text", "text": t }),
                    ContentPart::ToolUse { id, name, input } => json!({
                        "type": "tool_use", "id": id, "name": name, "input": input
                    }),
                    ContentPart::ToolResult { tool_use_id, content, is_error } => json!({
                        "type": "tool_result",
                        "tool_use_id": tool_use_id,
                        "content": content,
                        "is_error": is_error,
                    }),
                    ContentPart::Image { mime_type, data } => json!({
                        "type": "image",
                        "source": { "type": "base64", "media_type": mime_type, "data": data }
                    }),
                }).collect();
                    json!({ "role": role, "content": content })
                })
                .collect();

        let anthropic_tools: Vec<_> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "input_schema": t.parameters,
                })
            })
            .collect();

        let mut body = json!({
            "model":      config.model,
            "max_tokens": config.max_tokens.unwrap_or(8192),
            "messages":   msgs,
        });
        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !anthropic_tools.is_empty() {
            body["tools"] = json!(anthropic_tools);
        }

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(TransportError::Http { status, body });
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;

        let stop_reason = match data["stop_reason"].as_str() {
            Some("end_turn") | None => StopReason::EndTurn,
            Some("tool_use") => StopReason::ToolUse,
            Some("max_tokens") => StopReason::MaxTokens,
            Some(other) => StopReason::Other(other.into()),
        };

        let mut content = Vec::new();
        let mut tool_calls = Vec::new();

        if let Some(arr) = data["content"].as_array() {
            for block in arr {
                match block["type"].as_str() {
                    Some("text") => {
                        if let Some(t) = block["text"].as_str() {
                            content.push(ContentPart::Text(t.into()));
                        }
                    }
                    Some("tool_use") => {
                        tool_calls.push(ToolCall {
                            id: block["id"].as_str().unwrap_or("").into(),
                            name: block["name"].as_str().unwrap_or("").into(),
                            arguments: block["input"].clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        #[allow(clippy::cast_possible_truncation)]
        let usage = TokenUsage {
            input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
            cache_read_tokens: data["usage"]["cache_read_input_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
            cache_write_tokens: data["usage"]["cache_creation_input_tokens"]
                .as_u64()
                .unwrap_or(0) as u32,
        };

        Ok(TransportResponse {
            content,
            tool_calls,
            usage,
            stop_reason,
        })
    }

    async fn chat_stream(
        &self,
        _messages: &[Message],
        _config: &InferenceConfig,
    ) -> Result<StreamResult, TransportError> {
        Err(TransportError::Other(anyhow::anyhow!(
            "streaming not yet implemented"
        )))
    }
}
