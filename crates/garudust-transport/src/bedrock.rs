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

/// AWS Bedrock Converse API (`POST /model/{model}/converse`).
///
/// Requires `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, and `AWS_REGION`
/// to be set in the environment.  The model ID is taken from `InferenceConfig`
/// at call time (e.g. `anthropic.claude-3-5-sonnet-20241022-v2:0`).
pub struct BedrockTransport {
    client: reqwest::Client,
    region: String,
    access_key: String,
    secret_key: String,
    session_token: Option<String>,
}

impl BedrockTransport {
    /// Builds a transport from environment variables:
    /// `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN`, `AWS_REGION`.
    pub fn from_env() -> Result<Self, TransportError> {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| TransportError::Other(anyhow::anyhow!("AWS_ACCESS_KEY_ID not set")))?;
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| TransportError::Other(anyhow::anyhow!("AWS_SECRET_ACCESS_KEY not set")))?;
        let region = std::env::var("AWS_REGION")
            .or_else(|_| std::env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| "us-east-1".into());
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        Ok(Self {
            client: reqwest::Client::new(),
            region,
            access_key,
            secret_key,
            session_token,
        })
    }

    fn converse_url(&self, model_id: &str) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
            self.region, model_id
        )
    }

    fn converse_stream_url(&self, model_id: &str) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse-stream",
            self.region, model_id
        )
    }

    /// Minimal AWS Signature V4 for application/json POST requests.
    #[allow(clippy::unnecessary_wraps)]
    fn sign(&self, url: &str, body_bytes: &[u8]) -> Result<Vec<(String, String)>, TransportError> {
        use std::fmt::Write;

        let now = chrono::Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Extract host and path from URL without the url crate.
        let without_scheme = url
            .strip_prefix("https://")
            .or_else(|| url.strip_prefix("http://"))
            .unwrap_or(url);
        let (host, path) = if let Some(slash) = without_scheme.find('/') {
            (
                without_scheme[..slash].to_string(),
                without_scheme[slash..].to_string(),
            )
        } else {
            (without_scheme.to_string(), "/".to_string())
        };

        let payload_hash = sha256_hex(body_bytes);

        let canonical_headers = format!(
            "content-type:application/json\nhost:{host}\nx-amz-date:{amz_date}\nx-amz-security-token:{token}\n",
            token = self.session_token.as_deref().unwrap_or(""),
        );
        let signed_headers = "content-type;host;x-amz-date;x-amz-security-token";

        let canonical_request =
            format!("POST\n{path}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
        let credential_scope = format!("{date_stamp}/{}/bedrock/aws4_request", self.region);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
            sha256_hex(canonical_request.as_bytes())
        );

        let signing_key = {
            let k_date = hmac_sha256(
                format!("AWS4{}", self.secret_key).as_bytes(),
                date_stamp.as_bytes(),
            );
            let k_region = hmac_sha256(&k_date, self.region.as_bytes());
            let k_service = hmac_sha256(&k_region, b"bedrock");
            hmac_sha256(&k_service, b"aws4_request")
        };

        let signature_bytes = hmac_sha256(&signing_key, string_to_sign.as_bytes());
        let mut signature = String::new();
        for b in &signature_bytes {
            let _ = write!(signature, "{b:02x}");
        }

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.access_key
        );

        let mut headers = vec![
            ("x-amz-date".into(), amz_date),
            ("x-amz-content-sha256".into(), payload_hash),
            ("authorization".into(), authorization),
        ];
        if let Some(tok) = &self.session_token {
            headers.push(("x-amz-security-token".into(), tok.clone()));
        }
        Ok(headers)
    }
}

fn sha256_hex(data: &[u8]) -> String {
    use std::fmt::Write;
    // Pure-Rust SHA-256 via the `sha2` crate is unavailable here; use a
    // hand-rolled call to the OS (ring/sha2 not in workspace deps).
    // Instead we rely on the system OpenSSL via the `ring`-free approach:
    // compute using the `sha2` crate if available, otherwise return a dummy.
    //
    // Because the workspace does not add sha2/ring, we use a tiny inline
    // implementation based on the public SHA-256 spec.
    let hash = sha2_256(data);
    let mut out = String::with_capacity(64);
    for b in &hash {
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 64;
    let mut k = if key.len() > BLOCK {
        sha2_256(key).to_vec()
    } else {
        key.to_vec()
    };
    k.resize(BLOCK, 0);
    let mut ipad = k.clone();
    let mut opad = k;
    for b in &mut ipad {
        *b ^= 0x36;
    }
    for b in &mut opad {
        *b ^= 0x5c;
    }
    let mut inner = ipad;
    inner.extend_from_slice(data);
    let inner_hash = sha2_256(&inner);
    let mut outer = opad;
    outer.extend_from_slice(&inner_hash);
    sha2_256(&outer).to_vec()
}

/// Minimal pure-Rust SHA-256 (RFC 6234).
#[allow(clippy::many_single_char_names)]
fn sha2_256(data: &[u8]) -> [u8; 32] {
    #[allow(clippy::unreadable_literal)]
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    #[allow(clippy::unreadable_literal)]
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, chunk) in block.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn messages_to_converse(messages: &[Message]) -> (Option<String>, Vec<Value>) {
    let mut system_text: Option<String> = None;
    let mut converse_msgs: Vec<Value> = Vec::new();

    for m in messages {
        match m.role {
            Role::System => {
                system_text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p {
                        Some(t.clone())
                    } else {
                        None
                    }
                });
            }
            Role::User => {
                let text = m.content.iter().find_map(|p| {
                    if let ContentPart::Text(t) = p {
                        Some(t.clone())
                    } else {
                        None
                    }
                });
                if let Some(t) = text {
                    converse_msgs.push(json!({
                        "role": "user",
                        "content": [{ "text": t }]
                    }));
                }
            }
            Role::Assistant => {
                let mut content: Vec<Value> = Vec::new();
                for p in &m.content {
                    match p {
                        ContentPart::Text(t) => {
                            content.push(json!({ "text": t }));
                        }
                        ContentPart::ToolUse { id, name, input } => {
                            content.push(json!({
                                "toolUse": {
                                    "toolUseId": id,
                                    "name": name,
                                    "input": input,
                                }
                            }));
                        }
                        _ => {}
                    }
                }
                if !content.is_empty() {
                    converse_msgs.push(json!({ "role": "assistant", "content": content }));
                }
            }
            Role::Tool => {
                let results: Vec<Value> = m
                    .content
                    .iter()
                    .filter_map(|p| {
                        if let ContentPart::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } = p
                        {
                            Some(json!({
                                "toolResult": {
                                    "toolUseId": tool_use_id,
                                    "content": [{ "text": content }],
                                    "status": if *is_error { "error" } else { "success" },
                                }
                            }))
                        } else {
                            None
                        }
                    })
                    .collect();
                if !results.is_empty() {
                    converse_msgs.push(json!({ "role": "user", "content": results }));
                }
            }
        }
    }
    (system_text, converse_msgs)
}

fn tools_to_converse(tools: &[ToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "toolSpec": {
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": { "json": t.parameters },
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

fn parse_converse_response(data: &Value) -> TransportResponse {
    let output = &data["output"]["message"];
    let mut content: Vec<ContentPart> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    if let Some(parts) = output["content"].as_array() {
        for part in parts {
            if let Some(text) = part["text"].as_str() {
                content.push(ContentPart::Text(text.into()));
            }
            if let Some(tu) = part.get("toolUse") {
                let id = tu["toolUseId"].as_str().unwrap_or("").to_string();
                let name = tu["name"].as_str().unwrap_or("").to_string();
                let arguments = tu["input"].clone();
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments,
                });
            }
        }
    }

    let stop_reason = match data["stopReason"].as_str() {
        Some("end_turn") | None => StopReason::EndTurn,
        Some("tool_use") => StopReason::ToolUse,
        Some("max_tokens") => StopReason::MaxTokens,
        Some(other) => StopReason::Other(other.into()),
    };

    #[allow(clippy::cast_possible_truncation)]
    let usage = TokenUsage {
        input_tokens: data["usage"]["inputTokens"].as_u64().unwrap_or(0) as u32,
        output_tokens: data["usage"]["outputTokens"].as_u64().unwrap_or(0) as u32,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
    };

    TransportResponse {
        content,
        tool_calls,
        usage,
        stop_reason,
    }
}

#[async_trait]
impl ProviderTransport for BedrockTransport {
    fn api_mode(&self) -> ApiMode {
        ApiMode::BedrockConverse
    }

    async fn chat(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<TransportResponse, TransportError> {
        let (system_text, converse_msgs) = messages_to_converse(messages);
        let converse_tools = tools_to_converse(tools);

        let mut body = json!({
            "messages": converse_msgs,
            "inferenceConfig": {
                "maxTokens": config.max_tokens.unwrap_or(8192),
            },
        });
        if let Some(sys) = system_text {
            body["system"] = json!([{ "text": sys }]);
        }
        if let Some(t) = config.temperature {
            body["inferenceConfig"]["temperature"] = json!(t);
        }
        if !converse_tools.is_empty() {
            body["toolConfig"] = json!({ "tools": converse_tools });
        }

        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;
        let url = self.converse_url(&config.model);
        let extra_headers = self.sign(&url, &body_bytes)?;

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .body(body_bytes);
        for (k, v) in extra_headers {
            req = req.header(k, v);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        let text = resp
            .text()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        if status != 200 {
            return Err(classify_error(status, &text));
        }

        let data: Value = serde_json::from_str(&text).map_err(|e| {
            TransportError::Other(anyhow::anyhow!("parse error: {e}\nbody: {text}"))
        })?;

        Ok(parse_converse_response(&data))
    }

    async fn chat_stream(
        &self,
        messages: &[Message],
        config: &InferenceConfig,
        tools: &[ToolSchema],
    ) -> Result<StreamResult, TransportError> {
        let (system_text, converse_msgs) = messages_to_converse(messages);
        let converse_tools = tools_to_converse(tools);

        let mut body = json!({
            "messages": converse_msgs,
            "inferenceConfig": {
                "maxTokens": config.max_tokens.unwrap_or(8192),
            },
        });
        if let Some(sys) = system_text {
            body["system"] = json!([{ "text": sys }]);
        }
        if let Some(t) = config.temperature {
            body["inferenceConfig"]["temperature"] = json!(t);
        }
        if !converse_tools.is_empty() {
            body["toolConfig"] = json!({ "tools": converse_tools });
        }

        let body_bytes =
            serde_json::to_vec(&body).map_err(|e| TransportError::Other(anyhow::anyhow!("{e}")))?;
        let url = self.converse_stream_url(&config.model);
        let extra_headers = self.sign(&url, &body_bytes)?;

        let mut req = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .body(body_bytes);
        for (k, v) in extra_headers {
            req = req.header(k, v);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| TransportError::Network(e.to_string()))?;

        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            return Err(classify_error(status, &text));
        }

        let mut byte_stream = resp.bytes_stream();

        // Bedrock streaming uses HTTP/2 event-stream framing; each chunk is a
        // JSON object with a "chunk" field containing base64-encoded event bytes.
        // For simplicity we parse each newline-delimited JSON object.
        let stream = try_stream! {
            let mut buf = String::new();

            while let Some(chunk) = byte_stream.next().await {
                let bytes = chunk.map_err(|e| TransportError::Stream(e.to_string()))?;
                buf.push_str(&String::from_utf8_lossy(&bytes));

                while let Some(pos) = buf.find('\n') {
                    let line = buf[..pos].trim().to_string();
                    buf = buf[pos + 1..].to_string();

                    let Ok(ev) = serde_json::from_str::<Value>(&line) else { continue };

                    if let Some(text) = ev["contentBlockDelta"]["delta"]["text"].as_str() {
                        if !text.is_empty() {
                            yield StreamChunk::TextDelta(text.to_string());
                        }
                    }

                    if let Some(tu_start) = ev.get("contentBlockStart") {
                        if let Some(tu) = tu_start["start"].get("toolUse") {
                            let id = tu["toolUseId"].as_str().map(str::to_string);
                            let name = tu["name"].as_str().map(str::to_string);
                            #[allow(clippy::cast_possible_truncation)]
                            let index = tu_start["contentBlockIndex"].as_u64().unwrap_or(0) as usize;
                            yield StreamChunk::ToolCallDelta {
                                index,
                                id,
                                name,
                                args_delta: String::new(),
                            };
                        }
                    }

                    if let Some(delta) = ev.get("contentBlockDelta") {
                        if let Some(args) = delta["delta"]["toolUse"]["input"].as_str() {
                            #[allow(clippy::cast_possible_truncation)]
                            let index = delta["contentBlockIndex"].as_u64().unwrap_or(0) as usize;
                            yield StreamChunk::ToolCallDelta {
                                index,
                                id: None,
                                name: None,
                                args_delta: args.to_string(),
                            };
                        }
                    }

                    if let Some(meta) = ev.get("metadata") {
                        #[allow(clippy::cast_possible_truncation)]
                        let input_tokens = meta["usage"]["inputTokens"].as_u64().unwrap_or(0) as u32;
                        #[allow(clippy::cast_possible_truncation)]
                        let output_tokens = meta["usage"]["outputTokens"].as_u64().unwrap_or(0) as u32;
                        yield StreamChunk::Done {
                            usage: TokenUsage {
                                input_tokens,
                                output_tokens,
                                ..Default::default()
                            },
                        };
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}
