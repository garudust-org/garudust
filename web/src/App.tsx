import { useState } from "react";
import ChatPage from "./pages/ChatPage";
import StatusPage from "./pages/StatusPage";
import ConfigPage from "./pages/ConfigPage";
import EnvPage from "./pages/EnvPage";

type Page = "chat" | "status" | "config" | "env";

const NAV: { id: Page; label: string }[] = [
  { id: "chat", label: "Chat" },
  { id: "status", label: "Status" },
  { id: "config", label: "Config" },
  { id: "env", label: "Secrets" },
];

export default function App() {
  const [page, setPage] = useState<Page>("chat");

  return (
    <div className="flex h-full">
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
        <div className="mt-auto px-2 text-xs text-neutral-600">v0.13.4</div>
      </aside>

      <main className="flex-1 overflow-hidden">
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
      </main>
    </div>
  );
}
