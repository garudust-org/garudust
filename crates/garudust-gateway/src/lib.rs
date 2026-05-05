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
//!
//! # Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use garudust_gateway::{create_router, AppState};
//!
//! async fn serve(state: AppState) -> anyhow::Result<()> {
//!     let app      = create_router(Arc::new(state));
//!     let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
//!     axum::serve(listener, app).await?;
//!     Ok(())
//! }
//! ```

pub mod handler;
pub mod metrics;
pub mod router;
pub mod sessions;
pub mod state;

pub use handler::GatewayHandler;
pub use metrics::Metrics;
pub use router::create_router;
pub use sessions::SessionRegistry;
pub use state::AppState;
