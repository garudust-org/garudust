import { useEffect, useRef, useState } from "react";
import Markdown from "react-markdown";
import { chatStream, getConfig } from "../lib/api";

interface Msg {
  role: "user" | "assistant";
  content: string;
}

export default function ChatPage() {
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // A session key threads history on the gateway; "New chat" rotates it.
  const [sessionKey, setSessionKey] = useState(() => crypto.randomUUID());
  // Disposer for the active stream, so "Stop" can close the socket.
  const disposeRef = useRef<(() => void) | null>(null);
  // Runtime model selection: "" = default model, otherwise a routing-hint name
  // (config.routing maps hint → "provider/model"). Sent as `hint` per message.
  const [routing, setRouting] = useState<Record<string, string>>({});
  const [defaultModel, setDefaultModel] = useState("");
  const [hint, setHint] = useState("");
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    getConfig()
      .then((c) => {
        setRouting((c.routing as Record<string, string>) ?? {});
        setDefaultModel((c.model as string) ?? "");
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  function send() {
    const text = input.trim();
    if (!text || streaming) return;
    setError(null);
    setInput("");
    setMessages((m) => [...m, { role: "user", content: text }, { role: "assistant", content: "" }]);
    setStreaming(true);

    const appendToAssistant = (delta: string) =>
      setMessages((m) => {
        const next = [...m];
        next[next.length - 1] = {
          role: "assistant",
          content: next[next.length - 1].content + delta,
        };
        return next;
      });

    disposeRef.current = chatStream(
      text,
      {
        onDelta: appendToAssistant,
        onDone: () => setStreaming(false),
        onError: (err) => {
          setError(err);
          setStreaming(false);
        },
      },
      { sessionKey, hint: hint || undefined },
    );
  }

  function stop() {
    disposeRef.current?.();
    disposeRef.current = null;
    setStreaming(false);
  }

  function newSession() {
    stop();
    setMessages([]);
    setError(null);
    setSessionKey(crypto.randomUUID());
  }

  function onKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex-1 overflow-y-auto px-4 py-6">
        <div className="mx-auto flex max-w-3xl flex-col gap-4">
          {messages.length === 0 && (
            <div className="mt-20 text-center text-neutral-500">
              Ask Garudust anything — responses stream over the gateway WebSocket.
            </div>
          )}
          {messages.map((m, i) => (
            <div
              key={i}
              className={m.role === "user" ? "flex justify-end" : "flex justify-start"}
            >
              <div
                className={
                  m.role === "user"
                    ? "max-w-[80%] rounded-2xl bg-amber-500/90 px-4 py-2 text-neutral-950"
                    : "markdown max-w-[80%] rounded-2xl bg-neutral-800/80 px-4 py-2 text-neutral-100"
                }
              >
                {m.role === "assistant" ? (
                  m.content ? (
                    <Markdown>{m.content}</Markdown>
                  ) : (
                    <span className="text-neutral-500">…</span>
                  )
                ) : (
                  m.content
                )}
              </div>
            </div>
          ))}
          {error && (
            <div className="rounded-lg border border-red-800 bg-red-950/50 px-3 py-2 text-sm text-red-300">
              {error}
            </div>
          )}
          <div ref={bottomRef} />
        </div>
      </div>

      <div className="border-t border-neutral-800 bg-neutral-950/80 px-4 py-3">
        <div className="mx-auto mb-2 flex max-w-3xl items-center gap-2">
          <span className="text-xs text-neutral-500">Model</span>
          <select
            className="rounded-lg border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs outline-none focus:border-amber-500"
            value={hint}
            onChange={(e) => setHint(e.target.value)}
            title="Pick a routing hint from config.routing, or use the default model"
          >
            <option value="">Default{defaultModel ? ` · ${defaultModel}` : ""}</option>
            {Object.entries(routing).map(([name, target]) => (
              <option key={name} value={name}>
                {name} · {target}
              </option>
            ))}
          </select>
          {Object.keys(routing).length === 0 && (
            <span className="text-xs text-neutral-600">
              (add a <code>routing:</code> table in Config to switch models)
            </span>
          )}
          <button
            className="ml-auto rounded-lg border border-neutral-700 px-3 py-1 text-xs text-neutral-300 hover:border-amber-500"
            onClick={newSession}
            title="Clear the conversation and start a fresh session"
          >
            New chat
          </button>
        </div>
        <div className="mx-auto flex max-w-3xl items-end gap-2">
          <textarea
            className="flex-1 resize-none rounded-xl border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500"
            rows={1}
            placeholder="Message Garudust…  (Enter to send, Shift+Enter for newline)"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={onKeyDown}
            disabled={streaming}
          />
          {streaming ? (
            <button
              className="rounded-xl border border-neutral-600 px-4 py-2 text-sm font-medium text-neutral-200 hover:border-red-600 hover:text-red-400"
              onClick={stop}
            >
              Stop
            </button>
          ) : (
            <button
              className="rounded-xl bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
              onClick={send}
              disabled={!input.trim()}
            >
              Send
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
