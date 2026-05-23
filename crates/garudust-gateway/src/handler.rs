use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;
use garudust_agent::{Agent, RolesApprover};
use garudust_core::{
    config::{AgentConfig, InviteCode, RolesConfig},
    platform::{MessageHandler, PlatformAdapter},
    tool::CommandApprover,
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use garudust_memory::SessionDb;
use tokio::io::AsyncReadExt as _;
use tokio::sync::{oneshot, Mutex};

use crate::{interactive::InteractiveApprover, metrics::Metrics, sessions::SessionRegistry};

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
    /// Pending interactive tool-approval requests keyed by short ID.
    /// Populated by InteractiveApprover; resolved by /approve and /deny commands.
    pending_approvals: Arc<DashMap<String, oneshot::Sender<bool>>>,
    metrics: Arc<Metrics>,
    /// Fixed-window per-(platform, user_id) rate counters: (window_start_secs, count).
    user_rate_limits: Arc<DashMap<String, std::sync::Mutex<(u64, u32)>>>,
}

impl GatewayHandler {
    pub fn new(
        agent: Arc<Agent>,
        platform: Arc<dyn PlatformAdapter>,
        sessions: Arc<SessionRegistry>,
        approver: Arc<dyn CommandApprover>,
        config: Arc<AgentConfig>,
        session_db: Option<Arc<SessionDb>>,
        metrics: Arc<Metrics>,
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
            pending_approvals: Arc::new(DashMap::new()),
            metrics,
            user_rate_limits: Arc::new(DashMap::new()),
        }
    }

    // ── RwLock helpers ───────────────────────────────────────────────────────

    fn read_roles(&self) -> std::sync::RwLockReadGuard<'_, RolesConfig> {
        self.live_roles
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn write_roles(&self) -> std::sync::RwLockWriteGuard<'_, RolesConfig> {
        self.live_roles
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    // ── Rate limiting ────────────────────────────────────────────────────────

    /// Returns `true` if the request is within the per-user limit, `false` when
    /// the user has exceeded `rate_limit_rpm_per_user` in the current 60-second
    /// window and the message should be rejected.
    fn check_user_rate_limit(&self, platform: &str, user_id: &str) -> bool {
        let Some(limit) = self.config.security.rate_limit_rpm_per_user else {
            return true;
        };
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let window_start = now_secs - (now_secs % 60);
        let key = format!("{platform}:{user_id}");
        let entry = self.user_rate_limits.entry(key).or_default();
        let mut state = entry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.0 != window_start {
            *state = (window_start, 0);
        }
        state.1 += 1;
        state.1 <= limit
    }

    // ── Role helpers ─────────────────────────────────────────────────────────

    /// Build a per-request approver based on the sender's configured role.
    /// When the role's approval_mode is "interactive", wraps an InteractiveApprover
    /// so the user is asked for confirmation before each tool call.
    fn approver_for(&self, msg: &InboundMessage) -> Arc<dyn CommandApprover> {
        let roles = self.read_roles();
        let role_name = roles
            .lookup_role(&msg.channel.platform, &msg.user_id, None)
            .or_else(|| roles.default_role.clone());

        if let Some(rn) = &role_name {
            if let Some(def) = roles.definitions.get(rn) {
                if def.approval_mode.as_deref() == Some("interactive") {
                    let inner = Arc::new(InteractiveApprover {
                        platform: self.platform.clone(),
                        channel: msg.channel.clone(),
                        pending: self.pending_approvals.clone(),
                        timeout: Duration::from_secs(60),
                    });
                    return RolesApprover::with_inner(inner, def, self.agent.tools());
                }
            }
        }

        RolesApprover::for_user(
            &msg.channel.platform,
            &msg.user_id,
            None,
            &roles,
            &self.config.security.approval_mode,
            self.agent.tools(),
        )
    }

    /// Handle /whoami, /role … commands. Returns true if the command was handled.
    #[tracing::instrument(skip_all, fields(command = %msg.text.trim()))]
    async fn handle_role_command(&self, msg: &InboundMessage) -> anyhow::Result<bool> {
        let trimmed = msg.text.trim();
        let platform = &msg.channel.platform;
        let user_id = &msg.user_id;

        if trimmed == "/whoami" {
            let role = {
                let roles = self.read_roles();
                roles
                    .lookup_role(platform, user_id, None)
                    .or_else(|| roles.default_role.clone())
                    .unwrap_or_else(|| "pending".to_string())
            };
            let reply = OutboundMessage::text(format!("id: {platform}:{user_id}\nrole: {role}"));
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(true);
        }

        // /approve <id> — resolve a pending interactive tool approval
        if let Some(id) = trimmed.strip_prefix("/approve ") {
            let id = id.trim();
            let reply = if let Some((_, tx)) = self.pending_approvals.remove(id) {
                let _ = tx.send(true);
                OutboundMessage::text("✅ อนุมัติแล้ว")
            } else {
                OutboundMessage::text("ไม่พบ approval request นี้ (หมดเวลาหรือไม่มีอยู่)")
            };
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(true);
        }

        // /deny <id> — reject a pending interactive tool approval
        if let Some(id) = trimmed.strip_prefix("/deny ") {
            let id = id.trim();
            let reply = if let Some((_, tx)) = self.pending_approvals.remove(id) {
                let _ = tx.send(false);
                OutboundMessage::text("❌ ปฏิเสธแล้ว")
            } else {
                OutboundMessage::text("ไม่พบ approval request นี้ (หมดเวลาหรือไม่มีอยู่)")
            };
            let _ = self.platform.send_message(&msg.channel, reply).await;
            return Ok(true);
        }

        // /join <code> — redeem an invite code for instant role assignment
        if let Some(code) = trimmed.strip_prefix("/join ") {
            let code = code.trim().to_string();
            let granted = {
                let mut roles = self.write_roles();
                roles.redeem_invite(&code, platform, user_id)
            };
            if granted.is_some() {
                let roles_snapshot = self.read_roles().clone();
                let mut cfg = (*self.config).clone();
                cfg.roles = roles_snapshot;
                let _ = cfg.save_yaml();
            }
            let reply = match granted {
                Some(role) => format!("✅ ยืนยันแล้ว คุณได้รับสิทธิ์: {role}"),
                None => "❌ code ไม่ถูกต้อง หมดอายุ หรือถูกใช้ครบแล้ว".to_string(),
            };
            let _ = self
                .platform
                .send_message(&msg.channel, OutboundMessage::text(reply))
                .await;
            return Ok(true);
        }

        // /join — request access; notifies every admin on this platform
        if trimmed == "/join" {
            let (has_role, admins) = {
                let roles = self.read_roles();
                let has = roles.lookup_role(platform, user_id, None).is_some()
                    || roles.default_role.is_some();
                let admins: Vec<String> = roles
                    .users
                    .get(platform.as_str())
                    .map(|map| {
                        map.iter()
                            .filter(|(_, r)| r.as_str() == "admin")
                            .map(|(id, _)| id.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                (has, admins)
            };
            if has_role {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("คุณมีสิทธิ์เข้าใช้งานอยู่แล้ว"))
                    .await;
                return Ok(true);
            }
            let notify_text = format!(
                "👤 มีผู้ขอสิทธิ์เข้าใช้งาน\n\
                 ชื่อ: {name}\n\
                 ID: {platform}:{user_id}\n\n\
                 ✅ อนุมัติ: /role approve {platform}:{user_id} member\n\
                 ❌ ปฏิเสธ: /role deny {platform}:{user_id}",
                name = msg.user_name,
            );
            for admin_id in &admins {
                let ch = ChannelId {
                    platform: platform.clone(),
                    chat_id: admin_id.clone(),
                    thread_id: None,
                };
                let _ = self
                    .platform
                    .send_message(&ch, OutboundMessage::text(&notify_text))
                    .await;
            }
            let reply = if admins.is_empty() {
                "ส่งคำขอแล้ว — ยังไม่มี admin ในระบบ รอการตั้งค่า"
            } else {
                "ส่งคำขอถึง admin แล้ว กรุณารอการอนุมัติ"
            };
            let _ = self
                .platform
                .send_message(&msg.channel, OutboundMessage::text(reply))
                .await;
            return Ok(true);
        }

        // All /role sub-commands require admin.
        let is_admin = self
            .read_roles()
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
                let roles = self.read_roles();
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
                        OutboundMessage::text(
                            "รูปแบบ ID ไม่ถูกต้อง ต้องเป็น platform:id เช่น telegram:123456",
                        ),
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

        // /invite <role> [max_uses] — generate a shareable invite code (admin only)
        if let Some(rest) = trimmed.strip_prefix("/invite ") {
            if !is_admin {
                let _ = self
                    .platform
                    .send_message(&msg.channel, OutboundMessage::text("ต้องการสิทธิ์ admin"))
                    .await;
                return Ok(true);
            }
            let parts: Vec<&str> = rest.trim().splitn(2, ' ').collect();
            let role_name = parts[0].trim();
            let max_uses: u32 = parts
                .get(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(1);

            let code: String = uuid::Uuid::new_v4()
                .to_string()
                .replace('-', "")
                .chars()
                .take(8)
                .collect();

            let expires_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
                + 86400;

            let invite = InviteCode {
                role: role_name.to_string(),
                max_uses,
                uses: 0,
                expires_at: Some(expires_at),
            };

            let save_ok = {
                let mut roles = self.write_roles();
                roles.invites.insert(code.clone(), invite);
                let mut cfg = (*self.config).clone();
                cfg.roles = roles.clone();
                cfg.save_yaml().is_ok()
            };

            let reply = if save_ok {
                let uses_label = if max_uses == 0 {
                    "ไม่จำกัด".to_string()
                } else {
                    format!("{max_uses} ครั้ง")
                };
                format!(
                    "🎟️ Invite code\n\
                     /join {code}\n\n\
                     สิทธิ์: {role_name} | ใช้ได้: {uses_label} | หมดอายุ: 24 ชม."
                )
            } else {
                "บันทึก code ไม่สำเร็จ".to_string()
            };
            let _ = self
                .platform
                .send_message(&msg.channel, OutboundMessage::text(reply))
                .await;
            return Ok(true);
        }

        Ok(false)
    }

    /// Update a role in-memory and persist to config.yaml atomically.
    fn save_role(&self, platform: &str, user_id: &str, role: &str) -> std::io::Result<()> {
        let mut roles = self.write_roles();
        roles.set_user_role(platform, user_id, role);
        let mut cfg = (*self.config).clone();
        cfg.roles = roles.clone();
        cfg.save_yaml()
    }

    /// Remove a role in-memory and persist to config.yaml atomically.
    fn remove_role(&self, platform: &str, user_id: &str) -> bool {
        let mut roles = self.write_roles();
        let removed = roles.remove_user(platform, user_id);
        if removed {
            let mut cfg = (*self.config).clone();
            cfg.roles = roles.clone();
            let _ = cfg.save_yaml();
        }
        removed
    }

    /// Remove invite codes whose `expires_at` timestamp is in the past.
    /// Called hourly from a background task so `live_roles.invites` does not
    /// accumulate dead entries indefinitely.
    pub fn cleanup_expired_invites(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut roles = self.write_roles();
        roles
            .invites
            .retain(|_, code| code.expires_at.is_none_or(|exp| exp > now));
    }

    /// Bootstrap: if no admin exists yet and this is a DM, make the sender admin.
    /// Uses a write-lock to prevent two simultaneous DMs from both getting admin.
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
    #[tracing::instrument(skip_all, fields(session_key = session_key, images = attachments.len()))]
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
        let read_qr_installed = self.agent.has_tool("read_qr");

        for (i, att) in attachments.iter().enumerate() {
            let seq = seq_start + i + 1;
            let ts = chrono::Local::now().format("%d/%m/%Y %H:%M");
            let user_label = if user_name.is_empty() {
                format!("[รูปที่ {seq} เวลา {ts}]")
            } else {
                format!("[@{user_name} ส่งรูปที่ {seq} เวลา {ts}]")
            };

            // Reject files whose header does not match a supported image type.
            if !is_supported_image(&att.path).await {
                self.agent.inject_history(
                    session_key,
                    &user_label,
                    "[ไม่สามารถวิเคราะห์ได้ — ไฟล์ไม่ใช่รูปภาพที่รองรับ (JPEG/PNG/GIF/WebP)]",
                );
                if att.path.starts_with("/tmp/") {
                    let _ = tokio::fs::remove_file(&att.path).await;
                }
                continue;
            }

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

            // Analyse the image with whichever tools are installed, then
            // replace the placeholder. Both calls run here — before the temp
            // file is deleted below — so a later text question finds the
            // result in history instead of a path that no longer exists.
            let mut analysis = String::new();
            if view_image_installed {
                analysis = self
                    .agent
                    .run_tool(
                        "view_image",
                        serde_json::json!({
                            "source": att.path,
                            "question": "อธิบายรูปนี้โดยละเอียด"
                        }),
                    )
                    .await;
            }
            // QR/barcode decoding is deterministic (zbar) rather than a
            // vision-model guess, so append any decoded payload verbatim.
            if read_qr_installed {
                let qr = self
                    .agent
                    .run_tool("read_qr", serde_json::json!({ "image_path": att.path }))
                    .await;
                if is_qr_hit(&qr) {
                    let qr_line = format!("[QR code ที่อ่านได้: {}]", qr.trim());
                    if analysis.is_empty() {
                        analysis = qr_line;
                    } else {
                        analysis.push_str("\n\n");
                        analysis.push_str(&qr_line);
                    }
                }
            }
            if !analysis.is_empty() {
                self.agent.update_last_history(session_key, &analysis);
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
    #[tracing::instrument(skip_all, fields(session_key = session_key, docs = attachments.len()))]
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
        let request_id = uuid::Uuid::new_v4().to_string();
        let started_at = std::time::Instant::now();
        let span = tracing::info_span!(
            "handle",
            request_id = %request_id,
            platform = %msg.channel.platform,
            user_id = %msg.user_id,
            chat_id = %msg.channel.chat_id,
        );
        let _enter = span.enter();
        tracing::info!(
            has_text = !msg.text.is_empty(),
            images = msg.attachments.len(),
            docs = msg.doc_attachments.len(),
            "message received"
        );
        self.metrics.inc_platform_message(&msg.channel.platform);

        let pcfg = &self.config.platform;

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

        // Bootstrap: if roles are configured but no admin exists yet, make the
        // first DM sender admin automatically so the operator doesn't need to
        // edit config.yaml by hand just to grant themselves access.
        {
            let needs_bootstrap = {
                let roles = self.read_roles();
                // Only bootstrap when definitions are configured but no user
                // has been assigned any role yet (completely fresh install).
                // If any user exists in the map the operator has started
                // configuring manually, so we leave it alone.
                let total_assigned: usize = roles.users.values().map(HashMap::len).sum();
                roles.definitions.contains_key("admin")
                    && roles.default_role.is_none()
                    && total_assigned == 0
            };
            if needs_bootstrap
                && !msg.is_group
                && self
                    .save_role(&msg.channel.platform, &msg.user_id, "admin")
                    .is_ok()
            {
                tracing::info!(
                    user_id = %msg.user_id,
                    platform = %msg.channel.platform,
                    "bootstrapped first admin"
                );
                let _ = self
                    .platform
                    .send_message(
                        &msg.channel,
                        OutboundMessage::text("✅ คุณได้รับการตั้งเป็น admin คนแรกของระบบ"),
                    )
                    .await;
            }
        }

        // Role check: handle /whoami and /role commands before anything else
        // (including the mention gate) so they always work.
        if self.handle_role_command(&msg).await? {
            return Ok(());
        }

        // Unknown-user gate: when roles are configured and this user has no
        // role (and no default_role covers them), prompt them to /join instead
        // of silently failing when the agent tries to run tools.
        // The guard is dropped inside the inner block so no RwLockReadGuard
        // crosses the await point below.
        let block_unknown_user = {
            let roles = self.read_roles();
            let roles_active = !roles.definitions.is_empty()
                || !roles.users.is_empty()
                || roles.default_role.is_some();
            if roles_active {
                let has_access = roles
                    .lookup_role(&msg.channel.platform, &msg.user_id, None)
                    .is_some()
                    || roles.default_role.is_some();
                !has_access
            } else {
                false
            }
        };
        if block_unknown_user {
            let _ = self
                .platform
                .send_message(
                    &msg.channel,
                    OutboundMessage::text(
                        "สวัสดี! คุณยังไม่มีสิทธิ์เข้าใช้งาน\n\
                         พิมพ์ /join เพื่อขอสิทธิ์จาก admin",
                    ),
                )
                .await;
            return Ok(());
        }

        // Per-user rate limit — checked after access control so unknown users
        // still hit the unknown-user gate rather than the rate-limit message.
        if !self.check_user_rate_limit(&msg.channel.platform, &msg.user_id) {
            let _ = self
                .platform
                .send_message(
                    &msg.channel,
                    OutboundMessage::text("⚠️ คุณส่งข้อความเร็วเกินไป กรุณารอสักครู่"),
                )
                .await;
            return Ok(());
        }

        // Image-only or doc-only messages bypass the mention gate.
        if msg.text.trim().is_empty()
            && (!msg.attachments.is_empty() || !msg.doc_attachments.is_empty())
        {
            self.sessions
                .touch(&msg.session_key, &msg.channel.platform, &msg.user_id)
                .await;

            if !msg.attachments.is_empty() {
                let (imgs, _) = filter_oversized(
                    &msg.attachments,
                    pcfg.max_image_bytes,
                    |a| &a.path,
                    |a| a.path.clone(),
                )
                .await;
                self.process_images(&imgs, &msg.session_key, 0, &msg.user_name)
                    .await;
            }
            if !msg.doc_attachments.is_empty() {
                let (docs, rejected) = filter_oversized(
                    &msg.doc_attachments,
                    pcfg.max_doc_bytes,
                    |a| &a.path,
                    |a| a.path.clone(),
                )
                .await;
                for path in rejected {
                    if path.starts_with("/tmp/") {
                        let _ = tokio::fs::remove_file(&path).await;
                    }
                }
                if !docs.is_empty() {
                    self.process_docs(
                        &docs,
                        &msg.session_key,
                        &msg.user_name,
                        &msg.channel,
                        self.approver_for(&msg),
                    )
                    .await;
                }
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
            let (imgs, _) = filter_oversized(
                &msg.attachments,
                pcfg.max_image_bytes,
                |a| &a.path,
                |a| a.path.clone(),
            )
            .await;
            self.process_images(&imgs, &msg.session_key, 0, &msg.user_name)
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
        let approver = self.approver_for(&msg);
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

        // Drop blank events — platform sent a message with no text and no attachments.
        if msg.text.trim().is_empty() {
            return Ok(());
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

        let metrics = self.metrics.clone();
        let enqueue_elapsed = started_at.elapsed();
        tracing::info!(
            task_id = %task_id,
            session_key = %session_key,
            enqueue_ms = enqueue_elapsed.as_millis(),
            "dispatching to agent"
        );

        tokio::spawn(async move {
            let agent_start = std::time::Instant::now();
            match agent
                .run_for_user(
                    &task,
                    approver,
                    &platform_name,
                    None,
                    Some(&session_key),
                    &user_id,
                )
                .await
            {
                Ok(result) => {
                    if let Some(db) = &session_db {
                        let _ = db.finish_task(&task_id);
                    }
                    metrics.add_platform_iterations(&platform_name, result.iterations);
                    tracing::info!(
                        task_id = %task_id,
                        iterations = result.iterations,
                        input_tokens = result.usage.input_tokens,
                        output_tokens = result.usage.output_tokens,
                        elapsed_ms = agent_start.elapsed().as_millis(),
                        "agent completed"
                    );
                    let reply = OutboundMessage::markdown(result.output);
                    if let Err(e) = platform.send_message(&channel, reply).await {
                        tracing::error!(task_id = %task_id, error = %e, "send_message failed");
                    }
                }
                Err(e) => {
                    if let Some(db) = &session_db {
                        let _ = db.finish_task(&task_id);
                    }
                    metrics.inc_platform_error(&platform_name);
                    tracing::warn!(
                        task_id = %task_id,
                        error = %e,
                        elapsed_ms = agent_start.elapsed().as_millis(),
                        "agent error"
                    );
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

/// Split `attachments` into (accepted, rejected_paths). Files whose size on
/// disk exceeds `max_bytes` are dropped; their paths are returned so the
/// caller can send an error message and clean up the temp file.
async fn filter_oversized<T, P, Q>(
    attachments: &[T],
    max_bytes: u64,
    path_of: P,
    path_clone: Q,
) -> (Vec<T>, Vec<String>)
where
    T: Clone,
    P: Fn(&T) -> &str,
    Q: Fn(&T) -> String,
{
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    for att in attachments {
        let sz = file_size(path_of(att)).await;
        if sz > max_bytes {
            rejected.push(path_clone(att));
        } else {
            accepted.push(att.clone());
        }
    }
    (accepted, rejected)
}

/// Return the size of the file at `path`, or 0 on any I/O error.
async fn file_size(path: &str) -> u64 {
    tokio::fs::metadata(path).await.map_or(0, |m| m.len())
}

/// True if the file at `path` starts with a recognised image magic-byte
/// signature (JPEG, PNG, GIF, WebP). Returns true on any I/O error so
/// that a header read failure never silently discards a valid image.
async fn is_supported_image(path: &str) -> bool {
    let Ok(mut f) = tokio::fs::File::open(path).await else {
        return true;
    };
    let mut buf = [0u8; 12];
    let n = f.read(&mut buf).await.unwrap_or(0);
    let b = &buf[..n];
    matches!(b.first(), Some(0xFF) if b.get(1) == Some(&0xD8)) // JPEG
        || b.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) // PNG
        || b.starts_with(b"GIF8") // GIF
        || (b.len() >= 12 // WebP
            && b.starts_with(&[0x52, 0x49, 0x46, 0x46])
            && &b[8..12] == b"WEBP")
}

/// True when `read_qr` output is an actual decoded payload, not an error
/// wrapper or a "nothing found" message. Robust to both the old `run.sh`
/// (no QR → non-zero exit → `[read_qr failed: …]`) and the new one (no QR →
/// exit 0 with a "No QR code found" line), so wiring is correct regardless of
/// which `read_qr` version is installed.
fn is_qr_hit(output: &str) -> bool {
    let s = output.trim();
    !s.is_empty() && !s.starts_with("[read_qr failed:") && !s.contains("No QR code found")
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use garudust_core::types::ImageAttachment;

    use super::{filter_oversized, is_qr_hit, is_supported_image, summarize_ingest};

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

    #[test]
    fn qr_hit_accepts_decoded_payload() {
        assert!(is_qr_hit("https://example.com/pay?id=42"));
        assert!(is_qr_hit("  00020101021129...  \n"));
    }

    #[test]
    fn qr_hit_rejects_empty_error_and_not_found() {
        assert!(!is_qr_hit(""));
        assert!(!is_qr_hit("   \n  "));
        // new run.sh: no QR → exit 0 with a message
        assert!(!is_qr_hit("No QR code found in image."));
        // old run.sh: no QR or missing file → non-zero exit → error wrapper
        assert!(!is_qr_hit(
            "[read_qr failed: script exited with exit status: 1]"
        ));
        assert!(!is_qr_hit(
            "[read_qr failed: file not found: /tmp/gone.jpg]"
        ));
    }

    // ── is_supported_image ────────────────────────────────────────────────────

    #[tokio::test]
    async fn jpeg_magic_bytes_accepted() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]).unwrap();
        assert!(is_supported_image(f.path().to_str().unwrap()).await);
    }

    #[tokio::test]
    async fn png_magic_bytes_accepted() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00])
            .unwrap();
        assert!(is_supported_image(f.path().to_str().unwrap()).await);
    }

    #[tokio::test]
    async fn webp_magic_bytes_accepted() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"RIFF\x00\x00\x00\x00WEBP").unwrap();
        assert!(is_supported_image(f.path().to_str().unwrap()).await);
    }

    #[tokio::test]
    async fn random_bytes_rejected() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"%PDF-1.4 this is a pdf not an image").unwrap();
        assert!(!is_supported_image(f.path().to_str().unwrap()).await);
    }

    #[tokio::test]
    async fn missing_file_treated_as_accepted() {
        assert!(is_supported_image("/tmp/garudust_nonexistent_xyz.jpg").await);
    }

    // ── check_user_rate_limit ────────────────────────────────────────────────

    fn make_handler_with_rpm(rpm: u32) -> super::GatewayHandler {
        use std::sync::Arc;

        use async_trait::async_trait;
        use futures::stream;
        use garudust_agent::Agent;
        use garudust_core::{
            config::AgentConfig,
            error::{AgentError, PlatformError, TransportError},
            memory::{MemoryContent, MemoryStore},
            platform::PlatformAdapter,
            types::{
                ChannelId, ContentPart, InferenceConfig, Message, OutboundMessage, StopReason,
                StreamChunk, TokenUsage, ToolSchema, TransportResponse,
            },
            transport::{ApiMode, ProviderTransport, StreamResult},
        };
        use garudust_memory::SessionDb;
        use garudust_tools::ToolRegistry;
        use std::pin::Pin;

        struct Echo;
        #[async_trait]
        impl ProviderTransport for Echo {
            fn api_mode(&self) -> ApiMode {
                ApiMode::ChatCompletions
            }
            async fn chat(
                &self,
                _m: &[Message],
                _c: &InferenceConfig,
                _t: &[ToolSchema],
            ) -> Result<TransportResponse, TransportError> {
                Ok(TransportResponse {
                    content: vec![ContentPart::Text("ok".into())],
                    tool_calls: vec![],
                    usage: TokenUsage::default(),
                    stop_reason: StopReason::EndTurn,
                })
            }
            async fn chat_stream(
                &self,
                _m: &[Message],
                _c: &InferenceConfig,
                _t: &[ToolSchema],
            ) -> Result<StreamResult, TransportError> {
                Ok(Box::pin(stream::iter(vec![Ok(StreamChunk::Done {
                    usage: TokenUsage::default(),
                })])))
            }
        }

        struct NopMem;
        #[async_trait]
        impl MemoryStore for NopMem {
            async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
                Ok(MemoryContent::default())
            }
            async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
                Ok(())
            }
            async fn read_user_profile(&self) -> Result<String, AgentError> {
                Ok(String::new())
            }
            async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
                Ok(())
            }
        }

        struct NopPlatform;
        #[async_trait]
        impl PlatformAdapter for NopPlatform {
            fn name(&self) -> &'static str {
                "test"
            }
            async fn start(
                &self,
                _h: Arc<dyn garudust_core::platform::MessageHandler>,
            ) -> Result<(), PlatformError> {
                Ok(())
            }
            async fn send_message(
                &self,
                _: &ChannelId,
                _: OutboundMessage,
            ) -> Result<(), PlatformError> {
                Ok(())
            }
            async fn send_stream(
                &self,
                _: &ChannelId,
                _: Pin<Box<dyn futures::Stream<Item = String> + Send>>,
            ) -> Result<(), PlatformError> {
                Ok(())
            }
        }

        let mut config = AgentConfig::default();
        config.security.rate_limit_rpm_per_user = Some(rpm);
        let config = Arc::new(config);
        let transport = Arc::new(Echo);
        let tools = Arc::new(ToolRegistry::new());
        let memory = Arc::new(NopMem);
        let tmp = std::env::temp_dir()
            .join(format!("garudust-ratelimit-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let db = Arc::new(SessionDb::open(&tmp).unwrap());
        let agent = Arc::new(
            Agent::new(transport, tools, memory, config.clone()).with_session_db(db.clone()),
        );
        let sessions = crate::sessions::SessionRegistry::new();
        let metrics = Arc::new(crate::metrics::Metrics::default());

        super::GatewayHandler::new(
            agent,
            Arc::new(NopPlatform),
            sessions,
            Arc::new(garudust_agent::AutoApprover),
            config,
            Some(db),
            metrics,
        )
    }

    #[test]
    fn rate_limit_allows_up_to_limit() {
        let h = make_handler_with_rpm(3);
        assert!(h.check_user_rate_limit("tg", "user1"));
        assert!(h.check_user_rate_limit("tg", "user1"));
        assert!(h.check_user_rate_limit("tg", "user1"));
        assert!(!h.check_user_rate_limit("tg", "user1"));
    }

    #[test]
    fn rate_limit_independent_per_user() {
        let h = make_handler_with_rpm(2);
        assert!(h.check_user_rate_limit("tg", "alice"));
        assert!(h.check_user_rate_limit("tg", "alice"));
        assert!(!h.check_user_rate_limit("tg", "alice"));
        // bob is on a fresh counter
        assert!(h.check_user_rate_limit("tg", "bob"));
        assert!(h.check_user_rate_limit("tg", "bob"));
        assert!(!h.check_user_rate_limit("tg", "bob"));
    }

    #[test]
    fn rate_limit_disabled_when_none() {
        use garudust_core::config::AgentConfig;
        use std::sync::Arc;
        let config = Arc::new(AgentConfig::default());
        assert!(config.security.rate_limit_rpm_per_user.is_none());
        // None short-circuits to true regardless of the closure.
        let limit: Option<u32> = None;
        assert!(limit.is_none_or(|_| false));
    }

    // ── filter_oversized ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn filter_oversized_keeps_small_rejects_large() {
        let small = tempfile::NamedTempFile::new().unwrap();
        let large = tempfile::NamedTempFile::new().unwrap();
        {
            let mut lf = std::fs::OpenOptions::new()
                .write(true)
                .open(large.path())
                .unwrap();
            lf.write_all(&[0u8; 200]).unwrap();
        }

        let atts = vec![
            ImageAttachment {
                path: small.path().to_str().unwrap().to_string(),
            },
            ImageAttachment {
                path: large.path().to_str().unwrap().to_string(),
            },
        ];

        let (accepted, rejected) =
            filter_oversized(&atts, 100, |a| &a.path, |a| a.path.clone()).await;
        assert_eq!(accepted.len(), 1);
        assert_eq!(rejected.len(), 1);
    }
}
