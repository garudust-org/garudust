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

    /// Analyse one image attachment with the view_image hub tool and inject the
    /// description into the conversation history.  Returns true if at least one
    /// image was successfully stored.
    async fn process_images(
        &self,
        attachments: &[ImageAttachment],
        session_key: &str,
        seq_start: usize,
    ) {
        let script = self
            .config
            .home_dir
            .join("tools")
            .join("view_image")
            .join("run.py");

        for (i, att) in attachments.iter().enumerate() {
            let seq = seq_start + i + 1;
            let description = if script.exists() {
                match tokio::process::Command::new("uv")
                    .args([
                        "run",
                        "--with",
                        "httpx",
                        script.to_str().unwrap_or("run.py"),
                        &att.path,
                        "อธิบายรูปนี้โดยละเอียด",
                    ])
                    .output()
                    .await
                {
                    Ok(out) if out.status.success() => {
                        String::from_utf8_lossy(&out.stdout).trim().to_string()
                    }
                    Ok(out) => {
                        let err = String::from_utf8_lossy(&out.stderr);
                        tracing::warn!(path = %att.path, err = %err, "view_image failed");
                        "[ไม่สามารถวิเคราะห์รูปได้]".to_string()
                    }
                    Err(e) => {
                        tracing::warn!(path = %att.path, error = %e, "view_image exec error");
                        "[ไม่สามารถวิเคราะห์รูปได้]".to_string()
                    }
                }
            } else {
                "[รูปภาพ — ติดตั้ง view_image tool เพื่อให้วิเคราะห์ได้: garudust tool install view_image]"
                    .to_string()
            };

            let user_label = format!("[รูปที่ {seq}]");
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

        // Image-only messages bypass the mention gate (e.g. LINE where you
        // cannot @mention inside an image event) and are stored silently.
        if msg.text.trim().is_empty() && !msg.attachments.is_empty() {
            self.sessions
                .touch(&msg.session_key, &msg.channel.platform, &msg.user_id)
                .await;
            self.process_images(&msg.attachments, &msg.session_key, 0)
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

        // Per-user session isolation
        if pcfg.session_per_user && msg.channel.platform != "webhook" {
            msg.session_key = format!(
                "{}:{}:{}",
                msg.channel.platform, msg.channel.chat_id, msg.user_id
            );
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
            self.process_images(&msg.attachments, &msg.session_key, 0)
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
                .run(&task, approver, &platform_name, Some(&session_key))
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
