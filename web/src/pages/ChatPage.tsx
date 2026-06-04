import { useEffect, useRef, useState } from "react";
import Markdown from "react-markdown";
import { chatStream } from "../lib/api";

interface Msg {
  role: "user" | "assistant";
  content: string;
}

// One session key per page load gives conversation continuity across turns
// (the gateway threads history by session_key).
const SESSION_KEY = crypto.randomUUID();

export default function ChatPage() {
  const [messages, setMessages] = useState<Msg[]>([]);
  const [input, setInput] = useState("");
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);

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

    chatStream(
      text,
      {
        onDelta: appendToAssistant,
        onDone: () => setStreaming(false),
        onError: (err) => {
          setError(err);
          setStreaming(false);
        },
      },
      { sessionKey: SESSION_KEY },
    );
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
          <button
            className="rounded-xl bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
            onClick={send}
            disabled={streaming || !input.trim()}
          >
            {streaming ? "…" : "Send"}
          </button>
        </div>
      </div>
    </div>
  );
}
