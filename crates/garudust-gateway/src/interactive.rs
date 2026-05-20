use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use dashmap::DashMap;
use garudust_core::{
    platform::PlatformAdapter,
    tool::{ApprovalDecision, CommandApprover},
    types::{ChannelId, OutboundMessage},
};
use tokio::sync::oneshot;

/// Asks the user for approval via the messaging platform before running a tool.
///
/// When `approve()` is called the approver:
/// 1. Generates a short random ID
/// 2. Sends a message to the user listing the tool + params and two slash commands
/// 3. Blocks (async) until the user replies `/approve <id>` or `/deny <id>`,
///    or until the timeout elapses (default 60 s → auto-deny)
///
/// The handler resolves pending approvals in `GatewayHandler::handle_role_command`.
pub struct InteractiveApprover {
    pub(crate) platform: Arc<dyn PlatformAdapter>,
    pub(crate) channel: ChannelId,
    pub(crate) pending: Arc<DashMap<String, oneshot::Sender<bool>>>,
    pub(crate) timeout: Duration,
}

#[async_trait]
impl CommandApprover for InteractiveApprover {
    async fn approve(&self, tool: &str, params: &str, _user_id: &str) -> ApprovalDecision {
        let id: String = uuid::Uuid::new_v4()
            .to_string()
            .replace('-', "")
            .chars()
            .take(8)
            .collect();

        let (tx, rx) = oneshot::channel();
        self.pending.insert(id.clone(), tx);

        let secs = self.timeout.as_secs();
        let text = format!(
            "🔐 ขออนุมัติการใช้ tool\n\
             \n\
             Tool: {tool}\n\
             Params: {params}\n\
             \n\
             /approve {id} — อนุมัติครั้งนี้\n\
             /deny {id}    — ปฏิเสธ\n\
             (หมดเวลาใน {secs}s)"
        );
        let _ = self
            .platform
            .send_message(&self.channel, OutboundMessage::text(text))
            .await;

        match tokio::time::timeout(self.timeout, rx).await {
            Ok(Ok(true)) => {
                tracing::info!(tool, id, "interactive approval granted");
                ApprovalDecision::Approved
            }
            Ok(Ok(false)) => {
                tracing::info!(tool, id, "interactive approval denied by user");
                ApprovalDecision::Denied
            }
            _ => {
                self.pending.remove(&id);
                tracing::info!(tool, id, "interactive approval timed out → denied");
                ApprovalDecision::Denied
            }
        }
    }
}
