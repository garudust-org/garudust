use std::sync::Arc;

use async_trait::async_trait;
use garudust_agent::Agent;
use garudust_core::{
    config::{get_secret, AgentConfig},
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

        // The view_image script reads OPENROUTER_API_KEY / GOOGLE_AI_API_KEY from
        // its env. Since the server no longer loads ~/.garudust/.env into the
        // process environment, we forward the relevant keys explicitly here —
        // mirroring the env_required allowlist used by the regular tool dispatch.
        let or_key = get_secret("OPENROUTER_API_KEY");
        let gm_key = get_secret("GOOGLE_AI_API_KEY");

        // `uv` is typically installed at ~/.local/bin/uv which is NOT in the
        // server's PATH when launched as a daemon (PATH inherits from the
        // launching shell — desktop entries / systemd units often strip it).
        // Augment PATH so `Command::new("uv")` resolves regardless of how the
        // server was started.
        let augmented_path = {
            let cur = std::env::var("PATH").unwrap_or_default();
            match std::env::var("HOME") {
                Ok(home) if !home.is_empty() => {
                    let extra = format!("{home}/.local/bin");
                    if cur.split(':').any(|seg| seg == extra) {
                        cur
                    } else {
                        format!("{extra}:{cur}")
                    }
                }
                _ => cur,
            }
        };

        for (i, att) in attachments.iter().enumerate() {
            let seq = seq_start + i + 1;
            let description = if script.exists() {
                let mut cmd = tokio::process::Command::new("uv");
                cmd.args([
                    "run",
                    "--with",
                    "httpx",
                    script.to_str().unwrap_or("run.py"),
                    &att.path,
                    "อธิบายรูปนี้โดยละเอียด",
                ]);
                cmd.env("PATH", &augmented_path);
                if let Some(v) = &or_key {
                    cmd.env("OPENROUTER_API_KEY", v);
                }
                if let Some(v) = &gm_key {
                    cmd.env("GOOGLE_AI_API_KEY", v);
                }
                match cmd.output().await {
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

            let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
            let user_label = format!("[รูปที่ {seq} — {ts}]");
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
