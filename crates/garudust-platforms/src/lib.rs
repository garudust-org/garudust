//! Chat platform adapters for Garudust agents.
//!
//! Each adapter implements [`garudust_core::platform::PlatformAdapter`] and
//! connects the agent to an external messaging platform.  Enable only the
//! platforms you need via Cargo features.
//!
//! # Feature flags
//!
//! | Feature | Platform | Adapter |
//! |---|---|---|
//! | `telegram` *(default)* | Telegram Bot API | [`telegram::TelegramAdapter`] |
//! | `webhook` *(default)* | HTTP Webhook | [`webhook::WebhookAdapter`] |
//! | `discord` | Discord Gateway | [`discord::DiscordAdapter`] |
//! | `slack` | Slack RTM/Events | [`slack::SlackAdapter`] |
//! | `matrix` | Matrix (Element) | [`matrix::MatrixAdapter`] |
//! | `line` | LINE Messaging API | [`line::LineAdapter`] |
//! | `all` | All of the above | — |
//!
//! # Example — running Telegram and a webhook simultaneously
//!
//! ```no_run
//! use std::sync::Arc;
//! use garudust_platforms::{telegram::TelegramAdapter, webhook::WebhookAdapter};
//! use garudust_core::platform::{MessageHandler, PlatformAdapter};
//!
//! async fn start(handler: Arc<dyn MessageHandler>) -> anyhow::Result<()> {
//!     let tg = TelegramAdapter::new(std::env::var("TELEGRAM_TOKEN")?);
//!     let wh = WebhookAdapter::new(3001);
//!     tg.start(handler.clone()).await?;
//!     wh.start(handler).await?;
//!     Ok(())
//! }
//! ```

#[cfg(feature = "telegram")]
pub mod telegram;

#[cfg(feature = "discord")]
pub mod discord;

#[cfg(feature = "webhook")]
pub mod webhook;

#[cfg(feature = "slack")]
pub mod slack;

#[cfg(feature = "matrix")]
pub mod matrix;

#[cfg(feature = "line")]
pub mod line;
