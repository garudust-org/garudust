/// Integration tests for GatewayHandler roles/RBAC feature.
///
/// Uses a MockPlatform that captures every outbound message so we can assert
/// on what the handler sent back without a real bot or network.
#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures::{stream, Stream};
    use garudust_agent::Agent;
    use garudust_core::{
        config::{AgentConfig, RoleDefinition},
        error::{AgentError, PlatformError, TransportError},
        memory::{MemoryContent, MemoryStore},
        platform::PlatformAdapter,
        transport::{ApiMode, ProviderTransport, StreamResult},
        types::{
            ChannelId, ContentPart, InboundMessage, InferenceConfig, Message, OutboundMessage,
            StopReason, StreamChunk, TokenUsage, ToolSchema, TransportResponse,
        },
    };
    use garudust_memory::SessionDb;
    use garudust_tools::ToolRegistry;
    use tokio::sync::Mutex;

    use garudust_core::platform::MessageHandler as _;

    use crate::{handler::GatewayHandler, metrics::Metrics, sessions::SessionRegistry};

    // ── Minimal stubs ────────────────────────────────────────────────────────

    struct EchoTransport;
    #[async_trait]
    impl ProviderTransport for EchoTransport {
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

    struct NopMemory;
    #[async_trait]
    impl MemoryStore for NopMemory {
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

    /// Captures every outbound message (channel_id → text).
    #[derive(Clone)]
    struct MockPlatform {
        pub sent: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl MockPlatform {
        fn new() -> Self {
            Self {
                sent: Arc::new(Mutex::new(vec![])),
            }
        }

        async fn last_to(&self, chat_id: &str) -> Option<String> {
            self.sent
                .lock()
                .await
                .iter()
                .rev()
                .find(|(id, _)| id == chat_id)
                .map(|(_, text)| text.clone())
        }
    }

    #[async_trait]
    impl PlatformAdapter for MockPlatform {
        fn name(&self) -> &'static str {
            "mock"
        }
        async fn start(
            &self,
            _: Arc<dyn garudust_core::platform::MessageHandler>,
        ) -> Result<(), PlatformError> {
            Ok(())
        }
        async fn send_message(
            &self,
            channel: &ChannelId,
            msg: OutboundMessage,
        ) -> Result<(), PlatformError> {
            self.sent
                .lock()
                .await
                .push((channel.chat_id.clone(), msg.text));
            Ok(())
        }
        async fn send_stream(
            &self,
            _: &ChannelId,
            _: Pin<Box<dyn Stream<Item = String> + Send>>,
        ) -> Result<(), PlatformError> {
            Ok(())
        }
    }

    // ── Helper builders ──────────────────────────────────────────────────────

    fn dm(user_id: &str, text: &str) -> InboundMessage {
        InboundMessage {
            channel: ChannelId {
                platform: "mock".into(),
                chat_id: user_id.into(),
                thread_id: None,
            },
            user_id: user_id.into(),
            user_name: user_id.into(),
            text: text.into(),
            session_key: format!("mock:{user_id}"),
            is_group: false,
            bot_mentioned: None,
            attachments: vec![],
            doc_attachments: vec![],
        }
    }

    fn make_config(tmp: &std::path::Path) -> AgentConfig {
        use garudust_core::config::RolesConfig;
        let mut definitions = std::collections::HashMap::new();
        definitions.insert(
            "admin".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                ..Default::default()
            },
        );
        definitions.insert(
            "member".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                ..Default::default()
            },
        );
        AgentConfig {
            home_dir: tmp.to_path_buf(),
            roles: RolesConfig {
                definitions,
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn make_handler(platform: Arc<MockPlatform>, cfg: AgentConfig) -> GatewayHandler {
        let tmp = cfg.home_dir.clone();
        let cfg = Arc::new(cfg);
        let db = Arc::new(SessionDb::open(&tmp).unwrap());
        let agent = Arc::new(Agent::new(
            Arc::new(EchoTransport),
            Arc::new(ToolRegistry::new()),
            Arc::new(NopMemory),
            cfg.clone(),
        ));
        GatewayHandler::new(
            agent,
            platform,
            SessionRegistry::new(),
            Arc::new(garudust_agent::AutoApprover),
            cfg,
            Some(db),
            Arc::new(Metrics::default()),
        )
    }

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn whoami_shows_role() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("alice", "/whoami")).await.unwrap();

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("admin"), "whoami ควรแสดง role: {msg}");
        assert!(
            msg.contains("mock:alice"),
            "whoami ควรแสดง platform:id: {msg}"
        );
    }

    #[tokio::test]
    async fn unknown_user_uses_default_role_without_writing_config() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.default_role = Some("member".into());
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("stranger", "ทดสอบ")).await.unwrap();

        // live_roles must NOT have written stranger — default_role covers them
        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "stranger", None);
        assert!(
            role.is_none(),
            "ไม่ควรเขียน config เมื่อ default_role ครอบคลุมแล้ว"
        );
    }

    #[tokio::test]
    async fn whoami_shows_default_role_for_unknown_user() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.default_role = Some("member".into());
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/whoami")).await.unwrap();

        let msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(msg.contains("member"), "whoami ควรแสดง default_role: {msg}");
        assert!(msg.contains("mock:bob"), "ควรแสดง id: {msg}");
    }

    #[tokio::test]
    async fn non_admin_cannot_use_role_commands() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "bob", "member");
        cfg.roles.set_user_role("mock", "admin_user", "admin");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/role list")).await.unwrap();

        let msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(msg.contains("admin"), "member ไม่ควรใช้ /role list ได้: {msg}");
    }

    #[tokio::test]
    async fn role_list_shows_current_users() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.set_user_role("mock", "bob", "member");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("alice", "/role list")).await.unwrap();

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("alice"), "/role list ควรแสดง alice: {msg}");
        assert!(msg.contains("bob"), "/role list ควรแสดง bob: {msg}");
    }

    #[tokio::test]
    async fn role_approve_takes_effect_immediately() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // approve bob as member
        handler
            .handle(dm("alice", "/role approve mock:bob member"))
            .await
            .unwrap();

        // verify live_roles updated immediately (no restart needed)
        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "bob", None);
        assert_eq!(role, Some("member".into()), "live_roles ควร update ทันที");

        // verify alice received confirmation
        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("member"), "ควรได้รับ confirm approve: {msg}");
    }

    #[tokio::test]
    async fn role_deny_revokes_role() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.set_user_role("mock", "bob", "member");
        let handler = make_handler(platform.clone(), cfg);

        // alice denies (revokes) bob
        handler
            .handle(dm("alice", "/role deny mock:bob"))
            .await
            .unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "bob", None);
        assert_eq!(role, None, "/role deny ควรเพิกถอน role ออกจาก live_roles");

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(
            msg.contains("bob") || msg.contains("เพิกถอน"),
            "ควรได้ confirm: {msg}"
        );
    }

    #[tokio::test]
    async fn role_remove_revokes_access() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.set_user_role("mock", "bob", "member");
        let handler = make_handler(platform.clone(), cfg);

        // alice removes bob
        handler
            .handle(dm("alice", "/role remove mock:bob"))
            .await
            .unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "bob", None);
        assert_eq!(role, None, "หลัง remove ไม่ควรมี role เหลือ");

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(
            msg.contains("bob") || msg.contains("ลบ"),
            "ควรได้ confirm remove: {msg}"
        );
    }

    #[tokio::test]
    async fn role_add_works_without_pending() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // alice directly adds carol (who never sent a message)
        handler
            .handle(dm("alice", "/role add mock:carol member"))
            .await
            .unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "carol", None);
        assert_eq!(
            role,
            Some("member".into()),
            "/role add ควร update live_roles"
        );
    }

    #[tokio::test]
    async fn unknown_user_without_default_role_gets_join_prompt() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        // There's already an admin → bootstrap won't fire. "stranger" has no role.
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // "stranger" sends a normal message — should get the /join prompt, not an agent reply
        handler.handle(dm("stranger", "สวัสดี")).await.unwrap();

        let msg = platform.last_to("stranger").await.unwrap_or_default();
        assert!(
            msg.contains("/join"),
            "ผู้ใช้ไม่มีสิทธิ์ควรได้รับ /join prompt: {msg}"
        );
    }

    #[tokio::test]
    async fn join_notifies_admins_and_replies_to_user() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/join")).await.unwrap();

        // bob should get a confirmation reply
        let bob_msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(
            bob_msg.contains("admin") || bob_msg.contains("รอ"),
            "/join ควรแจ้ง bob ว่าส่งคำขอแล้ว: {bob_msg}"
        );

        // alice (admin) should receive a notification with pre-built commands
        let alice_msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(
            alice_msg.contains("mock:bob"),
            "admin ควรได้รับ ID ของผู้ขอ: {alice_msg}"
        );
        assert!(
            alice_msg.contains("/role approve"),
            "notification ควรมีคำสั่ง /role approve: {alice_msg}"
        );
    }

    #[tokio::test]
    async fn join_when_already_has_role_replies_with_info() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "member");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("alice", "/join")).await.unwrap();

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(
            msg.contains("สิทธิ์") || msg.contains("อยู่แล้ว"),
            "user ที่มี role อยู่แล้วควรได้รับ info ไม่ใช่ส่ง notification: {msg}"
        );
    }

    #[tokio::test]
    async fn first_dm_bootstraps_admin_when_no_admin_exists() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        // definitions have "admin" defined but no users assigned yet
        let cfg = make_config(tmp.path());
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("first_user", "สวัสดี")).await.unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "first_user", None);
        assert_eq!(
            role,
            Some("admin".into()),
            "ผู้ส่ง DM คนแรกควรได้รับ admin อัตโนมัติ"
        );

        let msg = platform.last_to("first_user").await.unwrap_or_default();
        assert!(msg.contains("admin"), "ควรแจ้ง user ว่าได้รับ admin: {msg}");
    }

    // ── Invite code tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn invite_code_grants_role_immediately() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        // Pre-insert a valid invite code
        cfg.roles.invites.insert(
            "abc12345".to_string(),
            garudust_core::config::InviteCode {
                role: "member".to_string(),
                max_uses: 1,
                uses: 0,
                expires_at: None,
            },
        );
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/join abc12345")).await.unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "bob", None);
        assert_eq!(role, Some("member".into()), "code ถูกต้องควรได้รับ role ทันที");

        let msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(msg.contains("member"), "ควรแจ้ง role ที่ได้รับ: {msg}");
    }

    #[tokio::test]
    async fn invite_code_removed_after_single_use() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.invites.insert(
            "once1234".to_string(),
            garudust_core::config::InviteCode {
                role: "member".to_string(),
                max_uses: 1,
                uses: 0,
                expires_at: None,
            },
        );
        let handler = make_handler(platform.clone(), cfg);

        // first use — succeeds
        handler.handle(dm("bob", "/join once1234")).await.unwrap();
        // second use — should fail (code exhausted)
        handler.handle(dm("carol", "/join once1234")).await.unwrap();

        let carol_role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "carol", None);
        assert!(carol_role.is_none(), "code ใช้ครบแล้วไม่ควรให้สิทธิ์");

        let msg = platform.last_to("carol").await.unwrap_or_default();
        assert!(
            msg.contains("❌") || msg.contains("ถูกใช้") || msg.contains("หมด"),
            "ควรแจ้งว่า code ใช้ไม่ได้: {msg}"
        );
    }

    #[tokio::test]
    async fn invite_code_multi_use_stays_until_exhausted() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.invites.insert(
            "multi123".to_string(),
            garudust_core::config::InviteCode {
                role: "member".to_string(),
                max_uses: 2,
                uses: 0,
                expires_at: None,
            },
        );
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/join multi123")).await.unwrap();
        // code should still exist after first use
        let still_exists = handler
            .live_roles
            .read()
            .unwrap()
            .invites
            .contains_key("multi123");
        assert!(still_exists, "max_uses=2 ควรยังคง code ไว้หลังใช้ครั้งแรก");

        handler.handle(dm("carol", "/join multi123")).await.unwrap();
        // exhausted after second use
        let gone = !handler
            .live_roles
            .read()
            .unwrap()
            .invites
            .contains_key("multi123");
        assert!(gone, "code ควรถูกลบหลังใช้ครบ max_uses");
    }

    #[tokio::test]
    async fn invite_code_expired_is_rejected() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        cfg.roles.invites.insert(
            "expired1".to_string(),
            garudust_core::config::InviteCode {
                role: "member".to_string(),
                max_uses: 1,
                uses: 0,
                expires_at: Some(1), // unix epoch + 1s — already expired
            },
        );
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/join expired1")).await.unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "bob", None);
        assert!(role.is_none(), "หมดอายุแล้วไม่ควรได้รับสิทธิ์");
    }

    #[tokio::test]
    async fn admin_can_create_invite_code() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("alice", "/invite member")).await.unwrap();

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("/join"), "reply ควรมี /join <code>: {msg}");
        assert!(msg.contains("member"), "reply ควรระบุ role ที่ให้: {msg}");
        // code should be stored in live_roles
        let has_code = !handler.live_roles.read().unwrap().invites.is_empty();
        assert!(has_code, "ควรบันทึก invite code ใน live_roles");
    }

    #[tokio::test]
    async fn non_admin_cannot_create_invite_code() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "bob", "member");
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("bob", "/invite member")).await.unwrap();

        let msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(msg.contains("admin"), "member ไม่ควรสร้าง invite ได้: {msg}");
        let no_code = handler.live_roles.read().unwrap().invites.is_empty();
        assert!(no_code, "ไม่ควรบันทึก code เมื่อไม่มีสิทธิ์");
    }

    #[tokio::test]
    async fn bootstrap_does_not_fire_in_group_chat() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let cfg = make_config(tmp.path());
        let handler = make_handler(platform.clone(), cfg);

        let mut group_msg = dm("first_user", "สวัสดี");
        group_msg.is_group = true;
        handler.handle(group_msg).await.unwrap();

        let role = handler
            .live_roles
            .read()
            .unwrap()
            .lookup_role("mock", "first_user", None);
        assert!(role.is_none(), "group chat ไม่ควร trigger bootstrap admin");
    }

    /// Regression: a message carrying BOTH text and an image attachment (e.g. a
    /// LINE reply that quotes an image) must not self-deadlock. The handler
    /// acquires the per-session image gate up-front, then later awaits the same
    /// gate to wait for in-flight image analysis. Re-locking the non-reentrant
    /// mutex from the same task wedged the session forever — the bot went silent.
    #[tokio::test]
    async fn text_with_image_attachment_does_not_deadlock() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "alice", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // A real file so filter_oversized has something to stat.
        let img = tmp.path().join("pic.jpg");
        tokio::fs::write(&img, b"\xff\xd8\xff\xe0fake")
            .await
            .unwrap();

        let mut msg = dm("alice", "นี่รูปอะไร");
        msg.attachments = vec![garudust_core::types::ImageAttachment {
            path: img.to_string_lossy().into_owned(),
        }];

        // Before the fix this never returned (self-deadlock on the image gate).
        tokio::time::timeout(std::time::Duration::from_secs(10), handler.handle(msg))
            .await
            .expect("handler deadlocked on the image gate")
            .unwrap();
    }
}
