use std::sync::Arc;

use chrono::Utc;
use futures::StreamExt;
use garudust_core::{
    budget::IterationBudget,
    config::AgentConfig,
    error::AgentError,
    memory::MemoryStore,
    tool::{SubAgentRunner, ToolContext},
    transport::ProviderTransport,
    types::{
        AgentResult, ContentPart, InferenceConfig, Message, Role, StopReason, StreamChunk,
        TokenUsage, ToolCall, ToolResult, TransportResponse,
    },
};
use garudust_memory::SessionDb;
use garudust_tools::ToolRegistry;
use serde_json::Value;
use tokio::sync::mpsc;

/// Tools whose output originates from external, untrusted sources.
/// Results from these tools are wrapped in XML tags to help the model
/// distinguish untrusted data from authoritative instructions.
const EXTERNAL_TOOLS: &[&str] = &["web_fetch", "web_search", "browser", "read_file"];
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::compressor::ContextCompressor;
use crate::prompt_builder::build_system_prompt;

async fn stream_turn(
    transport: &dyn ProviderTransport,
    history: &[Message],
    config: &InferenceConfig,
    schemas: &[garudust_core::types::ToolSchema],
    chunk_tx: &mpsc::UnboundedSender<String>,
) -> Result<TransportResponse, AgentError> {
    let mut stream = transport.chat_stream(history, config, schemas).await?;

    let mut text = String::new();
    let mut tc_acc: Vec<(String, String, String)> = Vec::new();
    let mut usage = TokenUsage::default();

    while let Some(result) = stream.next().await {
        match result? {
            StreamChunk::TextDelta(delta) => {
                let _ = chunk_tx.send(delta.clone());
                text.push_str(&delta);
            }
            StreamChunk::ToolCallDelta {
                index,
                id,
                name,
                args_delta,
            } => {
                while tc_acc.len() <= index {
                    tc_acc.push((String::new(), String::new(), String::new()));
                }
                if let Some(v) = id {
                    tc_acc[index].0 = v;
                }
                if let Some(v) = name {
                    tc_acc[index].1 = v;
                }
                tc_acc[index].2.push_str(&args_delta);
            }
            StreamChunk::Done { usage: u } => {
                usage = u;
            }
        }
    }

    let content = if text.is_empty() {
        vec![]
    } else {
        vec![ContentPart::Text(text)]
    };

    let tool_calls: Vec<ToolCall> = tc_acc
        .into_iter()
        .filter(|(id, ..)| !id.is_empty())
        .map(|(id, name, args)| ToolCall {
            id,
            name,
            arguments: serde_json::from_str(&args).unwrap_or(Value::Null),
        })
        .collect();

    let stop_reason = if tool_calls.is_empty() {
        StopReason::EndTurn
    } else {
        StopReason::ToolUse
    };

    Ok(TransportResponse {
        content,
        tool_calls,
        usage,
        stop_reason,
    })
}

pub struct Agent {
    id: String,
    transport: Arc<dyn ProviderTransport>,
    tools: Arc<ToolRegistry>,
    memory: Arc<dyn MemoryStore>,
    budget: Arc<IterationBudget>,
    config: Arc<AgentConfig>,
    compressor: ContextCompressor,
    session_db: Option<Arc<SessionDb>>,
}

impl Clone for Agent {
    fn clone(&self) -> Self {
        let comp_model = self
            .config
            .compression
            .model
            .clone()
            .unwrap_or_else(|| self.config.model.clone());
        Self {
            id: self.id.clone(),
            transport: self.transport.clone(),
            tools: self.tools.clone(),
            memory: self.memory.clone(),
            budget: self.budget.clone(),
            config: self.config.clone(),
            compressor: ContextCompressor::new(self.transport.clone(), comp_model),
            session_db: self.session_db.clone(),
        }
    }
}

#[async_trait::async_trait]
impl SubAgentRunner for Agent {
    async fn run_task(&self, task: &str, session_id: &str) -> Result<String, AgentError> {
        let approver = Arc::new(crate::approver::AutoApprover);
        let result = self.run(task, approver, session_id).await?;
        Ok(result.output)
    }
}

impl Agent {
    pub fn new(
        transport: Arc<dyn ProviderTransport>,
        tools: Arc<ToolRegistry>,
        memory: Arc<dyn MemoryStore>,
        config: Arc<AgentConfig>,
    ) -> Self {
        let budget = Arc::new(IterationBudget::new(config.max_iterations));
        let comp_model = config
            .compression
            .model
            .clone()
            .unwrap_or_else(|| config.model.clone());
        let compressor = ContextCompressor::new(transport.clone(), comp_model);
        Self {
            id: Uuid::new_v4().to_string(),
            transport,
            tools,
            memory,
            budget,
            config,
            compressor,
            session_db: None,
        }
    }

    pub fn with_session_db(mut self, db: Arc<SessionDb>) -> Self {
        self.session_db = Some(db);
        self
    }

    pub fn spawn_child(&self) -> Self {
        let comp_model = self
            .config
            .compression
            .model
            .clone()
            .unwrap_or_else(|| self.config.model.clone());
        Self {
            id: Uuid::new_v4().to_string(),
            transport: self.transport.clone(),
            tools: self.tools.clone(),
            memory: self.memory.clone(),
            budget: self.budget.clone(),
            config: self.config.clone(),
            compressor: ContextCompressor::new(self.transport.clone(), comp_model),
            session_db: self.session_db.clone(),
        }
    }

    pub async fn run(
        &self,
        task: &str,
        approver: Arc<dyn garudust_core::tool::CommandApprover>,
        platform: &str,
    ) -> Result<AgentResult, AgentError> {
        self.run_inner(task, approver, platform, None).await
    }

    pub async fn run_streaming(
        &self,
        task: &str,
        approver: Arc<dyn garudust_core::tool::CommandApprover>,
        platform: &str,
        chunk_tx: mpsc::UnboundedSender<String>,
    ) -> Result<AgentResult, AgentError> {
        self.run_inner(task, approver, platform, Some(chunk_tx))
            .await
    }

    async fn run_inner(
        &self,
        task: &str,
        approver: Arc<dyn garudust_core::tool::CommandApprover>,
        platform: &str,
        chunk_tx: Option<mpsc::UnboundedSender<String>>,
    ) -> Result<AgentResult, AgentError> {
        let session_id = Uuid::new_v4().to_string();
        #[allow(clippy::cast_precision_loss)]
        let started_at = Utc::now().timestamp_millis() as f64 / 1000.0;
        // Read memory once — shared by system-prompt serialization and prefetch injection.
        let mem = self
            .memory
            .read_memory()
            .await
            .map_err(|e| {
                warn!("failed to read memory: {e}");
                e
            })
            .ok();
        let profile = self
            .memory
            .read_user_profile()
            .await
            .map_err(|e| {
                warn!("failed to read user profile: {e}");
                e
            })
            .ok();
        let system_prompt =
            build_system_prompt(&self.config, mem.as_ref(), profile.as_deref(), platform).await;
        let inf_config = InferenceConfig {
            model: self.config.model.clone(),
            max_tokens: Some(8192),
            temperature: None,
            reasoning_effort: None,
        };

        // Pre-turn memory recall: surface entries relevant to this task so the
        // model sees them immediately before the question, not buried in the system prompt.
        // Note: prefetch uses ASCII/Latin keyword matching; non-Latin scripts (e.g. Thai)
        // are not word-tokenized and will not trigger recall via this path — the full
        // memory block in the system prompt still covers those cases.
        let user_msg = mem
            .as_ref()
            .and_then(|m| {
                let s = m.prefetch_for_prompt(task);
                (!s.is_empty()).then_some(s)
            })
            .map_or_else(
                || task.to_string(),
                |recalled| {
                    // Strip < and > so an agent-written memory entry (e.g. from a
                    // malicious web page instructing the agent to save crafted content)
                    // cannot inject a closing tag and break out of the block.
                    let safe = recalled.replace(['<', '>'], "");
                    format!("<recalled_memory>\n{safe}\n</recalled_memory>\n\n{task}")
                },
            );

        let mut history: Vec<Message> =
            vec![Message::system(&system_prompt), Message::user(&user_msg)];

        let schemas = self.tools.all_schemas();
        let mut total_in = 0u32;
        let mut total_out = 0u32;
        let mut iters = 0u32;

        loop {
            // Compress if needed before every LLM call
            if self.config.compression.enabled && self.compressor.should_compress(&history) {
                info!("compressing context before turn {}", iters + 1);
                let (compressed, usage) = self.compressor.compress(history).await?;
                history = compressed;
                total_in += usage.input_tokens;
                total_out += usage.output_tokens;
            }

            self.budget.consume()?;
            iters += 1;
            info!(agent_id = %self.id, iteration = iters, "agent turn");

            let resp = if let Some(tx) = &chunk_tx {
                stream_turn(self.transport.as_ref(), &history, &inf_config, &schemas, tx).await?
            } else {
                self.transport.chat(&history, &inf_config, &schemas).await?
            };
            total_in += resp.usage.input_tokens;
            total_out += resp.usage.output_tokens;

            history.push(Message {
                role: Role::Assistant,
                content: resp.content.clone(),
            });

            if resp.tool_calls.is_empty() || resp.stop_reason == StopReason::EndTurn {
                let output = resp
                    .content
                    .iter()
                    .filter_map(|p| {
                        if let ContentPart::Text(t) = p {
                            Some(t.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let result = AgentResult {
                    output,
                    usage: garudust_core::types::TokenUsage {
                        input_tokens: total_in,
                        output_tokens: total_out,
                        ..Default::default()
                    },
                    iterations: iters,
                    session_id: session_id.clone(),
                };

                self.persist_session(&session_id, platform, started_at, &history, &result);
                return Ok(result);
            }

            // Parallel tool dispatch via tokio::join_all
            let sub_agent: Arc<dyn SubAgentRunner> = Arc::new(self.clone());
            let ctx = Arc::new(ToolContext {
                session_id: session_id.clone(),
                agent_id: self.id.clone(),
                iteration: iters,
                budget: self.budget.clone(),
                memory: self.memory.clone(),
                config: self.config.clone(),
                approver: approver.clone(),
                sub_agent: Some(sub_agent),
            });

            let tool_futs: Vec<_> = resp
                .tool_calls
                .iter()
                .map(|tc| {
                    let tools = self.tools.clone();
                    let ctx = ctx.clone();
                    let name = tc.name.clone();
                    let args = tc.arguments.clone();
                    let id = tc.id.clone();
                    async move {
                        debug!(tool = %name, "dispatching");
                        let res = tools.dispatch(&name, args, &ctx).await;
                        let tr = match res {
                            Ok(r) => r,
                            Err(e) => ToolResult::err(&id, e.to_string()),
                        };
                        // Wrap output from external tools so the model can distinguish
                        // untrusted data from trusted instructions (prompt injection defence).
                        let content = if !tr.is_error && EXTERNAL_TOOLS.contains(&name.as_str()) {
                            format!(
                                "<untrusted_external_content>\n{}\n\
                                 </untrusted_external_content>",
                                tr.content
                            )
                        } else {
                            tr.content
                        };
                        Message {
                            role: Role::Tool,
                            content: vec![ContentPart::ToolResult {
                                tool_use_id: id,
                                content,
                                is_error: tr.is_error,
                            }],
                        }
                    }
                })
                .collect();

            let tool_msgs = futures::future::join_all(tool_futs).await;
            history.extend(tool_msgs);
        }
    }

    fn persist_session(
        &self,
        session_id: &str,
        source: &str,
        started_at: f64,
        history: &[Message],
        result: &AgentResult,
    ) {
        let db = match &self.session_db {
            Some(db) => db.clone(),
            None => return,
        };

        #[allow(clippy::cast_precision_loss)]
        let ended_at = Utc::now().timestamp_millis() as f64 / 1000.0;
        let non_system: Vec<_> = history.iter().filter(|m| m.role != Role::System).collect();
        #[allow(clippy::cast_possible_truncation)]
        let message_count = non_system.len() as u32;

        if let Err(e) = db.save_session(
            session_id,
            source,
            &self.config.model,
            started_at,
            ended_at,
            result.usage.input_tokens,
            result.usage.output_tokens,
            message_count,
        ) {
            warn!("failed to save session: {e}");
        }

        #[allow(clippy::cast_precision_loss)]
        let now = Utc::now().timestamp_millis() as f64 / 1000.0;
        let rows: Vec<(String, String, String, f64)> = non_system
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "tool",
                    Role::System => "system",
                };
                let content = serde_json::to_string(&m.content).unwrap_or_default();
                (Uuid::new_v4().to_string(), role.into(), content, now)
            })
            .collect();

        if let Err(e) = db.append_messages(session_id, &rows) {
            warn!("failed to save messages: {e}");
        }
    }
}
