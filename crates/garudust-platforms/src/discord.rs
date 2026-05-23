use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use serenity::{
    async_trait as serenity_async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};
use tokio::sync::Mutex;

struct DiscordHandler {
    handler: Arc<dyn MessageHandler>,
    ctx_store: Arc<Mutex<Option<Context>>>,
}

#[serenity_async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot {
            return;
        }

        // Download attachments to /tmp — images and supported document types
        let mut attachments: Vec<ImageAttachment> = Vec::new();
        let mut doc_attachments: Vec<DocAttachment> = Vec::new();

        for att in &msg.attachments {
            let ct = att.content_type.as_deref().unwrap_or("");
            if ct.starts_with("image/") {
                let dest = format!("/tmp/garudust_discord_{}.jpg", att.id);
                match reqwest::get(&att.url).await {
                    Ok(resp) => match resp.bytes().await {
                        Ok(bytes) => match tokio::fs::write(&dest, &bytes).await {
                            Ok(()) => attachments.push(ImageAttachment { path: dest }),
                            Err(e) => {
                                tracing::warn!(att_id = %att.id, error = %e, "Discord: write img failed");
                            }
                        },
                        Err(e) => {
                            tracing::warn!(att_id = %att.id, error = %e, "Discord: read img bytes failed");
                        }
                    },
                    Err(e) => {
                        tracing::warn!(att_id = %att.id, error = %e, "Discord: download img failed");
                    }
                }
            } else {
                let ext = std::path::Path::new(&att.filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("bin");
                let supported = matches!(
                    ext.to_lowercase().as_str(),
                    "pdf" | "txt" | "csv" | "md" | "json" | "docx" | "doc" | "xlsx" | "xls"
                );
                if supported {
                    let dest = format!("/tmp/garudust_discord_{}.{ext}", att.id);
                    match reqwest::get(&att.url).await {
                        Ok(resp) => match resp.bytes().await {
                            Ok(bytes) => match tokio::fs::write(&dest, &bytes).await {
                                Ok(()) => doc_attachments.push(DocAttachment {
                                    path: dest,
                                    file_name: att.filename.clone(),
                                }),
                                Err(e) => {
                                    tracing::warn!(att_id = %att.id, error = %e, "Discord: write doc failed");
                                }
                            },
                            Err(e) => {
                                tracing::warn!(att_id = %att.id, error = %e, "Discord: read doc bytes failed");
                            }
                        },
                        Err(e) => {
                            tracing::warn!(att_id = %att.id, error = %e, "Discord: download doc failed");
                        }
                    }
                }
            }
        }

        if msg.content.is_empty() && attachments.is_empty() && doc_attachments.is_empty() {
            return;
        }

        let inbound = InboundMessage {
            channel: ChannelId {
                platform: "discord".into(),
                chat_id: msg.channel_id.to_string(),
                thread_id: None,
            },
            user_id: msg.author.id.to_string(),
            user_name: msg.author.name.clone(),
            text: msg.content.clone(),
            session_key: format!("discord:{}", msg.channel_id),
            is_group: msg.guild_id.is_some(),
            bot_mentioned: None,
            attachments,
            doc_attachments,
        };
        let _ = self.handler.handle(inbound).await;
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        *self.ctx_store.lock().await = Some(ctx);
        tracing::info!("Discord bot connected as {}", ready.user.name);
    }
}

pub struct DiscordAdapter {
    token: String,
    ctx_store: Arc<Mutex<Option<Context>>>,
    task: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl DiscordAdapter {
    pub fn new(token: String) -> Self {
        Self {
            token,
            ctx_store: Arc::new(Mutex::new(None)),
            task: Arc::new(std::sync::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &'static str {
        "discord"
    }

    async fn stop(&self) {
        if let Some(h) = self.task.lock().unwrap().take() {
            h.abort();
        }
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let token = self.token.clone();
        let ctx_store = self.ctx_store.clone();
        let handle = tokio::spawn(async move {
            let intents = GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;
            let mut client = Client::builder(&token, intents)
                .event_handler(DiscordHandler { handler, ctx_store })
                .await
                .expect("Discord client failed");
            client.start().await.expect("Discord start failed");
        });
        *self.task.lock().unwrap() = Some(handle);
        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        let guard = self.ctx_store.lock().await;
        let ctx = guard
            .as_ref()
            .ok_or_else(|| PlatformError::Send("Discord not connected yet".into()))?;

        let channel_id: u64 = channel
            .chat_id
            .parse()
            .map_err(|_| PlatformError::Send("invalid channel_id".into()))?;

        serenity::model::id::ChannelId::new(channel_id)
            .say(&ctx.http, &message.text)
            .await
            .map_err(|e| PlatformError::Send(e.to_string()))?;
        Ok(())
    }

    async fn send_stream(
        &self,
        channel: &ChannelId,
        mut stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        use futures::StreamExt;
        let mut buf = String::new();
        while let Some(chunk) = stream.next().await {
            buf.push_str(&chunk);
        }
        self.send_message(channel, OutboundMessage::text(buf)).await
    }
}
