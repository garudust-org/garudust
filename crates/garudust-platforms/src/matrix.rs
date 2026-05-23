use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use matrix_sdk::{
    config::SyncSettings,
    media::{MediaFormat, MediaRequestParameters},
    room::Room,
    ruma::{
        events::room::{
            message::{MessageType, OriginalSyncRoomMessageEvent, RoomMessageEventContent},
            MediaSource,
        },
        RoomId,
    },
    Client,
};
use tokio::sync::OnceCell;

async fn download_matrix_image(
    client: &Client,
    source: &MediaSource,
    event_id: &str,
    ext: &str,
) -> Vec<ImageAttachment> {
    let req = MediaRequestParameters {
        source: source.clone(),
        format: MediaFormat::File,
    };
    match client.media().get_media_content(&req, false).await {
        Ok(bytes) => {
            let safe_id = event_id.replace(['/', '$', ':'], "_");
            let dest = format!("/tmp/garudust_matrix_{safe_id}.{ext}");
            match tokio::fs::write(&dest, &bytes).await {
                Ok(()) => vec![ImageAttachment { path: dest }],
                Err(e) => {
                    tracing::warn!(event_id, error = %e, "Matrix: write image failed");
                    vec![]
                }
            }
        }
        Err(e) => {
            tracing::warn!(event_id, error = %e, "Matrix: download image failed");
            vec![]
        }
    }
}

async fn download_matrix_doc(
    client: &Client,
    source: &MediaSource,
    event_id: &str,
    ext: &str,
    file_name: &str,
) -> Vec<DocAttachment> {
    let supported = matches!(
        ext.to_lowercase().as_str(),
        "pdf" | "txt" | "csv" | "md" | "json" | "docx" | "doc" | "xlsx" | "xls"
    );
    if !supported {
        return vec![];
    }
    let req = MediaRequestParameters {
        source: source.clone(),
        format: MediaFormat::File,
    };
    match client.media().get_media_content(&req, false).await {
        Ok(bytes) => {
            let safe_id = event_id.replace(['/', '$', ':'], "_");
            let dest = format!("/tmp/garudust_matrix_{safe_id}.{ext}");
            match tokio::fs::write(&dest, &bytes).await {
                Ok(()) => vec![DocAttachment {
                    path: dest,
                    file_name: file_name.to_string(),
                }],
                Err(e) => {
                    tracing::warn!(event_id, error = %e, "Matrix: write doc failed");
                    vec![]
                }
            }
        }
        Err(e) => {
            tracing::warn!(event_id, error = %e, "Matrix: download doc failed");
            vec![]
        }
    }
}

pub struct MatrixAdapter {
    homeserver: String,
    username: String,
    password: String,
    client: Arc<OnceCell<Client>>,
    task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl MatrixAdapter {
    pub fn new(homeserver: String, username: String, password: String) -> Self {
        Self {
            homeserver,
            username,
            password,
            client: Arc::new(OnceCell::new()),
            task: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl PlatformAdapter for MatrixAdapter {
    fn name(&self) -> &'static str {
        "matrix"
    }

    async fn stop(&self) {
        if let Some(h) = self.task.lock().unwrap().take() {
            h.abort();
        }
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

        let client_for_handler = client.clone();
        client.add_event_handler(move |ev: OriginalSyncRoomMessageEvent, _room: Room| {
            let handler = handler.clone();
            let bot_uid = bot_user_id.clone();
            let dl_client = client_for_handler.clone();
            async move {
                if bot_uid.as_ref().is_some_and(|id| id == &ev.sender) {
                    return;
                }
                let room_id = _room.room_id().to_string();
                let user_id = ev.sender.to_string();
                let user_name = ev.sender.localpart().to_string();
                let session_key = format!("matrix:{room_id}");

                let (text, attachments, doc_attachments) = match ev.content.msgtype {
                    MessageType::Text(c) => (c.body, vec![], vec![]),
                    MessageType::Image(c) => {
                        let atts = download_matrix_image(
                            &dl_client,
                            &c.source,
                            ev.event_id.as_ref(),
                            "jpg",
                        )
                        .await;
                        (String::new(), atts, vec![])
                    }
                    MessageType::File(c) => {
                        let ext = std::path::Path::new(&c.body)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("bin");
                        let docs = download_matrix_doc(
                            &dl_client,
                            &c.source,
                            ev.event_id.as_ref(),
                            ext,
                            &c.body,
                        )
                        .await;
                        (String::new(), vec![], docs)
                    }
                    _ => return,
                };

                if text.is_empty() && attachments.is_empty() && doc_attachments.is_empty() {
                    return;
                }

                let inbound = InboundMessage {
                    channel: ChannelId {
                        platform: "matrix".into(),
                        chat_id: room_id,
                        thread_id: None,
                    },
                    user_id,
                    user_name,
                    text,
                    session_key,
                    is_group: true,
                    bot_mentioned: None,
                    attachments,
                    doc_attachments,
                };
                let _ = handler.handle(inbound).await;
            }
        });

        // Long-poll sync loop in background
        let handle = tokio::spawn(async move {
            if let Err(e) = client.sync(SyncSettings::default()).await {
                tracing::error!("Matrix sync error: {e}");
            }
        });
        *self.task.lock().unwrap() = Some(handle);

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
