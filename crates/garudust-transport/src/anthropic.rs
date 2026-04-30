use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use garudust_core::{
    error::TransportError,
    transport::{ApiMode, ProviderTransport, StreamResult},
    types::{
        ContentPart, InferenceConfig, Message, Role, StopReason, StreamChunk, TokenUsage, ToolCall,
        ToolSchema, TransportResponse,
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

    fn build_body(
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
        stream: bool,
    ) -> serde_json::Value {
        let system = messages
            .iter()
            .filter(|m| m.role == Role::System)
            .filter_map(|m| m.text())
            .collect::<Vec<_>>()
            .join("\n");

        let msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                let role = match m.role {
                    Role::User | Role::Tool => "user",
                    Role::Assistant => "assistant",
                    Role::System => unreachable!(),
                };
                let content: Vec<_> = m
                    .content
                    .iter()
                    .map(|p| match p {
                        ContentPart::Text(t) => json!({ "type": "text", "text": t }),
                        ContentPart::ToolUse { id, name, input } => {
                            json!({ "type": "tool_use", "id": id, "name": name, "input": input })
                        }
                        ContentPart::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error,
                        }),
                        ContentPart::Image { mime_type, data } => json!({
                            "type": "image",
                            "source": { "type": "base64", "media_type": mime_type, "data": data }
                        }),
                    })
                    .collect();
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
        if stream {
            body["stream"] = json!(true);
        }
        if !system.is_empty() {
            body["system"] = json!(system);
        }
        if !anthropic_tools.is_empty() {
            body["tools"] = json!(anthropic_tools);
        }
        body
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
        let body = Self::build_body(messages, config, tools, false);

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(if status == 401 {
                TransportError::Auth
            } else {
                TransportError::Http { status, body }
            });
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        parse_response(&data)
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        let body = Self::build_body(messages, config, tools, true);

        let resp = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(if status == 401 {
                TransportError::Auth
            } else {
                TransportError::Http { status, body }
            });
        }

        let mut byte_stream = resp.bytes_stream();

        let stream = try_stream! {
            let mut buf = String::new();

            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| TransportError::Stream(e.to_string()))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf = buf[pos + 1..].to_string();

                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(data) {
                            for chunk in parse_anthropic_sse_event(&event) {
                                yield chunk;
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

fn parse_anthropic_sse_event(event: &serde_json::Value) -> Vec<StreamChunk> {
    let mut chunks = Vec::new();
    match event["type"].as_str() {
        Some("content_block_start") => {
            let index = usize::try_from(event["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let block = &event["content_block"];
            if block["type"].as_str() == Some("tool_use") {
                chunks.push(StreamChunk::ToolCallDelta {
                    index,
                    id: block["id"].as_str().map(str::to_string),
                    name: block["name"].as_str().map(str::to_string),
                    args_delta: String::new(),
                });
            }
        }
        Some("content_block_delta") => {
            let index = usize::try_from(event["index"].as_u64().unwrap_or(0)).unwrap_or(0);
            let delta = &event["delta"];
            match delta["type"].as_str() {
                Some("text_delta") => {
                    if let Some(text) = delta["text"].as_str() {
                        if !text.is_empty() {
                            chunks.push(StreamChunk::TextDelta(text.to_string()));
                        }
                    }
                }
                Some("input_json_delta") => {
                    if let Some(partial) = delta["partial_json"].as_str() {
                        chunks.push(StreamChunk::ToolCallDelta {
                            index,
                            id: None,
                            name: None,
                            args_delta: partial.to_string(),
                        });
                    }
                }
                _ => {}
            }
        }
        Some("message_delta") => {
            #[allow(clippy::cast_possible_truncation)]
            let output_tokens = event["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32;
            chunks.push(StreamChunk::Done {
                usage: TokenUsage {
                    output_tokens,
                    ..Default::default()
                },
            });
        }
        _ => {}
    }
    chunks
}

#[allow(clippy::unnecessary_wraps)]
fn parse_response(data: &serde_json::Value) -> Result<TransportResponse, TransportError> {
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
