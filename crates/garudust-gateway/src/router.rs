use axum::{routing::get, Router};

use crate::state::AppState;

async fn health() -> &'static str {
    "ok"
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}
