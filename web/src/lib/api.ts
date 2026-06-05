// Typed gateway client. In the browser/web build the SPA is served by the
// gateway itself, so requests use *relative* URLs. Inside the Tauri desktop
// shell the SPA is served from the app's own asset protocol, so the shell
// injects `window.__GARUDUST_GATEWAY__` (the localhost sidecar origin) and we
// prefix every request with it. One code path, both deployments.

declare global {
  interface Window {
    __GARUDUST_GATEWAY__?: string;
  }
}

// Origin like "http://127.0.0.1:38123" in Tauri, or "" (relative) on the web.
let baseUrl = (typeof window !== "undefined" && window.__GARUDUST_GATEWAY__) || "";
export function setBaseUrl(url: string) {
  baseUrl = url.replace(/\/$/, "");
}
export function getBaseUrl(): string {
  return baseUrl;
}

function httpUrl(path: string): string {
  return `${baseUrl}${path}`;
}

function wsUrl(path: string): string {
  if (baseUrl) {
    return baseUrl.replace(/^http/, "ws") + path;
  }
  const proto = location.protocol === "https:" ? "wss" : "ws";
  return `${proto}://${location.host}${path}`;
}

export interface ChatResponse {
  output: string;
  session_id: string;
  iterations: number;
  input_tokens: number;
  output_tokens: number;
}

export interface EnvEntry {
  key: string;
  masked: string;
}

export interface HealthResponse {
  status: string;
  // `platforms` is omitted when no platform adapters are running.
  checks: { db: string; platforms?: Record<string, string> };
}

/** Optional Bearer token (set when the gateway has GARUDUST_API_KEY). */
let authToken: string | null = null;
export function setAuthToken(token: string | null) {
  authToken = token;
}

function authHeaders(extra?: HeadersInit): HeadersInit {
  const h: Record<string, string> = { ...(extra as Record<string, string>) };
  if (authToken) h["Authorization"] = `Bearer ${authToken}`;
  return h;
}

export async function getHealth(): Promise<HealthResponse> {
  const r = await fetch(httpUrl("/health"));
  if (!r.ok && r.status !== 503) throw new Error(`GET /health failed: ${r.status}`);
  return r.json();
}

export async function getConfig(): Promise<Record<string, unknown>> {
  const r = await fetch(httpUrl("/api/config"), { headers: authHeaders() });
  if (!r.ok) throw new Error(`GET /api/config failed: ${r.status}`);
  return r.json();
}

export async function putConfig(config: Record<string, unknown>): Promise<void> {
  const r = await fetch(httpUrl("/api/config"), {
    method: "PUT",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify(config),
  });
  if (!r.ok) throw new Error(`PUT /api/config failed: ${r.status} ${await r.text()}`);
}

export async function getEnv(): Promise<EnvEntry[]> {
  const r = await fetch(httpUrl("/api/env"), { headers: authHeaders() });
  if (!r.ok) throw new Error(`GET /api/env failed: ${r.status}`);
  return r.json();
}

export async function setEnv(key: string, value: string): Promise<void> {
  const r = await fetch(httpUrl("/api/env"), {
    method: "PUT",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify({ key, value }),
  });
  if (!r.ok) throw new Error(`PUT /api/env failed: ${r.status} ${await r.text()}`);
}

export async function deleteEnv(key: string): Promise<void> {
  const r = await fetch(httpUrl("/api/env"), {
    method: "DELETE",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify({ key }),
  });
  if (!r.ok) throw new Error(`DELETE /api/env failed: ${r.status} ${await r.text()}`);
}

export interface ChatStreamHandlers {
  onDelta: (text: string) => void;
  onDone: () => void;
  onError: (err: string) => void;
}

export interface ChatStreamOptions {
  sessionKey?: string;
  hint?: string;
}

/**
 * Open a streaming chat over the gateway WebSocket. Returns a disposer that
 * closes the socket. The gateway sends raw text deltas, then a final
 * `{"done":true}` frame to signal completion.
 */
export function chatStream(
  message: string,
  handlers: ChatStreamHandlers,
  opts: ChatStreamOptions = {},
): () => void {
  const ws = new WebSocket(wsUrl("/chat/ws"));

  ws.onopen = () => {
    ws.send(
      JSON.stringify({
        message,
        session_key: opts.sessionKey,
        hint: opts.hint,
      }),
    );
  };

  ws.onmessage = (ev) => {
    const data = typeof ev.data === "string" ? ev.data : "";
    // The done sentinel is exactly this JSON object; anything else is a delta.
    if (data.trim() === '{"done":true}') {
      handlers.onDone();
      ws.close();
      return;
    }
    handlers.onDelta(data);
  };

  ws.onerror = () => handlers.onError("websocket error");
  ws.onclose = (ev) => {
    if (!ev.wasClean) handlers.onError("connection closed");
  };

  return () => ws.close();
}
