use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use garudust_agent::AutoApprover;
use serde::{Deserialize, Serialize};

use crate::state::AppState;

async fn health() -> &'static str {
    "ok"
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
}

#[derive(Serialize)]
struct ChatResponse {
    output: String,
    session_id: String,
    iterations: u32,
    input_tokens: u32,
    output_tokens: u32,
}

async fn chat(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    let approver = Arc::new(AutoApprover);
    state
        .agent
        .run(&req.message, approver, "http")
        .await
        .map(|r| {
            Json(ChatResponse {
                output: r.output,
                session_id: r.session_id,
                iterations: r.iterations,
                input_tokens: r.usage.input_tokens,
                output_tokens: r.usage.output_tokens,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/chat", post(chat))
        .with_state(state)
}
