use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use dashmap::DashMap;
use garudust_agent::{Agent, RolesApprover};
use garudust_core::{
    config::{AgentConfig, RolesConfig},
    platform::{MessageHandler, PlatformAdapter},
    tool::CommandApprover,
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use garudust_memory::SessionDb;

use tokio::sync::Mutex;

use crate::sessions::SessionRegistry;

/// Routes inbound platform messages to an agent and sends the reply back.
pub struct GatewayHandler {
    agent: Arc<Agent>,
    platform: Arc<dyn PlatformAdapter>,
    sessions: Arc<SessionRegistry>,
    /// Global fallback approver used when no roles are configured.
    approver: Arc<dyn CommandApprover>,
    config: Arc<AgentConfig>,
    session_db: Option<Arc<SessionDb>>,
    /// Files awaiting RAG ingest confirmation, keyed by session_key.
    pending_docs: Arc<DashMap<String, Vec<DocAttachment>>>,
    /// Per-session gate held for the full duration of image analysis.
    image_gates: Arc<DashMap<String, Arc<Mutex<()>>>>,
    /// Live, in-memory roles config — updated immediately on every /role change
    /// so the new permissions take effect without a restart.
    pub(crate) live_roles: Arc<RwLock<RolesConfig>>,
    /// Mutex that serialises the bootstrap write so two simultaneous DMs cannot
    /// both see "no admin" and both get promoted.
    bootstrap_lock: Arc<tokio::sync::RwLock<()>>,
}

impl GatewayHandler {
    pub fn new(
        agent: Arc<Agent>,
        platform: Arc<dyn PlatformAdapter>,
        sessions: Arc<SessionRegistry>,
        approver: Arc<dyn CommandApprover>,
        config: Arc<AgentConfig>,
        session_db: Option<Arc<SessionDb>>,
    ) -> Self {
        let live_roles = Arc::new(RwLock::new(config.roles.clone()));
        Self {
            agent,
            platform,
            sessions,
            approver,
            config,
            session_db,
            pending_docs: Arc::new(DashMap::new()),
            image_gates: Arc::new(DashMap::new()),
            live_roles,
            bootstrap_lock: Arc::new(tokio::sync::RwLock::new(())),
        }
    }

    // ── Role helpers ─────────────────────────────────────────────────────────

    /// Build a per-request approver based on the sender's configured role.
    fn approver_for(&self, platform: &str, user_id: &str) -> Arc<dyn CommandApprover> {
        let roles = self.live_roles.read().unwrap();
        RolesApprover::for_user(
            platform,
            user_id,
            None,
            &roles,
            &self.config.security.approval_mode,
            self.agent.tools(),
        )
    }

    /// Handle /whoami, /role … commands. Returns true if the command was handled.
    async fn handle_role_command(&self, msg: &InboundMessage) -> anyhow::Result<bool> {
        let trimmed = msg.text.trim();
        let platform = &msg.channel.platform;
        let user_id = &msg.user_id;

        if trimmed == "/whoami" {
            let role = {
                let roles = self.live_roles.read().unwrap();
                roles
                    .lookup_role(platform, user_id, None)
                    .or_else(|| roles.default_role.clone())
                    .unwrap_or_else(|| "pending".to_string())
            };
            let reply = OutboundMessage::text(format!(
                "id: {platform}:{user_id}\nrole: {role}"
            ));
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(true);
        }

        // All /role sub-commands require admin.
        let is_admin = self
            .live_roles
            .read()
            .unwrap()
            .lookup_role(platform, user_id, None)
            .as_deref()
            == Some("admin");

        if trimmed == "/role list" {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            let lines = {
                let roles = self.live_roles.read().unwrap();
                match roles.users.get(platform.as_str()) {
                    None => "ยังไม่มีผู้ใช้ที่กำหนดสิทธิ์".to_string(),
                    Some(map) => map
                        .iter()
                        .map(|(id, role)| format!("{id} → {role}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                }
            };
            let _ = self
                .platform
                .send_message(&msg.channel, OutboundMessage::text(lines))
                .await;
            return Ok(true);
        }

        // /role approve <platform:id> <role>
        if let Some(rest) = trimmed.strip_prefix("/role approve ") {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            let parts: Vec<&str> = rest.trim().splitn(2, ' ').collect();
            if parts.len() != 2 {
                let _ = self
                    .platform
                    .send_message(
                        &msg.channel,
                        OutboundMessage::text("ใช้: /role approve <platform:id> <role>"),
                    )
                    .await;
                return Ok(true);
            }
            let (target, role_name) = (parts[0], parts[1].trim());
            if let Some((tplatform, tid)) = target.split_once(':') {
                if self.save_role(tplatform, tid, role_name).is_ok() {
                    let ch = garudust_core::types::ChannelId {
                        platform: tplatform.to_string(),
                        chat_id: tid.to_string(),
                        thread_id: None,
                    };
                    let _ = self
                        .platform
                        .send_message(
                            &ch,
                            OutboundMessage::text(format!(
                                "✅ สิทธิ์ของคุณได้รับการอัปเดตเป็น: {role_name}"
                            )),
                        )
                        .await;
                    let _ = self
                        .platform
                        .send_message(
                            &msg.channel,
                            OutboundMessage::text(format!("อนุมัติ {target} เป็น {role_name} แล้ว")),
                        )
                        .await;
                } else {
                    let _ = self
                        .platform
                        .send_message(&msg.channel, OutboundMessage::text("บันทึกสิทธิ์ไม่สำเร็จ"))
                        .await;
                }
            } else {
                let _ = self
                    .platform
                    .send_message(
                        &msg.channel,
                        OutboundMessage::text("รูปแบบ ID ไม่ถูกต้อง ต้องเป็น platform:id เช่น telegram:123456"),
                    )
                    .await;
            }
            return Ok(true);
        }

        // /role deny <platform:id>
        if let Some(rest) = trimmed.strip_prefix("/role deny ") {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            // /role deny now works like /role remove — revokes access and
            // resets the user to the lowest default role on next message.
            let target = rest.trim();
            if let Some((tplatform, tid)) = target.split_once(':') {
                let removed = self.remove_role(tplatform, tid);
                let reply = if removed {
                    format!("เพิกถอนสิทธิ์ {target} แล้ว")
                } else {
                    format!("ไม่พบ {target}")
                };
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text(reply))
                    .await;
            }
            return Ok(true);
        }

        // /role remove <platform:id>
        if let Some(rest) = trimmed.strip_prefix("/role remove ") {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            let target = rest.trim();
            if let Some((tplatform, tid)) = target.split_once(':') {
                let removed = self.remove_role(tplatform, tid);
                let reply = if removed {
                    format!("ลบ {target} ออกแล้ว")
                } else {
                    format!("ไม่พบ {target}")
                };
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text(reply))
                    .await;
            }
            return Ok(true);
        }

        // /role add <platform:id> <role>
        if let Some(rest) = trimmed.strip_prefix("/role add ") {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            let parts: Vec<&str> = rest.trim().splitn(2, ' ').collect();
            if parts.len() != 2 {
                let _ = self
                    .platform
                    .send_message(
                        &msg.channel,
                        OutboundMessage::text("ใช้: /role add <platform:id> <role>"),
                    )
                    .await;
                return Ok(true);
            }
            let (target, role_name) = (parts[0], parts[1].trim());
            if let Some((tplatform, tid)) = target.split_once(':') {
                let reply = if self.save_role(tplatform, tid, role_name).is_ok() {
                    format!("เพิ่ม {target} เป็น {role_name} แล้ว")
                } else {
                    "บันทึกสิทธิ์ไม่สำเร็จ".to_string()
                };
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text(reply))
                    .await;
            }
            return Ok(true);
        }

        Ok(false)
    }

    /// Update a role in-memory and persist to config.yaml atomically.
    fn save_role(&self, platform: &str, user_id: &str, role: &str) -> std::io::Result<()> {
        let mut roles = self.live_roles.write().unwrap();
        roles.set_user_role(platform, user_id, role);
        let mut cfg = (*self.config).clone();
        cfg.roles = roles.clone();
        cfg.save_yaml()
    }

    /// Remove a role in-memory and persist to config.yaml atomically.
    fn remove_role(&self, platform: &str, user_id: &str) -> bool {
        let mut roles = self.live_roles.write().unwrap();
        let removed = roles.remove_user(platform, user_id);
        if removed {
            let mut cfg = (*self.config).clone();
            cfg.roles = roles.clone();
            let _ = cfg.save_yaml();
        }
        removed
    }

    /// Bootstrap: if no admin exists yet and this is a DM, make the sender admin.
    /// Uses a write-lock to prevent two simultaneous DMs from both getting admin.
    async fn maybe_bootstrap_admin(&self, msg: &InboundMessage) -> bool {
        if msg.is_group {
            return false;
        }
        // Fast path without lock: admin already exists.
        if self.live_roles.read().unwrap().has_any_admin() {
            return false;
        }
        let _guard = self.bootstrap_lock.write().await;
        // Re-check under lock (another task may have written by now).
        if self.live_roles.read().unwrap().has_any_admin() {
            return false;
        }
        let platform = &msg.channel.platform;
        let user_id = &msg.user_id;
        if self.save_role(platform, user_id, "admin").is_ok() {
            tracing::info!(platform, user_id, "roles: bootstrap — first DM user promoted to admin");
            let _ = self
                .platform
                .send_message(
                    &msg.channel,
                    OutboundMessage::text(
                        "🎉 คุณได้รับสิทธิ์ admin เนื่องจากเป็นผู้ใช้คนแรก\n\
                         ใช้ /role approve <platform:id> <role> เพื่อเพิ่มผู้ใช้คนอื่น",
                    ),
                )
                .await;
            return true;
        }
        false
    }

    /// On startup, re-run any agent tasks that were interrupted by a server crash or restart.
    /// Tasks are stored in SQLite before the agent run begins and deleted on completion.
    pub fn resume_pending(&self) {
        let db = match &self.session_db {
            Some(db) => db.clone(),
            None => return,
        };
        let platform_name = self.platform.name().to_string();
        let tasks = match db.drain_tasks(&platform_name) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(platform = %platform_name, "drain_tasks failed: {e}");
                return;
            }
        };
        if tasks.is_empty() {
            return;
        }
        tracing::info!(
            count = tasks.len(),
            platform = %platform_name,
            "resuming interrupted tasks after restart"
        );
        for pending in tasks {
            let channel = ChannelId {
                platform: pending.platform.clone(),
                chat_id: pending.chat_id.clone(),
                thread_id: None,
            };
            let agent = self.agent.clone();
            let platform = self.platform.clone();
            let approver = self.approver.clone();
            let session_db = self.session_db.clone();
            let task_id = pending.id.clone();
            let session_key = pending.session_key.clone();
            let task = pending.task.clone();
            let hint = pending.hint.clone();
            let pname = platform_name.clone();
            tokio::spawn(async move {
                match agent
                    .run(&task, approver, &pname, hint.as_deref(), Some(&session_key))
                    .await
                {
                    Ok(result) => {
                        if let Some(db) = &session_db {
                            let _ = db.finish_task(&task_id);
                        }
                        let _ = platform
                            .send_message(&channel, OutboundMessage::markdown(result.output))
                            .await;
                    }
                    Err(e) => {
                        if let Some(db) = &session_db {
                            let _ = db.finish_task(&task_id);
                        }
                        tracing::warn!(task_id = %task_id, "resumed task failed: {e}");
                    }
                }
            });
        }
    }

    /// Get-or-create the per-session image-analysis gate. One `Mutex` per
    /// session; the count is bounded by the number of distinct sessions.
    fn image_gate(&self, session_key: &str) -> Arc<Mutex<()>> {
        self.image_gates
            .entry(session_key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
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
        // Held for the whole analysis so a follow-up text question on the
        // same session blocks until every description is in history.
        let gate = self.image_gate(session_key);
        let _gate_guard = gate.lock().await;

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

        // Whitelist: silently drop messages from unlisted users (legacy allowed_user_ids).
        if !pcfg.allowed_user_ids.is_empty() && !pcfg.allowed_user_ids.contains(&msg.user_id) {
            tracing::debug!(user_id = %msg.user_id, "message dropped: user not in whitelist");
            return Ok(());
        }

        // Bootstrap: first DM user becomes admin automatically (no-op if admin exists).
        self.maybe_bootstrap_admin(&msg).await;

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

        // Role check: handle /whoami and /role commands before anything else
        // (including the mention gate) so they always work.
        if self.handle_role_command(&msg).await? {
            return Ok(());
        }

        // Auto-assign: new user with no role gets the lowest defined role immediately.
        {
            let (has_roles_configured, needs_assign) = {
                let roles = self.live_roles.read().unwrap();
                let configured = !roles.definitions.is_empty() || roles.default_role.is_some();
                let has_role = roles.lookup_role(&msg.channel.platform, &msg.user_id, None).is_some();
                let effective = has_role || roles.default_role.is_some();
                (configured, configured && !effective)
            };
            if has_roles_configured && needs_assign {
                let lowest = self.live_roles.read().unwrap().effective_default_role();
                if let Some(role) = lowest {
                    let _ = self.save_role(&msg.channel.platform, &msg.user_id, &role);
                    tracing::info!(
                        platform = %msg.channel.platform,
                        user_id  = %msg.user_id,
                        role     = %role,
                        "roles: auto-assigned default role to new user"
                    );
                }
            }
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
                    self.approver_for(&msg.channel.platform, &msg.user_id),
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
                        // Native detection unavailable and no bot_username to
                        // match against — we genuinely cannot tell if the bot
                        // was addressed. The operator explicitly set
                        // require_mention, so fail closed (stay silent) rather
                        // than reply to every group message. This window is
                        // rare: the LINE adapter caches/retries/lazily
                        // re-fetches its userId so `bot_mentioned` is Some.
                        false
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
        if trimmed == "/cleargoal" {
            self.agent.clear_goal(&msg.session_key).await;
            let reply = OutboundMessage::text("ล้าง goal แล้ว");
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(());
        }
        if trimmed == "/goal" {
            let reply = match self.agent.get_goal(&msg.session_key).await {
                Some(g) => OutboundMessage::text(format!("Goal ปัจจุบัน:\n{g}")),
                None => OutboundMessage::text("ยังไม่มี goal — ตั้งด้วย /goal <เป้าหมาย>"),
            };
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(());
        }
        if let Some(goal_text) = trimmed.strip_prefix("/goal ") {
            let goal_text = goal_text.trim();
            if goal_text.is_empty() {
                let reply = OutboundMessage::text("ใช้: /goal <เป้าหมาย>");
                let _ = self.platform.send_message(&msg.channel, reply).await;
            } else {
                self.agent.set_goal(&msg.session_key, goal_text).await?;
                let reply = OutboundMessage::text(format!("บันทึก goal แล้ว:\n{goal_text}"));
                let _ = self.platform.send_message(&msg.channel, reply).await;
            }
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
        let approver = self.approver_for(&msg.channel.platform, &msg.user_id);
        let platform_name = msg.channel.platform.clone();
        let user_id = msg.user_id.clone();
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

        // A question about an image arrives as a separate platform event from
        // the image itself. If that image is still being analysed, wait for
        // its description to land in history before answering — otherwise the
        // model sees only the "[analysing…]" placeholder and wrongly reports
        // it cannot read the image. `.get()` (not get-or-create) avoids
        // allocating a gate for pure-text sessions.
        if let Some(gate) = self
            .image_gates
            .get(&msg.session_key)
            .map(|g| Arc::clone(g.value()))
        {
            drop(gate.lock().await);
        }

        let task = msg.text.clone();

        // Journal the task to SQLite before spawning — if the server crashes
        // mid-run, `drain_tasks` on the next startup will replay it.
        let task_id = uuid::Uuid::new_v4().to_string();
        if let Some(db) = &self.session_db {
            if let Err(e) = db.begin_task(
                &task_id,
                &session_key,
                &platform_name,
                &channel.chat_id,
                &task,
                None,
            ) {
                tracing::warn!(task_id = %task_id, "begin_task failed: {e}");
            }
        }
        let session_db = self.session_db.clone();

        tokio::spawn(async move {
            match agent
                .run_for_user(&task, approver, &platform_name, None, Some(&session_key), &user_id)
                .await
            {
                Ok(result) => {
                    if let Some(db) = &session_db {
                        let _ = db.finish_task(&task_id);
                    }
                    let reply = OutboundMessage::markdown(result.output);
                    if let Err(e) = platform.send_message(&channel, reply).await {
                        tracing::error!("send_message failed: {e}");
                    }
                }
                Err(e) => {
                    if let Some(db) = &session_db {
                        let _ = db.finish_task(&task_id);
                    }
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
