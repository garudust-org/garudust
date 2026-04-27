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
use serde_json::{json, Value};

/// OpenAI Responses API (`POST /v1/responses`).
pub struct CodexTransport {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl CodexTransport {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com".into(),
            api_key,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    fn endpoint(&self) -> String {
        format!("{}/v1/responses", self.base_url.trim_end_matches('/'))
    }
}

fn messages_to_input(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .flat_map(|m| match m.role {
            Role::System => {
                let text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p {
                        Some(t.clone())
                    } else {
                        None
                    }
                });
                text.map(|t| vec![json!({ "role": "system", "content": t })])
                    .unwrap_or_default()
            }
            Role::User => {
                let text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p {
                        Some(t.clone())
                    } else {
                        None
                    }
                });
                text.map(|t| vec![json!({ "role": "user", "content": t })])
                    .unwrap_or_default()
            }
            Role::Assistant => {
                let mut items: Vec<Value> = Vec::new();
                if let Some(text) = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p {
                        Some(t.clone())
                    } else {
                        None
                    }
                }) {
                    items.push(json!({ "role": "assistant", "content": text }));
                }
                for p in &m.content {
                    if let ContentPart::ToolUse { id, name, input } = p {
                        items.push(json!({
                            "type": "function_call",
                            "call_id": id,
                            "name": name,
                            "arguments": input.to_string(),
                        }));
                    }
                }
                items
            }
            Role::Tool => m
                .content
                .iter()
                .filter_map(|p| {
                    if let ContentPart::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = p
                    {
                        Some(json!({
                            "type": "function_call_output",
                            "call_id": tool_use_id,
                            "output": content,
                        }))
                    } else {
                        None
                    }
                })
                .collect(),
        })
        .collect()
}

fn tools_to_json(tools: &[ToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect()
}

fn classify_error(status: u16, body: &str) -> TransportError {
    match status {
        401 | 403 => TransportError::Auth,
        429 => TransportError::RateLimit {
            retry_after_secs: 60,
        },
        _ => TransportError::Http {
            status,
            body: body.to_string(),
        },
    }
}

#[allow(clippy::unnecessary_wraps)]
fn parse_response(data: &Value) -> Result<TransportResponse, TransportError> {
    let output_arr = data["output"]
        .as_array()
        .map_or_else(Vec::new, Clone::clone);

    let mut content: Vec<ContentPart> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut stop_reason = StopReason::EndTurn;

    for item in &output_arr {
        match item["type"].as_str() {
            Some("message") => {
                if let Some(parts) = item["content"].as_array() {
                    for part in parts {
                        if part["type"].as_str() == Some("text") {
                            if let Some(t) = part["text"].as_str() {
                                content.push(ContentPart::Text(t.into()));
                            }
                        }
                    }
                }
            }
            Some("function_call") => {
                let id = item["call_id"].as_str().unwrap_or("").to_string();
                let name = item["name"].as_str().unwrap_or("").to_string();
                let args_str = item["arguments"].as_str().unwrap_or("{}");
                let arguments = serde_json::from_str(args_str).unwrap_or(Value::Null);
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
                stop_reason = StopReason::ToolUse;
            }
            _ => {}
        }
    }

    if data["incomplete_details"]["reason"].as_str() == Some("max_output_tokens") {
        stop_reason = StopReason::MaxTokens;
    }

    #[allow(clippy::cast_possible_truncation)]
    let usage = TokenUsage {
        input_tokens: data["usage"]["input_tokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: data["usage"]["output_tokens"].as_u64().unwrap_or(0) as u32,
        cache_read_tokens: data["usage"]["input_tokens_details"]["cached_tokens"]
            .as_u64()
            .unwrap_or(0) as u32,
        cache_write_tokens: 0,
    };

    Ok(TransportResponse {
        content,
        tool_calls,
        usage,
        stop_reason,
    })
}

#[async_trait]
impl ProviderTransport for CodexTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::CodexResponses
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        let input = messages_to_input(messages);
        let oai_tools = tools_to_json(tools);

        let mut body = json!({
            "model":      config.model,
            "input":      input,
            "max_output_tokens": config.max_tokens.unwrap_or(8192),
        });
        if let Some(t) = config.temperature {
            body["temperature"] = json!(t);
        }
        if !oai_tools.is_empty() {
            body["tools"] = json!(oai_tools);
        }

        let resp = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;

        if status != 200 {
            return Err(classify_error(status, &text));
        }

        let data: Value = serde_json::from_str(&text).map_err(|e| {
            TransportError::Other(anyhow::anyhow!("parse error: {e}\nbody: {text}"))
        })?;

        parse_response(&data)
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        let input = messages_to_input(messages);
        let oai_tools = tools_to_json(tools);

        let mut body = json!({
            "model":             config.model,
            "input":             input,
            "max_output_tokens": config.max_tokens.unwrap_or(8192),
            "stream":            true,
        });
        if let Some(t) = config.temperature {
            body["temperature"] = json!(t);
        }
        if !oai_tools.is_empty() {
            body["tools"] = json!(oai_tools);
        }

        let resp = self
            .client
            .post(self.endpoint())
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            return Err(classify_error(status, &text));
        }

        let mut byte_stream = resp.bytes_stream();

        let stream = try_stream! {
            let mut buf = String::new();
            let mut tc_index: usize = 0;

            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| TransportError::Stream(e.to_string()))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf = buf[pos + 1..].to_string();

                    let event_type = if let Some(t) = line.strip_prefix("event: ") {
                        t.to_string()
                    } else if line.starts_with("data: ") {
                        String::new()
                    } else {
                        continue;
                    };

                    let _ = event_type; // event type used below via data parsing
                    let Some(data_str) = line.strip_prefix("data: ") else { continue };
                    if data_str == "[DONE]" { break; }

                    let Ok(ev) = serde_json::from_str::<Value>(data_str) else { continue };

                    match ev["type"].as_str() {
                        Some("response.output_text.delta") => {
                            if let Some(delta) = ev["delta"].as_str() {
                                if !delta.is_empty() {
                                    yield StreamChunk::TextDelta(delta.to_string());
                                }
                            }
                        }
                        Some("response.function_call_arguments.delta") => {
                            let args_delta = ev["delta"].as_str().unwrap_or("").to_string();
                            let idx = ev["output_index"].as_u64().map_or(tc_index, |v| {
                                #[allow(clippy::cast_possible_truncation)]
                                { v as usize }
                            });
                            tc_index = idx;
                            let id = ev["item_id"].as_str().map(str::to_string);
                            let name = ev["name"].as_str().map(str::to_string);
                            yield StreamChunk::ToolCallDelta {
                                index: idx,
                                id,
                                name,
                                args_delta,
                            };
                        }
                        Some("response.completed") => {
                            #[allow(clippy::cast_possible_truncation)]
                            if let Some(usage_obj) = ev["response"]["usage"].as_object() {
                                let input_tokens = usage_obj.get("input_tokens")
                                    .and_then(Value::as_u64).unwrap_or(0) as u32;
                                let output_tokens = usage_obj.get("output_tokens")
                                    .and_then(Value::as_u64).unwrap_or(0) as u32;
                                yield StreamChunk::Done {
                                    usage: TokenUsage {
                                        input_tokens,
                                        output_tokens,
                                        ..Default::default()
                                    },
                                };
                            }
                        }
                        _ => {}
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
