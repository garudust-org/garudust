use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Request, State,
    },
    http::StatusCode,
    middleware,
    response::{
        sse::{Event, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer};

use crate::state::AppState;

/// Bearer token middleware — rejects requests to /chat* if a key is configured
/// and the Authorization header does not match.
async fn require_auth(
    State(state): State<AppState>,
    req: Request<Body>,
    next: middleware::Next,
) -> Response {
    if let Some(expected) = &state.config.security.gateway_api_key {
        let provided = req
            .headers()
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));
        if provided != Some(expected.as_str()) {
            return (StatusCode::UNAUTHORIZED, "Unauthorized\n").into_response();
        }
    }
    next.run(req).await
}

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
    let approver = state.approver.clone();
    let result = state
        .agent
        .load_full()
        .run(&req.message, approver, "http")
        .await;
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

    let approver = state.approver.clone();
    tokio::spawn(async move {
        let _ = state
            .agent
            .load_full()
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

    let approver2 = state2.approver.clone();
    tokio::spawn(async move {
        let _ = state2
            .agent
            .load_full()
            .run_streaming(&message, approver2, "ws", chunk_tx)
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
    let rate_limit_rpm = state.config.security.rate_limit_rpm;

    // /chat* routes require Bearer token auth (if GARUDUST_API_KEY is set)
    let protected = Router::new()
        .route("/chat", post(chat))
        .route("/chat/stream", post(chat_stream))
        .route("/chat/ws", get(chat_ws))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    // /metrics is protected by the same Bearer token when a key is configured;
    // /health is always open so load balancers and health probes still work.
    let metrics_route = Router::new().route("/metrics", get(metrics));
    let metrics_route = if state.config.security.gateway_api_key.is_some() {
        metrics_route.route_layer(middleware::from_fn_with_state(state.clone(), require_auth))
    } else {
        metrics_route
    };

    let mut router = Router::new()
        .route("/health", get(health))
        .merge(metrics_route)
        .merge(protected)
        .layer(ConcurrencyLimitLayer::new(concurrency_limit))
        .with_state(state);

    if let Some(rpm) = rate_limit_rpm {
        let burst = std::cmp::max(1, rpm / 10);
        let period_ms = u64::from(std::cmp::max(1, 60_000 / rpm));
        let governor_conf = Arc::new(
            GovernorConfigBuilder::default()
                .per_millisecond(period_ms)
                .burst_size(burst)
                .finish()
                .expect("valid governor config"),
        );
        router = router.layer(GovernorLayer::new(governor_conf));
    }

    router
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
            agent: Arc::new(arc_swap::ArcSwap::from(agent)),
            metrics: Arc::new(crate::metrics::Metrics::default()),
            approver: Arc::new(garudust_agent::AutoApprover),
        }
    }

    fn test_state_with_key(key: &str) -> AppState {
        use garudust_core::config::SecurityConfig;
        use std::sync::Arc;

        let mut state = test_state();
        let mut config = (*state.config).clone();
        config.security = SecurityConfig {
            gateway_api_key: Some(key.to_string()),
            ..config.security
        };
        state.config = Arc::new(config);
        state
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

    #[tokio::test]
    async fn metrics_open_without_api_key() {
        let router = create_router(test_state());
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn metrics_blocked_without_token_when_key_set() {
        let router = create_router(test_state_with_key("secret"));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn metrics_accessible_with_correct_token() {
        let router = create_router(test_state_with_key("secret"));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("Authorization", "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
    }

    #[tokio::test]
    async fn health_always_open_even_with_key_set() {
        let router = create_router(test_state_with_key("secret"));
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
    }
}
