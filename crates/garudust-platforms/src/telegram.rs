use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use garudust_core::{
    error::PlatformError,
    platform::{MessageHandler, PlatformAdapter},
    types::{ChannelId, DocAttachment, ImageAttachment, InboundMessage, OutboundMessage},
};
use teloxide::{net::Download, prelude::*};

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
            teloxide::repl(bot, move |bot: Bot, msg: Message| {
                let handler = handler.clone();
                async move {
                    let is_group = msg.chat.is_group() || msg.chat.is_supergroup();
                    let user_id = msg
                        .from
                        .as_ref()
                        .map(|u| u.id.to_string())
                        .unwrap_or_default();
                    let user_name = msg
                        .from
                        .as_ref()
                        .map(|u| {
                            let mut name = u.first_name.clone();
                            if let Some(last) = &u.last_name {
                                name.push(' ');
                                name.push_str(last);
                            }
                            name
                        })
                        .unwrap_or_default();
                    let chat_id = msg.chat.id;

                    let mut attachments: Vec<ImageAttachment> = Vec::new();
                    let mut doc_attachments: Vec<DocAttachment> = Vec::new();

                    // Photos
                    if let Some(photos) = msg.photo() {
                        if let Some(photo) = photos.last() {
                            let file_id = photo.file.id.to_string();
                            let dest = format!("/tmp/garudust_telegram_{file_id}.jpg");
                            match bot.get_file(photo.file.id.clone()).await {
                                Ok(tg_file) => match tokio::fs::File::create(&dest).await {
                                    Ok(mut writer) => {
                                        match bot.download_file(&tg_file.path, &mut writer).await {
                                            Ok(()) => {
                                                attachments.push(ImageAttachment { path: dest });
                                            }
                                            Err(e) => {
                                                tracing::warn!(
                                                    file_id = %file_id,
                                                    error = %e,
                                                    "Telegram: download_file failed"
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            file_id = %file_id,
                                            error = %e,
                                            "Telegram: create tmp file failed"
                                        );
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!(
                                        file_id = %file_id,
                                        error = %e,
                                        "Telegram: get_file failed"
                                    );
                                }
                            }
                        }
                    }

                    // Documents (PDF, TXT, CSV, DOCX, …)
                    if let Some(doc) = msg.document() {
                        let file_name = doc
                            .file_name
                            .clone()
                            .unwrap_or_else(|| format!("{}.bin", doc.file.id));
                        let ext = std::path::Path::new(&file_name)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("bin");
                        let supported = matches!(
                            ext.to_lowercase().as_str(),
                            "pdf" | "txt" | "csv" | "md" | "json" | "docx" | "doc" | "xlsx" | "xls"
                        );
                        if supported {
                            let file_id = doc.file.id.to_string();
                            let dest = format!("/tmp/garudust_telegram_{file_id}.{ext}");
                            match bot.get_file(doc.file.id.clone()).await {
                                Ok(tg_file) => match tokio::fs::File::create(&dest).await {
                                    Ok(mut writer) => {
                                        match bot.download_file(&tg_file.path, &mut writer).await {
                                            Ok(()) => doc_attachments.push(DocAttachment {
                                                path: dest,
                                                file_name,
                                            }),
                                            Err(e) => tracing::warn!(
                                                file_id,
                                                error = %e,
                                                "Telegram: doc download_file failed"
                                            ),
                                        }
                                    }
                                    Err(e) => tracing::warn!(
                                        file_id,
                                        error = %e,
                                        "Telegram: doc create tmp failed"
                                    ),
                                },
                                Err(e) => tracing::warn!(
                                    file_id,
                                    error = %e,
                                    "Telegram: doc get_file failed"
                                ),
                            }
                        }
                    }

                    let text = msg.text().unwrap_or("").to_string();

                    if text.is_empty() && attachments.is_empty() && doc_attachments.is_empty() {
                        return respond(());
                    }

                    let inbound = InboundMessage {
                        channel: ChannelId {
                            platform: "telegram".into(),
                            chat_id: chat_id.to_string(),
                            thread_id: None,
                        },
                        user_id,
                        user_name,
                        text,
                        session_key: format!("telegram:{chat_id}"),
                        is_group,
                        bot_mentioned: None,
                        attachments,
                        doc_attachments,
                    };
                    let _ = handler.handle(inbound).await;
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
