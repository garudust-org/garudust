use std::sync::Arc;

use async_trait::async_trait;
use dashmap::DashMap;
use garudust_agent::Agent;
use garudust_core::{
    config::AgentConfig,
    platform::{MessageHandler, PlatformAdapter},
    tool::CommandApprover,
    types::{DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};

use crate::sessions::SessionRegistry;

/// Routes inbound platform messages to an agent and sends the reply back.
pub struct GatewayHandler {
    agent: Arc<Agent>,
    platform: Arc<dyn PlatformAdapter>,
    sessions: Arc<SessionRegistry>,
    approver: Arc<dyn CommandApprover>,
    config: Arc<AgentConfig>,
    /// Files awaiting RAG ingest confirmation, keyed by session_key.
    /// Populated when a doc-only message arrives; consumed on the next text turn.
    pending_docs: Arc<DashMap<String, Vec<DocAttachment>>>,
}

impl GatewayHandler {
    pub fn new(
        agent: Arc<Agent>,
        platform: Arc<dyn PlatformAdapter>,
        sessions: Arc<SessionRegistry>,
        approver: Arc<dyn CommandApprover>,
        config: Arc<AgentConfig>,
    ) -> Self {
        Self {
            agent,
            platform,
            sessions,
            approver,
            config,
            pending_docs: Arc::new(DashMap::new()),
        }
    }

    /// Analyse image attachments via the registered view_image tool and inject
    /// the descriptions into conversation history.
    async fn process_images(
        &self,
        attachments: &[ImageAttachment],
        session_key: &str,
        seq_start: usize,
        user_name: &str,
    ) {
        let view_image_installed = self.agent.has_tool("view_image");

        for (i, att) in attachments.iter().enumerate() {
            let seq = seq_start + i + 1;
            let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
            let user_label = if user_name.is_empty() {
                format!("[รูปที่ {seq} เวลา {ts}]")
            } else {
                format!("[@{user_name} ส่งรูปที่ {seq} เวลา {ts}]")
            };

            // Inject label immediately so the sender is visible in history
            // even if view_image takes several seconds to complete.
            let placeholder = if view_image_installed {
                "[กำลังวิเคราะห์ภาพ...]".to_string()
            } else {
                "[รูปภาพ — ติดตั้ง view_image tool เพื่อให้วิเคราะห์ได้: garudust tool install view_image]"
                    .to_string()
            };
            self.agent
                .inject_history(session_key, &user_label, &placeholder);

            // Run view_image and replace the placeholder once done.
            if view_image_installed {
                let description = self
                    .agent
                    .run_tool(
                        "view_image",
                        serde_json::json!({
                            "source": att.path,
                            "question": "อธิบายรูปนี้โดยละเอียด"
                        }),
                    )
                    .await;
                self.agent.update_last_history(session_key, &description);
            }

            // Clean up temp file
            if att.path.starts_with("/tmp/") {
                let _ = tokio::fs::remove_file(&att.path).await;
            }
        }
    }

    /// Inject received document info into history (with file path so the agent
    /// can later ingest on confirmation) and spawn an agent turn that asks the
    /// user whether they want the file ingested into the RAG store.  The LLM
    /// generates the question in whichever language the conversation is in.
    async fn process_docs(
        &self,
        attachments: &[DocAttachment],
        session_key: &str,
        user_name: &str,
        channel: &garudust_core::types::ChannelId,
        approver: Arc<dyn CommandApprover>,
    ) {
        if attachments.is_empty() {
            return;
        }

        let rag_enabled = self.agent.has_tool("doc_ingest");

        for att in attachments {
            let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
            let user_label = if user_name.is_empty() {
                format!("[ส่งไฟล์ {} เวลา {ts}]", att.file_name)
            } else {
                format!("[@{user_name} ส่งไฟล์ {} เวลา {ts}]", att.file_name)
            };

            let note = if rag_enabled {
                "[รอการยืนยันจากผู้ใช้ว่าต้องการให้นำเข้าไฟล์นี้หรือไม่]".to_string()
            } else {
                "[RAG ยังไม่ได้เปิดใช้งาน — ลบ 'rag' ออกจาก disabled_toolsets ใน config.yaml]"
                    .to_string()
            };
            self.agent.inject_history(session_key, &user_label, &note);
        }

        // Store attachments so the next text turn can pass the exact paths to
        // doc_ingest without relying on the LLM to extract them from history.
        if rag_enabled {
            self.pending_docs
                .insert(session_key.to_string(), attachments.to_vec());
        }

        if !rag_enabled {
            // No point asking; inform directly.
            let names = attachments
                .iter()
                .map(|a| a.file_name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let reply = OutboundMessage::text(format!(
                "รับไฟล์แล้ว: {names}\n\nRAG ยังไม่ได้เปิดใช้งาน — ลบ \"rag\" ออกจาก \
                 disabled_toolsets ใน config.yaml เพื่อให้บอทอ่านเนื้อหาไฟล์ได้"
            ));
            let _ = self.platform.send_message(channel, reply).await;
            // Clean up temp files
            for att in attachments {
                if att.path.starts_with("/tmp/") {
                    let _ = tokio::fs::remove_file(&att.path).await;
                }
            }
            return;
        }

        // Ask via agent LLM so the question is in the same language as the
        // ongoing conversation.
        let names = attachments
            .iter()
            .map(|a| a.file_name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let task = format!(
            "[SYSTEM] ผู้ใช้ส่งไฟล์เอกสาร: {names} \
             ถามผู้ใช้ว่าต้องการให้บอทอ่านและจำเนื้อหาไฟล์นี้ไว้หรือไม่ \
             ถามสั้นๆ ในภาษาเดียวกับที่ผู้ใช้ใช้ล่าสุดในการสนทนา"
        );

        let agent = self.agent.clone();
        let platform = self.platform.clone();
        let channel = channel.clone();
        let session_key = session_key.to_string();
        let platform_name = channel.platform.clone();

        tokio::spawn(async move {
            match agent
                .run(&task, approver, &platform_name, None, Some(&session_key))
                .await
            {
                Ok(result) => {
                    let _ = platform
                        .send_message(&channel, OutboundMessage::markdown(result.output))
                        .await;
                }
                Err(e) => {
                    let _ = platform
                        .send_message(&channel, OutboundMessage::text(format!("Error: {e}")))
                        .await;
                }
            }
        });
    }
}

#[async_trait]
impl MessageHandler for GatewayHandler {
    async fn handle(&self, mut msg: InboundMessage) -> Result<(), anyhow::Error> {
        let pcfg = &self.config.platform;

        // Whitelist: silently drop messages from unlisted users
        if !pcfg.allowed_user_ids.is_empty() && !pcfg.allowed_user_ids.contains(&msg.user_id) {
            tracing::debug!(user_id = %msg.user_id, "message dropped: user not in whitelist");
            return Ok(());
        }

        // Per-user session isolation — only for non-group (DM) chats.
        // In group chats every member shares one session so that images sent by
        // any member are visible to everyone who later asks about them.
        // Applying per-user keys in groups would mean image events land in the
        // sender's session bucket while a different member's follow-up text query
        // reads from their own bucket and sees nothing.
        if pcfg.session_per_user && !msg.is_group && msg.channel.platform != "webhook" {
            msg.session_key = format!(
                "{}:{}:{}",
                msg.channel.platform, msg.channel.chat_id, msg.user_id
            );
        }

        // Image-only or doc-only messages bypass the mention gate.
        if msg.text.trim().is_empty()
            && (!msg.attachments.is_empty() || !msg.doc_attachments.is_empty())
        {
            self.sessions
                .touch(&msg.session_key, &msg.channel.platform, &msg.user_id)
                .await;

            if !msg.attachments.is_empty() {
                self.process_images(&msg.attachments, &msg.session_key, 0, &msg.user_name)
                    .await;
            }
            if !msg.doc_attachments.is_empty() {
                // process_docs spawns the agent to ask confirmation — no extra
                // return value needed; the spawned task sends the reply.
                self.process_docs(
                    &msg.doc_attachments,
                    &msg.session_key,
                    &msg.user_name,
                    &msg.channel,
                    self.approver.clone(),
                )
                .await;
            }
            return Ok(());
        }

        // Mention gate: in group chats only respond when @mentioned.
        if pcfg.require_mention && msg.is_group {
            let mentioned = match msg.bot_mentioned {
                Some(b) => b,
                None => {
                    if pcfg.bot_username.is_empty() {
                        true
                    } else {
                        let mention = format!("@{}", pcfg.bot_username);
                        msg.text.to_lowercase().contains(&mention.to_lowercase())
                    }
                }
            };
            if !mentioned {
                return Ok(());
            }
        }

        self.sessions
            .touch(&msg.session_key, &msg.channel.platform, &msg.user_id)
            .await;

        // Strip @bot_username prefix from group mentions
        if !pcfg.bot_username.is_empty() {
            let mention = format!("@{}", pcfg.bot_username);
            let lower = msg.text.to_lowercase();
            if let Some(_rest) = lower.strip_prefix(&mention.to_lowercase()) {
                msg.text = msg.text[mention.len()..].trim().to_string();
            }
        }

        // Slash commands — handled synchronously before spawning
        let trimmed = msg.text.trim();
        if trimmed == "/new" || trimmed == "/clear" {
            self.agent.clear_session(&msg.session_key);
            let reply = OutboundMessage::text("เริ่มการสนทนาใหม่แล้ว");
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(());
        }

        // Process any image or document attachments that come alongside text
        if !msg.attachments.is_empty() {
            self.process_images(&msg.attachments, &msg.session_key, 0, &msg.user_name)
                .await;
        }
        // Doc attachments alongside text: inject into history only (the user's
        // text message is the "confirmation" if they explicitly ask to ingest).
        if !msg.doc_attachments.is_empty() {
            for att in &msg.doc_attachments {
                let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
                let user_label = if msg.user_name.is_empty() {
                    format!("[ส่งไฟล์ {} เวลา {ts}]", att.file_name)
                } else {
                    format!("[@{} ส่งไฟล์ {} เวลา {ts}]", msg.user_name, att.file_name)
                };
                let note = format!("[ไฟล์บันทึกชั่วคราวที่ {} — รอการยืนยันจากผู้ใช้]", att.path);
                self.agent
                    .inject_history(&msg.session_key, &user_label, &note);
            }
        }

        let channel = msg.channel.clone();
        let agent = self.agent.clone();
        let platform = self.platform.clone();
        let approver = self.approver.clone();
        let platform_name = msg.channel.platform.clone();
        let session_key = msg.session_key.clone();

        // If files are waiting for RAG confirmation, handle this turn
        // deterministically: ingest (or cancel), clean up the temp file, and
        // reply directly — then return. We must NOT hand the raw confirmation
        // to the LLM: history only ever contains the friendly filename (the
        // real /tmp path lives in `pending_docs`, never in history), so the
        // LLM would re-attempt doc_ingest with a bare filename and fail the
        // allowed-paths check, reporting a false "outside allowed read
        // directories" error even though ingestion already succeeded.
        if let Some((_, pending)) = self.pending_docs.remove(&msg.session_key) {
            if !pending.is_empty() {
                let text_lower = msg.text.to_lowercase();
                let is_negative = [
                    "ไม่ต้องการ",
                    "ไม่เอา",
                    "ไม่ต้อง",
                    "ยกเลิก",
                    "no\n",
                    " no ",
                    "nope",
                    "cancel",
                    "不要",
                ]
                .iter()
                .any(|n| text_lower.contains(n))
                    || text_lower.trim() == "ไม่"
                    || text_lower.trim() == "no";

                let reply_text = if is_negative {
                    let names = pending
                        .iter()
                        .map(|a| a.file_name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    for att in &pending {
                        if att.path.starts_with("/tmp/") {
                            let _ = tokio::fs::remove_file(&att.path).await;
                        }
                    }
                    self.agent.inject_history(
                        &msg.session_key,
                        "[ระบบ]",
                        "[ผู้ใช้ยกเลิก — ไม่มีการนำเข้าไฟล์]",
                    );
                    format!("รับทราบ ไม่ได้นำเข้าไฟล์ {names} เข้าระบบ")
                } else {
                    // Ingest each file directly with the correct conv_key so the
                    // document lands in the right per-chat RAG bucket.
                    let mut lines = Vec::with_capacity(pending.len());
                    for att in &pending {
                        let result = self
                            .agent
                            .run_tool_scoped(
                                "doc_ingest",
                                serde_json::json!({"path": att.path}),
                                &msg.session_key,
                            )
                            .await;
                        self.agent.inject_history(
                            &msg.session_key,
                            "[ระบบ]",
                            &format!("[นำเข้าไฟล์ {} แล้ว: {}]", att.file_name, result),
                        );
                        lines.push(summarize_ingest(&att.file_name, &result));
                        if att.path.starts_with("/tmp/") {
                            let _ = tokio::fs::remove_file(&att.path).await;
                        }
                    }
                    lines.join("\n")
                };

                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text(reply_text))
                    .await;
                return Ok(());
            }
        }

        let task = msg.text.clone();

        tokio::spawn(async move {
            match agent
                .run(&task, approver, &platform_name, None, Some(&session_key))
                .await
            {
                Ok(result) => {
                    let reply = OutboundMessage::markdown(result.output);
                    if let Err(e) = platform.send_message(&channel, reply).await {
                        tracing::error!("send_message failed: {e}");
                    }
                }
                Err(e) => {
                    let reply = OutboundMessage::text(format!("Error: {e}"));
                    let _ = platform.send_message(&channel, reply).await;
                }
            }
        });

        Ok(())
    }
}

/// Turn a raw `doc_ingest` tool result into a user-facing Thai status line.
/// Success content is the tool's JSON (`{"chunks_indexed":N,...}`); empty docs
/// return a plain "empty" string; failures arrive as `[doc_ingest failed: …]`.
fn summarize_ingest(file_name: &str, result: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(result) {
        if let Some(n) = v.get("chunks_indexed").and_then(serde_json::Value::as_u64) {
            return format!(
                "นำเข้าไฟล์ \"{file_name}\" เรียบร้อยแล้ว ({n} ส่วน) — ถามเกี่ยวกับเนื้อหาไฟล์นี้ได้เลย"
            );
        }
    }
    if result.contains("empty") {
        return format!("ไฟล์ \"{file_name}\" ว่างเปล่า ไม่มีเนื้อหาให้นำเข้า");
    }
    format!("นำเข้าไฟล์ \"{file_name}\" ไม่สำเร็จ: {}", result.trim())
}

#[cfg(test)]
mod tests {
    use super::summarize_ingest;

    #[test]
    fn summarize_success_reports_chunk_count() {
        let r = r#"{"file":"a.md","chunks_indexed":12,"preview":"x"}"#;
        let s = summarize_ingest("a.md", r);
        assert!(s.contains("เรียบร้อยแล้ว"));
        assert!(s.contains("12 ส่วน"));
    }

    #[test]
    fn summarize_empty_doc() {
        let s = summarize_ingest("a.md", "Document is empty — nothing ingested.");
        assert!(s.contains("ว่างเปล่า"));
    }

    #[test]
    fn summarize_failure_passes_error_through() {
        let s = summarize_ingest(
            "a.md",
            "[doc_ingest failed: path 'a.md' is outside allowed read directories]",
        );
        assert!(s.contains("ไม่สำเร็จ"));
        assert!(s.contains("outside allowed read directories"));
    }
}
