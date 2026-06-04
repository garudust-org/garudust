// Typed gateway client. Everything talks to garudust-server over its standard
// HTTP/WS API using *relative* URLs, so the exact same build runs in a browser
// (served by the gateway or via the Vite dev proxy) and inside a future Tauri
// shell pointed at a localhost sidecar.

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

export async function getConfig(): Promise<Record<string, unknown>> {
  const r = await fetch("/api/config", { headers: authHeaders() });
  if (!r.ok) throw new Error(`GET /api/config failed: ${r.status}`);
  return r.json();
}

export async function putConfig(config: Record<string, unknown>): Promise<void> {
  const r = await fetch("/api/config", {
    method: "PUT",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify(config),
  });
  if (!r.ok) throw new Error(`PUT /api/config failed: ${r.status} ${await r.text()}`);
}

export async function getEnv(): Promise<EnvEntry[]> {
  const r = await fetch("/api/env", { headers: authHeaders() });
  if (!r.ok) throw new Error(`GET /api/env failed: ${r.status}`);
  return r.json();
}

export async function setEnv(key: string, value: string): Promise<void> {
  const r = await fetch("/api/env", {
    method: "PUT",
    headers: authHeaders({ "Content-Type": "application/json" }),
    body: JSON.stringify({ key, value }),
  });
  if (!r.ok) throw new Error(`PUT /api/env failed: ${r.status} ${await r.text()}`);
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
  const proto = location.protocol === "https:" ? "wss" : "ws";
  const url = `${proto}://${location.host}/chat/ws`;
  const ws = new WebSocket(url);

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
