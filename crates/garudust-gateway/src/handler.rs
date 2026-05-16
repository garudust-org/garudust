use std::sync::Arc;

use async_trait::async_trait;
use garudust_agent::Agent;
use garudust_core::{
    config::AgentConfig,
    platform::{MessageHandler, PlatformAdapter},
    tool::CommandApprover,
    types::{ImageAttachment, InboundMessage, OutboundMessage},
};

use crate::sessions::SessionRegistry;

/// Routes inbound platform messages to an agent and sends the reply back.
pub struct GatewayHandler {
    agent: Arc<Agent>,
    platform: Arc<dyn PlatformAdapter>,
    sessions: Arc<SessionRegistry>,
    approver: Arc<dyn CommandApprover>,
    config: Arc<AgentConfig>,
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
            let description = if view_image_installed {
                self.agent
                    .run_tool(
                        "view_image",
                        serde_json::json!({
                            "source": att.path,
                            "question": "อธิบายรูปนี้โดยละเอียด"
                        }),
                    )
                    .await
            } else {
                "[รูปภาพ — ติดตั้ง view_image tool เพื่อให้วิเคราะห์ได้: garudust tool install view_image]"
                    .to_string()
            };

            let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
            let user_label = if user_name.is_empty() {
                format!("[รูปที่ {seq} เวลา {ts}]")
            } else {
                format!("[@{user_name} ส่งรูปที่ {seq} เวลา {ts}]")
            };
            self.agent
                .inject_history(session_key, &user_label, &description);

            // Clean up temp file
            if att.path.starts_with("/tmp/") {
                let _ = tokio::fs::remove_file(&att.path).await;
            }
        }
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

        // Per-user session isolation — must run BEFORE both the image-only and
        // image+text branches so the silent image description is injected into
        // the same session bucket the agent will later read from. Otherwise an
        // image-only event gets stored under `line:{chat_id}` while a follow-up
        // text mention from the same user is rewritten to
        // `line:{chat_id}:{user_id}` and the agent sees no image.
        if pcfg.session_per_user && msg.channel.platform != "webhook" {
            msg.session_key = format!(
                "{}:{}:{}",
                msg.channel.platform, msg.channel.chat_id, msg.user_id
            );
        }

        // Image-only messages bypass the mention gate (e.g. LINE where you
        // cannot @mention inside an image event) and are stored silently.
        if msg.text.trim().is_empty() && !msg.attachments.is_empty() {
            self.sessions
                .touch(&msg.session_key, &msg.channel.platform, &msg.user_id)
                .await;
            self.process_images(&msg.attachments, &msg.session_key, 0, &msg.user_name)
                .await;
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

        // If the message has images alongside text, process them first (silent)
        if !msg.attachments.is_empty() {
            self.process_images(&msg.attachments, &msg.session_key, 0, &msg.user_name)
                .await;
        }

        let channel = msg.channel.clone();
        let agent = self.agent.clone();
        let platform = self.platform.clone();
        let approver = self.approver.clone();
        let task = msg.text.clone();
        let platform_name = msg.channel.platform.clone();
        let session_key = msg.session_key.clone();

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
