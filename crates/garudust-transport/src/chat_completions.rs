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

pub struct ChatCompletionsTransport {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl ChatCompletionsTransport {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }

    fn endpoint(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

fn messages_to_json(messages: &[Message]) -> Vec<Value> {
    messages
        .iter()
        .flat_map(|m| {
            let role = match m.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
                Role::Tool => "tool",
            };

            match m.role {
            Role::Tool => m.content.iter().filter_map(|p| {
                if let ContentPart::ToolResult { tool_use_id, content, .. } = p {
                    Some(json!({ "role": "tool", "tool_call_id": tool_use_id, "content": content }))
                } else { None }
            }).collect(),

            Role::Assistant => {
                let text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p { Some(t.clone()) } else { None }
                });
                let tool_calls: Vec<_> = m.content.iter().filter_map(|p| {
                    if let ContentPart::ToolUse { id, name, input } = p {
                        Some(json!({
                            "id": id,
                            "type": "function",
                            "function": { "name": name, "arguments": input.to_string() }
                        }))
                    } else { None }
                }).collect();

                let mut obj = json!({ "role": role });
                if let Some(t) = text { obj["content"] = json!(t); }
                if !tool_calls.is_empty() { obj["tool_calls"] = json!(tool_calls); }
                vec![obj]
            },

            _ => {
                let text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p { Some(t.clone()) } else { None }
                }).unwrap_or_default();
                vec![json!({ "role": role, "content": text })]
            },
        }
        })
        .collect()
}

fn tools_to_json(tools: &[ToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name":        t.name,
                    "description": t.description,
                    "parameters":  t.parameters,
                }
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

#[async_trait]
impl ProviderTransport for ChatCompletionsTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::ChatCompletions
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        let oai_messages = messages_to_json(messages);
        let oai_tools = tools_to_json(tools);

        let mut body = json!({
            "model":              config.model,
            "messages":           oai_messages,
            "max_completion_tokens": config.max_tokens.unwrap_or(8192),
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

        let choice = data["choices"]
            .as_array()
            .and_then(|a| a.first())
            .ok_or_else(|| TransportError::Other(anyhow::anyhow!("no choices in response")))?;

        let stop_reason = match choice["finish_reason"].as_str() {
            Some("stop") | None => StopReason::EndTurn,
            Some("tool_calls") => StopReason::ToolUse,
            Some("length") => StopReason::MaxTokens,
            Some(other) => StopReason::Other(other.into()),
        };

        let msg = &choice["message"];
        let mut content = Vec::new();
        if let Some(t) = msg["content"].as_str() {
            if !t.is_empty() {
                content.push(ContentPart::Text(t.into()));
            }
        }

        let tool_calls: Vec<ToolCall> = msg["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let id = tc["id"].as_str()?;
                        let name = tc["function"]["name"].as_str()?;
                        let args_str = tc["function"]["arguments"].as_str().unwrap_or("{}");
                        let arguments = serde_json::from_str(args_str).unwrap_or(Value::Null);
                        Some(ToolCall {
                            id: id.into(),
                            name: name.into(),
                            arguments,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        #[allow(clippy::cast_possible_truncation)]
        let usage = TokenUsage {
            input_tokens: data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            output_tokens: data["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            cache_read_tokens: data["usage"]["prompt_tokens_details"]["cached_tokens"]
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

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        let oai_messages = messages_to_json(messages);
        let oai_tools = tools_to_json(tools);

        let mut body = json!({
            "model":                 config.model,
            "messages":              oai_messages,
            "max_completion_tokens": config.max_tokens.unwrap_or(8192),
            "stream":                true,
            "stream_options":        { "include_usage": true },
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
            // tool call accumulator: index → (id, name, args)
            let mut tc_acc: Vec<(String, String, String)> = Vec::new();

            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| TransportError::Stream(e.to_string()))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf = buf[pos + 1..].to_string();

                    let Some(d) = line.strip_prefix("data: ") else {
                        continue;
                    };
                    let data = d.to_string();
                    if data == "[DONE]" {
                        break;
                    }

                    let Ok(event) = serde_json::from_str::<Value>(&data) else {
                        continue;
                    };
                    let Some(choice_ref) =
                        event["choices"].as_array().and_then(|a| a.first())
                    else {
                        // usage-only chunk
                        #[allow(clippy::cast_possible_truncation)]
                        if let (Some(p), Some(c)) = (
                            event["usage"]["prompt_tokens"].as_u64(),
                            event["usage"]["completion_tokens"].as_u64(),
                        ) {
                            yield StreamChunk::Done {
                                usage: TokenUsage {
                                    input_tokens: p as u32,
                                    output_tokens: c as u32,
                                    ..Default::default()
                                },
                            };
                        }
                        continue;
                    };
                    let choice = choice_ref.clone();

                    let delta = &choice["delta"];

                    if let Some(text) = delta["content"].as_str() {
                        if !text.is_empty() {
                            yield StreamChunk::TextDelta(text.to_string());
                        }
                    }

                    if let Some(tcs) = delta["tool_calls"].as_array() {
                        for tc in tcs {
                            #[allow(clippy::cast_possible_truncation)]
                            let index = tc["index"].as_u64().unwrap_or(0) as usize;
                            while tc_acc.len() <= index {
                                tc_acc.push((String::new(), String::new(), String::new()));
                            }
                            if let Some(id) = tc["id"].as_str() {
                                tc_acc[index].0 = id.to_string();
                            }
                            if let Some(name) = tc["function"]["name"].as_str() {
                                tc_acc[index].1 = name.to_string();
                            }
                            if let Some(args) = tc["function"]["arguments"].as_str() {
                                let entry = &mut tc_acc[index];
                                let is_first = entry.0.is_empty() && entry.1.is_empty();
                                yield StreamChunk::ToolCallDelta {
                                    index,
                                    id: if is_first {
                                        tc["id"].as_str().map(str::to_string)
                                    } else {
                                        None
                                    },
                                    name: if is_first {
                                        tc["function"]["name"].as_str().map(str::to_string)
                                    } else {
                                        None
                                    },
                                    args_delta: args.to_string(),
                                };
                                entry.2.push_str(args);
                            }
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use garudust_core::types::{ContentPart, Message, Role};

    #[test]
    fn classify_401_is_auth() {
        assert!(matches!(classify_error(401, ""), TransportError::Auth));
    }

    #[test]
    fn classify_429_is_rate_limit() {
        assert!(matches!(
            classify_error(429, ""),
            TransportError::RateLimit { .. }
        ));
    }

    #[test]
    fn classify_500_is_http() {
        assert!(matches!(
            classify_error(500, "oops"),
            TransportError::Http { status: 500, .. }
        ));
    }

    #[test]
    fn system_message_serializes() {
        let msgs = vec![Message::system("be helpful")];
        let json = messages_to_json(&msgs);
        assert_eq!(json.len(), 1);
        assert_eq!(json[0]["role"], "system");
        assert_eq!(json[0]["content"], "be helpful");
    }

    #[test]
    fn tool_result_becomes_tool_role() {
        let msg = Message {
            role: Role::Tool,
            content: vec![ContentPart::ToolResult {
                tool_use_id: "call_1".into(),
                content: "42".into(),
                is_error: false,
            }],
        };
        let json = messages_to_json(&[msg]);
        assert_eq!(json.len(), 1);
        assert_eq!(json[0]["role"], "tool");
        assert_eq!(json[0]["tool_call_id"], "call_1");
        assert_eq!(json[0]["content"], "42");
    }

    #[test]
    fn assistant_with_tool_call() {
        let msg = Message {
            role: Role::Assistant,
            content: vec![ContentPart::ToolUse {
                id: "tc1".into(),
                name: "read_file".into(),
                input: serde_json::json!({ "path": "/tmp/x" }),
            }],
        };
        let json = messages_to_json(&[msg]);
        assert_eq!(json.len(), 1);
        let tcs = json[0]["tool_calls"].as_array().unwrap();
        assert_eq!(tcs[0]["function"]["name"], "read_file");
    }
}
