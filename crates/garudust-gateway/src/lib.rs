//! Production HTTP API gateway and streaming server for Garudust agents.
//!
//! Exposes the agent over HTTP with rate-limiting, session management,
//! and Server-Sent Events streaming so web clients can display responses
//! token by token.
//!
//! # Endpoints
//!
//! | Method | Path | Description |
//! |---|---|---|
//! | `POST` | `/chat` | Single-turn request/response (JSON) |
//! | `POST` | `/stream` | Streaming response via SSE |
//! | `GET`  | `/ws` | WebSocket bi-directional chat |
//! | `GET`  | `/health` | Health check |
//! | `GET`  | `/metrics` | Prometheus-compatible metrics |
//! | `GET`/`PUT` | `/api/config` | Read / replace `config.yaml` (non-secret) |
//! | `GET`/`PUT` | `/api/env` | List masked secret keys / set a secret (write-only) |
//!
//! With the `web-ui` feature, the embedded dashboard SPA is served on all other
//! paths (see [`static_assets`]).
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use garudust_gateway::{create_router, AppState};
//!
//! async fn serve(state: AppState) -> anyhow::Result<()> {
//!     let app      = create_router(state);
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!     Ok(())
//! }
//! ```

pub mod config_api;
pub mod env_api;
pub mod handler;
pub mod handler_tests;
pub mod interactive;
pub mod metrics;
pub mod router;
pub mod sessions;
pub mod state;
#[cfg(feature = "web-ui")]
pub mod static_assets;

pub use handler::GatewayHandler;
pub use metrics::Metrics;
pub use router::create_router;
pub use sessions::SessionRegistry;
pub use state::AppState;
