//! Typed gateway client — mirrors `lib/api.ts` in the old React build.
//!
//! URL resolution mirrors the TypeScript version: if the Tauri shell has
//! injected `window.__GARUDUST_GATEWAY__` (the sidecar origin, e.g.
//! `http://127.0.0.1:52157`) we prefix every request with it; otherwise we use
//! relative URLs (works when the SPA is served by the gateway itself in the
//! embedded `web-ui` feature build).

use js_sys::Reflect;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::WebSocket;

// ── URL helpers ───────────────────────────────────────────────────────────

fn gateway_origin() -> String {
    if let Some(w) = web_sys::window() {
        if let Ok(v) = Reflect::get(&w, &wasm_bindgen::JsValue::from_str("__GARUDUST_GATEWAY__"))
        {
            if let Some(s) = v.as_string() {
                return s.trim_end_matches('/').to_string();
            }
        }
    }
    String::new()
}

pub fn http_url(path: &str) -> String {
    format!("{}{path}", gateway_origin())
}

pub fn ws_url(path: &str) -> String {
    let origin = gateway_origin();
    if origin.is_empty() {
        // Same-origin: swap scheme based on the page protocol.
        if let Some(w) = web_sys::window() {
            let proto = w.location().protocol().unwrap_or_default();
            let host = w.location().host().unwrap_or_default();
            let ws_proto = if proto == "https:" { "wss" } else { "ws" };
            return format!("{ws_proto}://{host}{path}");
        }
        format!("ws://localhost{path}")
    } else {
        origin.replacen("http", "ws", 1) + path
    }
}

// ── Request helper ────────────────────────────────────────────────────────

async fn api_get(path: &str) -> Result<gloo_net::http::Response, String> {
    gloo_net::http::Request::get(&http_url(path))
        .send()
        .await
        .map_err(|e| e.to_string())
}

async fn api_put(path: &str, body: &str) -> Result<gloo_net::http::Response, String> {
    gloo_net::http::Request::put(&http_url(path))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())
}

async fn api_delete(path: &str, body: &str) -> Result<gloo_net::http::Response, String> {
    gloo_net::http::Request::delete(&http_url(path))
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())
}

// ── Health ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub checks: HealthChecks,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct HealthChecks {
    pub db: String,
    // Omitted when no platform adapters run (serde skip_serializing_if).
    #[serde(default)]
    pub platforms: std::collections::HashMap<String, String>,
}

pub async fn get_health() -> Result<HealthResponse, String> {
    let resp = api_get("/health").await?;
    resp.json::<HealthResponse>().await.map_err(|e| e.to_string())
}

// ── Config ────────────────────────────────────────────────────────────────

pub async fn get_config() -> Result<serde_json::Value, String> {
    let resp = api_get("/api/config").await?;
    if !resp.ok() {
        return Err(format!("GET /api/config {}", resp.status()));
    }
    resp.json::<serde_json::Value>().await.map_err(|e| e.to_string())
}

pub async fn put_config(config: &serde_json::Value) -> Result<(), String> {
    let body = serde_json::to_string(config).map_err(|e| e.to_string())?;
    let resp = api_put("/api/config", &body).await?;
    if !resp.ok() {
        return Err(format!("PUT /api/config {}", resp.status()));
    }
    Ok(())
}

// ── Env ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EnvEntry {
    pub key: String,
    pub masked: String,
}

pub async fn get_env() -> Result<Vec<EnvEntry>, String> {
    let resp = api_get("/api/env").await?;
    if !resp.ok() {
        return Err(format!("GET /api/env {}", resp.status()));
    }
    resp.json::<Vec<EnvEntry>>().await.map_err(|e| e.to_string())
}

pub async fn set_env(key: &str, value: &str) -> Result<(), String> {
    let body = serde_json::json!({ "key": key, "value": value }).to_string();
    let resp = api_put("/api/env", &body).await?;
    if !resp.ok() {
        let msg = resp.text().await.unwrap_or_default();
        return Err(format!("PUT /api/env {}: {msg}", resp.status()));
    }
    Ok(())
}

pub async fn delete_env(key: &str) -> Result<(), String> {
    let body = serde_json::json!({ "key": key }).to_string();
    let resp = api_delete("/api/env", &body).await?;
    if !resp.ok() {
        return Err(format!("DELETE /api/env {}", resp.status()));
    }
    Ok(())
}

// ── Chat WebSocket ────────────────────────────────────────────────────────

use wasm_bindgen::prelude::Closure;

/// JS-function version of chat_stream: callbacks are `js_sys::Function` so no
/// Send/Sync constraints are needed (WASM is single-threaded).
pub fn chat_stream_js(
    message: String,
    session_key: String,
    hint: Option<String>,
    on_delta: js_sys::Function,
    on_done: js_sys::Function,
    on_error: js_sys::Function,
) -> js_sys::Function {
    let url = ws_url("/chat/ws");
    let ws = WebSocket::new(&url).expect("WebSocket::new");

    // onopen → send payload
    let ws_send = ws.clone();
    let mut payload = serde_json::json!({ "message": message, "session_key": session_key });
    if let Some(h) = hint {
        payload["hint"] = serde_json::Value::String(h);
    }
    let payload_str = payload.to_string();
    let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
        ws_send.send_with_str(&payload_str).unwrap_or_default();
    }) as Box<dyn FnMut(web_sys::Event)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // onmessage
    let on_delta2 = on_delta.clone();
    let on_done2 = on_done.clone();
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        let data = e.data().as_string().unwrap_or_default();
        if data.trim() == r#"{"done":true}"# {
            let _ = on_done2.call0(&wasm_bindgen::JsValue::NULL);
        } else {
            let _ = on_delta2.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str(&data));
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // onerror
    let onerror = Closure::wrap(Box::new(move |_: web_sys::ErrorEvent| {
        let _ = on_error.call1(&wasm_bindgen::JsValue::NULL, &wasm_bindgen::JsValue::from_str("WebSocket error"));
    }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    // Return a JS function that closes the socket (usable as disposer).
    let ws_close = ws;
    let close_fn = Closure::wrap(Box::new(move || {
        let _ = ws_close.close();
    }) as Box<dyn FnMut()>);
    let f = close_fn.as_ref().unchecked_ref::<js_sys::Function>().clone();
    close_fn.forget();
    f
}

/// Typed Rust version (kept for reference; use chat_stream_js from components).
pub fn chat_stream(
    message: String,
    session_key: String,
    hint: Option<String>,
    on_delta: impl Fn(String) + 'static,
    on_done: impl Fn() + 'static,
    on_error: impl Fn(String) + 'static,
) -> impl FnOnce() {
    let url = ws_url("/chat/ws");
    let ws = WebSocket::new(&url).expect("WebSocket::new");

    // onopen → send the payload
    let ws_send = ws.clone();
    let mut payload = serde_json::json!({
        "message": message,
        "session_key": session_key,
    });
    if let Some(h) = hint {
        payload["hint"] = serde_json::Value::String(h);
    }
    let payload_str = payload.to_string();
    let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
        ws_send.send_with_str(&payload_str).unwrap_or_default();
    }) as Box<dyn FnMut(web_sys::Event)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
    onopen.forget();

    // onmessage → call delta/done handlers
    let onmessage = Closure::wrap(Box::new(move |e: web_sys::MessageEvent| {
        let data = e.data().as_string().unwrap_or_default();
        if data.trim() == r#"{"done":true}"# {
            on_done();
        } else {
            on_delta(data);
        }
    }) as Box<dyn FnMut(web_sys::MessageEvent)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();

    // onerror
    let onerror = Closure::wrap(Box::new(move |_: web_sys::ErrorEvent| {
        on_error("WebSocket error".to_string());
    }) as Box<dyn FnMut(web_sys::ErrorEvent)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    // Disposer
    let ws_close = ws;
    move || {
        let _ = ws_close.close();
    }
}
