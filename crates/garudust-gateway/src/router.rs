use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use garudust_agent::AutoApprover;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tower::limit::ConcurrencyLimitLayer;

use crate::state::AppState;

async fn health() -> &'static str {
    "ok"
}

async fn metrics(State(state): State<AppState>) -> Response {
    let body = state.metrics.prometheus_text();
    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        body,
    )
        .into_response()
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
    state.metrics.inc_request();
    let approver = Arc::new(AutoApprover);
    let result = state.agent.run(&req.message, approver, "http").await;
    state.metrics.dec_active();
    result
        .map(|r| {
            state
                .metrics
                .add_tokens(r.usage.input_tokens, r.usage.output_tokens);
            Json(ChatResponse {
                output: r.output,
                session_id: r.session_id,
                iterations: r.iterations,
                input_tokens: r.usage.input_tokens,
                output_tokens: r.usage.output_tokens,
            })
        })
        .map_err(|e| {
            state.metrics.inc_error();
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })
}

async fn chat_stream(
    State(state): State<AppState>,
    Json(req): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let (chunk_tx, chunk_rx) = mpsc::unbounded_channel::<String>();

    tokio::spawn(async move {
        let approver = Arc::new(AutoApprover);
        let _ = state
            .agent
            .run_streaming(&req.message, approver, "http-sse", chunk_tx)
            .await;
    });

    let stream =
        UnboundedReceiverStream::new(chunk_rx).map(|delta| Ok(Event::default().data(delta)));

    Sse::new(stream)
}

async fn chat_ws(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let Some(Ok(Message::Text(task))) = socket.recv().await else {
        return;
    };

    let message = serde_json::from_str::<serde_json::Value>(&task)
        .ok()
        .and_then(|v| v["message"].as_str().map(String::from))
        .unwrap_or_else(|| task.to_string());

    state.metrics.inc_request();
    let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<String>();
    let state2 = state.clone();

    tokio::spawn(async move {
        let approver = Arc::new(AutoApprover);
        let _ = state2
            .agent
            .run_streaming(&message, approver, "ws", chunk_tx)
            .await;
    });

    while let Some(chunk) = chunk_rx.recv().await {
        if socket.send(Message::Text(chunk.into())).await.is_err() {
            break;
        }
    }

    state.metrics.dec_active();
    let _ = socket.send(Message::Text(r#"{"done":true}"#.into())).await;
}

pub fn create_router(state: AppState) -> Router {
    let concurrency_limit = state.config.max_concurrent_requests.unwrap_or(64);
    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics))
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/chat/ws", get(chat_ws))
        .layer(ConcurrencyLimitLayer::new(concurrency_limit))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;
    use crate::state::AppState;

    fn test_state() -> AppState {
        use std::sync::Arc;

        use async_trait::async_trait;
        use futures::stream;
        use garudust_agent::Agent;
        use garudust_core::{
            config::AgentConfig,
            error::AgentError,
            error::TransportError,
            memory::{MemoryContent, MemoryStore},
            transport::{ApiMode, ProviderTransport, StreamResult},
            types::{
                ContentPart, InferenceConfig, Message, StopReason, StreamChunk, TokenUsage,
                ToolSchema, TransportResponse,
            },
        };
        use garudust_memory::SessionDb;
        use garudust_tools::ToolRegistry;

        struct EchoTransport;
        #[async_trait]
        impl ProviderTransport for EchoTransport {
            fn api_mode(&self) -> ApiMode {
                ApiMode::ChatCompletions
            }
            async fn chat(
                &self,
                _m: &[Message],
                _c: &InferenceConfig,
                _t: &[ToolSchema],
            ) -> Result<TransportResponse, TransportError> {
                Ok(TransportResponse {
                    content: vec![ContentPart::Text("ok".into())],
                    tool_calls: vec![],
                    usage: TokenUsage::default(),
                    stop_reason: StopReason::EndTurn,
                })
            }
            async fn chat_stream(
                &self,
                _m: &[Message],
                _c: &InferenceConfig,
                _t: &[ToolSchema],
            ) -> Result<StreamResult, TransportError> {
                let chunks = vec![
                    Ok(StreamChunk::TextDelta("ok".into())),
                    Ok(StreamChunk::Done {
                        usage: TokenUsage::default(),
                    }),
                ];
                Ok(Box::pin(stream::iter(chunks)))
            }
        }

        struct NopMemory;
        #[async_trait]
        impl MemoryStore for NopMemory {
            async fn read_memory(&self) -> Result<MemoryContent, AgentError> {
                Ok(MemoryContent::default())
            }
            async fn write_memory(&self, _: &MemoryContent) -> Result<(), AgentError> {
                Ok(())
            }
            async fn read_user_profile(&self) -> Result<String, AgentError> {
                Ok(String::new())
            }
            async fn write_user_profile(&self, _: &str) -> Result<(), AgentError> {
                Ok(())
            }
        }

        let config = Arc::new(AgentConfig::default());
        let transport = Arc::new(EchoTransport);
        let tools = Arc::new(ToolRegistry::new());
        let memory = Arc::new(NopMemory);
        let tmp = std::env::temp_dir().join(format!("garudust-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp).unwrap();
        let db = Arc::new(SessionDb::open(&tmp).unwrap());
        let agent = Arc::new(
            Agent::new(transport, tools, memory, config.clone()).with_session_db(db.clone()),
        );

        AppState {
            config,
            session_db: db,
            agent,
            metrics: Arc::new(crate::metrics::Metrics::default()),
        }
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let router = create_router(test_state());
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"ok");
    }
}
