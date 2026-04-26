use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, InboundMessage, OutboundMessage},
};
use teloxide::prelude::*;

pub struct TelegramAdapter {
    bot: Bot,
}

impl TelegramAdapter {
    pub fn new(token: String) -> Self {
        Self {
            bot: Bot::new(token),
        }
    }
}

#[async_trait]
impl PlatformAdapter for TelegramAdapter {
    fn name(&self) -> &'static str {
        "telegram"
    }

    async fn start(&self, handler: Arc<dyn MessageHandler>) -> Result<(), PlatformError> {
        let bot = self.bot.clone();
        tokio::spawn(async move {
            teloxide::repl(bot, move |_bot: Bot, msg: Message| {
                let handler = handler.clone();
                async move {
                    if let Some(text) = msg.text() {
                        let inbound = InboundMessage {
                            channel: ChannelId {
                                platform: "telegram".into(),
                                chat_id: msg.chat.id.to_string(),
                                thread_id: None,
                            },
                            user_id: msg
                                .from
                                .as_ref()
                                .map(|u| u.id.to_string())
                                .unwrap_or_default(),
                            user_name: msg
                                .from
                                .as_ref()
                                .and_then(|u| u.username.clone())
                                .unwrap_or_default(),
                            text: text.to_string(),
                            session_key: format!("telegram:{}", msg.chat.id),
                        };
                        let _ = handler.handle(inbound).await;
                    }
                    respond(())
                }
            })
            .await;
        });
        Ok(())
    }

    async fn send_message(
        &self,
        channel: &ChannelId,
        message: OutboundMessage,
    ) -> Result<(), PlatformError> {
        let chat_id: i64 = channel
            .chat_id
            .parse()
            .map_err(|_| PlatformError::Send("invalid chat_id".into()))?;
        self.bot
            .send_message(ChatId(chat_id), &message.text)
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
