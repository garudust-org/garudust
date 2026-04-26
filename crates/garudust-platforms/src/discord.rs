use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
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
}

impl DiscordAdapter {
    pub fn new(token: String) -> Self {
        Self {
            token,
            ctx_store: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &'static str {
        "discord"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let token = self.token.clone();
        let ctx_store = self.ctx_store.clone();
        tokio::spawn(async move {
            let intents = GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;
            let mut client = Client::builder(&token, intents)
                .event_handler(DiscordHandler { handler, ctx_store })
                .await
                .expect("Discord client failed");
            client.start().await.expect("Discord start failed");
        });
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
