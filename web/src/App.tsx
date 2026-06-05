import { useEffect, useState } from "react";
import ChatPage from "./pages/ChatPage";
import StatusPage from "./pages/StatusPage";
import ConfigPage from "./pages/ConfigPage";
import EnvPage from "./pages/EnvPage";
import ErrorBoundary from "./components/ErrorBoundary";
import { getHealth } from "./lib/api";

type Page = "chat" | "status" | "config" | "env";
type Conn = "connecting" | "online" | "offline";

const NAV: { id: Page; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "status", label: "Status" },
  { id: "config", label: "Config" },
  { id: "env", label: "Secrets" },
];

export default function App() {
  const [page, setPage] = useState<Page>("chat");
  // Track the gateway/sidecar so the first paint waits for it to come up and a
  // later drop is surfaced instead of failing every action silently.
  const [conn, setConn] = useState<Conn>("connecting");

  useEffect(() => {
    let alive = true;
    const ping = async () => {
      try {
        await getHealth();
        if (alive) setConn("online");
      } catch {
        // Stay on the splash until the first success; after that, show offline.
        if (alive) setConn((c) => (c === "online" ? "offline" : "connecting"));
      }
    };
    ping();
    const t = setInterval(ping, 4000);
    return () => {
      alive = false;
      clearInterval(t);
    };
  }, []);

  if (conn === "connecting") {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 text-neutral-400">
        <div className="text-2xl">🪶</div>
        <div className="animate-pulse text-sm">Connecting to Garudust…</div>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {conn === "offline" && (
        <div className="bg-red-900/70 px-4 py-1.5 text-center text-xs text-red-100">
          Lost connection to the agent server — retrying…
        </div>
      )}
      <div className="flex min-h-0 flex-1">
      <aside className="flex w-52 flex-col border-r border-neutral-800 bg-neutral-950 p-3">
        <div className="mb-6 px-2 pt-2 text-lg font-semibold tracking-tight">🪶 Garudust</div>
        <nav className="flex flex-col gap-1">
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => setPage(item.id)}
              className={
                "rounded-lg px-3 py-2 text-left text-sm " +
                (page === item.id
                  ? "bg-neutral-800 text-neutral-50"
                  : "text-neutral-300 hover:bg-neutral-900")
              }
            >
              {item.label}
            </button>
          ))}
        </nav>
        <div className="mt-auto px-2 text-xs text-neutral-600">v0.13.6</div>
      </aside>

      <main className="flex-1 overflow-hidden">
        {/* key={page} remounts the boundary on navigation so a crashed page
            recovers when you switch away and back. */}
        <ErrorBoundary key={page}>
          {page === "chat" && <ChatPage />}
          {page === "status" && (
            <div className="h-full overflow-y-auto">
              <StatusPage />
            </div>
          )}
          {page === "config" && (
            <div className="h-full overflow-y-auto">
              <ConfigPage />
            </div>
          )}
          {page === "env" && (
            <div className="h-full overflow-y-auto">
              <EnvPage />
            </div>
          )}
        </ErrorBoundary>
      </main>
      </div>
    </div>
  );
}
