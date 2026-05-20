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

    use crate::{handler::GatewayHandler, sessions::SessionRegistry};

    // ── Minimal stubs ────────────────────────────────────────────────────────

    struct EchoTransport;
    #[async_trait]
    impl ProviderTransport for EchoTransport {
        fn api_mode(&self) -> ApiMode { ApiMode::ChatCompletions }
        async fn chat(&self, _m: &[Message], _c: &InferenceConfig, _t: &[ToolSchema]) -> Result<TransportResponse, TransportError> {
            Ok(TransportResponse {
                content: vec![ContentPart::Text("ok".into())],
                tool_calls: vec![],
                usage: TokenUsage::default(),
                stop_reason: StopReason::EndTurn,
            })
        }
        async fn chat_stream(&self, _m: &[Message], _c: &InferenceConfig, _t: &[ToolSchema]) -> Result<StreamResult, TransportError> {
            Ok(Box::pin(stream::iter(vec![Ok(StreamChunk::Done { usage: TokenUsage::default() })])))
        }
    }

    struct NopMemory;
    #[async_trait]
    impl MemoryStore for NopMemory {
        async fn read_memory(&self) -> Result<MemoryContent, AgentError> { Ok(MemoryContent::default()) }
        async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> { Ok(()) }
        async fn read_user_profile(&self) -> Result<String, AgentError> { Ok(String::new()) }
        async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> { Ok(()) }
    }

    /// Captures every outbound message (channel_id → text).
    #[derive(Clone)]
    struct MockPlatform {
        pub sent: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl MockPlatform {
        fn new() -> Self {
            Self { sent: Arc::new(Mutex::new(vec![])) }
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
        fn name(&self) -> &'static str { "mock" }
        async fn start(&self, _: Arc<dyn garudust_core::platform::MessageHandler>) -> Result<(), PlatformError> { Ok(()) }
        async fn send_message(&self, channel: &ChannelId, msg: OutboundMessage) -> Result<(), PlatformError> {
            self.sent.lock().await.push((channel.chat_id.clone(), msg.text));
            Ok(())
        }
        async fn send_stream(&self, _: &ChannelId, _: Pin<Box<dyn Stream<Item = String> + Send>>) -> Result<(), PlatformError> { Ok(()) }
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
            RoleDefinition { approval_mode: Some("auto".into()), ..Default::default() },
        );
        definitions.insert(
            "member".into(),
            RoleDefinition { approval_mode: Some("auto".into()), ..Default::default() },
        );
        AgentConfig {
            home_dir: tmp.to_path_buf(),
            roles: RolesConfig { definitions, ..Default::default() },
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
        )
    }

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn bootstrap_first_dm_becomes_admin() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let cfg = make_config(tmp.path());
        let handler = make_handler(platform.clone(), cfg);

        handler.handle(dm("alice", "สวัสดี")).await.unwrap();

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(
            msg.contains("admin") || msg.contains("🎉"),
            "ควรได้รับแจ้ง bootstrap admin แต่ได้: {msg}"
        );
        assert!(
            handler.live_roles.read().unwrap().has_any_admin(),
            "live_roles ควร has_any_admin = true"
        );
    }

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
        assert!(msg.contains("mock:alice"), "whoami ควรแสดง platform:id: {msg}");
    }

    #[tokio::test]
    async fn unknown_user_gets_lowest_role_automatically() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        // pre-set an admin so bootstrap doesn't fire
        cfg.roles.set_user_role("mock", "admin_user", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // "stranger" sends a message → should get auto-assigned the lowest role
        handler.handle(dm("stranger", "ทดสอบ")).await.unwrap();

        // live_roles should now have stranger assigned (not blocked/pending)
        let role = handler.live_roles.read().unwrap().lookup_role("mock", "stranger", None);
        assert!(role.is_some(), "ผู้ใช้ใหม่ควรได้รับ role อัตโนมัติ ไม่ใช่ถูก block");
    }

    #[tokio::test]
    async fn whoami_shows_auto_assigned_role_after_first_message() {
        let tmp = tmpdir();
        let platform = Arc::new(MockPlatform::new());
        let mut cfg = make_config(tmp.path());
        cfg.roles.set_user_role("mock", "admin_user", "admin");
        let handler = make_handler(platform.clone(), cfg);

        // First message triggers auto-assign, then /whoami shows the assigned role
        handler.handle(dm("bob", "สวัสดี")).await.unwrap();
        handler.handle(dm("bob", "/whoami")).await.unwrap();

        let msg = platform.last_to("bob").await.unwrap_or_default();
        assert!(!msg.contains("pending"), "หลัง auto-assign ไม่ควรแสดง pending: {msg}");
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
        let role = handler.live_roles.read().unwrap().lookup_role("mock", "bob", None);
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
        handler.handle(dm("alice", "/role deny mock:bob")).await.unwrap();

        let role = handler.live_roles.read().unwrap().lookup_role("mock", "bob", None);
        assert_eq!(role, None, "/role deny ควรเพิกถอน role ออกจาก live_roles");

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("bob") || msg.contains("เพิกถอน"), "ควรได้ confirm: {msg}");
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
        handler.handle(dm("alice", "/role remove mock:bob")).await.unwrap();

        let role = handler.live_roles.read().unwrap().lookup_role("mock", "bob", None);
        assert_eq!(role, None, "หลัง remove ไม่ควรมี role เหลือ");

        let msg = platform.last_to("alice").await.unwrap_or_default();
        assert!(msg.contains("bob") || msg.contains("ลบ"), "ควรได้ confirm remove: {msg}");
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

        let role = handler.live_roles.read().unwrap().lookup_role("mock", "carol", None);
        assert_eq!(role, Some("member".into()), "/role add ควร update live_roles");
    }
}
