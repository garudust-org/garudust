use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};
use matrix_sdk::{
    config::SyncSettings,
    room::Room,
    ruma::{
        events::room::message::{
            MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent,
        },
        RoomId,
    },
    Client,
};
use tokio::sync::OnceCell;

pub struct MatrixAdapter {
    homeserver: String,
    username: String,
    password: String,
    client: Arc<OnceCell<Client>>,
}

impl MatrixAdapter {
    pub fn new(homeserver: String, username: String, password: String) -> Self {
        Self {
            homeserver,
            username,
            password,
            client: Arc::new(OnceCell::new()),
        }
    }
}

#[async_trait]
impl PlatformAdapter for MatrixAdapter {
    fn name(&self) -> &'static str {
        "matrix"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let client = Client::builder()
            .homeserver_url(&self.homeserver)
            .build()
            .await
            .map_err(|e| PlatformError::Connection(e.to_string()))?;

        client
            .matrix_auth()
            .login_username(&self.username, &self.password)
            .initial_device_display_name("Garudust")
            .send()
            .await
            .map_err(|_| PlatformError::Auth)?;

        tracing::info!("Matrix logged in as {}", self.username);

        // Store client for send_message
        let _ = self.client.set(client.clone());

        // Filter out our own messages
        let bot_user_id = client.user_id().map(std::borrow::ToOwned::to_owned);

        client.add_event_handler(move |ev: OriginalSyncRoomMessageEvent, _room: Room| {
            let handler = handler.clone();
            let bot_uid = bot_user_id.clone();
            async move {
                if bot_uid.as_ref().is_some_and(|id| id == &ev.sender) {
                    return;
                }
                let MessageType::Text(text_content) = ev.content.msgtype else {
                    return;
                };
                let room_id = _room.room_id().to_string();
                let inbound = InboundMessage {
                    channel: ChannelId {
                        platform: "matrix".into(),
                        chat_id: room_id.clone(),
                        thread_id: None,
                    },
                    user_id: ev.sender.to_string(),
                    user_name: ev.sender.localpart().to_string(),
                    text: text_content.body,
                    session_key: format!("matrix:{room_id}"),
                };
                let _ = handler.handle(inbound).await;
            }
        });

        // Long-poll sync loop in background
        tokio::spawn(async move {
            if let Err(e) = client.sync(SyncSettings::default()).await {
                tracing::error!("Matrix sync error: {e}");
            }
        });

        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        let client = self
            .client
            .get()
            .ok_or_else(|| PlatformError::Send("Matrix not started".into()))?;

        let room_id = RoomId::parse(&channel.chat_id)
            .map_err(|e| PlatformError::Send(format!("invalid room id: {e}")))?;

        let room = client
            .get_room(&room_id)
            .ok_or_else(|| PlatformError::Send(format!("not in room {}", channel.chat_id)))?;

        room.send(RoomMessageEventContent::text_plain(message.text))
            .await
            .map_err(|e: matrix_sdk::Error| PlatformError::Send(e.to_string()))?;

        Ok(())
    }

    async fn send_stream(
        &self,
        channel: &ChannelId,
        mut stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            buf.push_str(&chunk);
        }
        self.send_message(channel, OutboundMessage::text(buf)).await
    }
}
