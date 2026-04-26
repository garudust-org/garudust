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

struct DiscordHandler {
    handler: Arc<dyn MessageHandler>,
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

    async fn ready(&self, _ctx: Context, ready: Ready) {
        tracing::info!("Discord bot connected as {}", ready.user.name);
    }
}

pub struct DiscordAdapter {
    token: String,
}

impl DiscordAdapter {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl PlatformAdapter for DiscordAdapter {
    fn name(&self) -> &'static str {
        "discord"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let token = self.token.clone();
        tokio::spawn(async move {
            let intents = GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::MESSAGE_CONTENT;
            let mut client = Client::builder(&token, intents)
                .event_handler(DiscordHandler { handler })
                .await
                .expect("Discord client failed");
            client.start().await.expect("Discord start failed");
        });
        Ok(())
    }

    async fn send_message(
        &self,
        _channel: &ChannelId,
        _message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        // Sending requires ctx from serenity — wired up properly in Phase 6
        Err(PlatformError::Send("send not yet implemented".into()))
    }

    async fn send_stream(
        &self,
        _channel: &ChannelId,
        _stream: Pin<Box<dyn Stream<Item = String> + Send>>,
    ) -> Result<(), PlatformError> {
        Err(PlatformError::Send(
            "stream send not yet implemented".into(),
        ))
    }
}
