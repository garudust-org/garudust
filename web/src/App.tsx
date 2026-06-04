import { useState } from "react";
import ChatPage from "./pages/ChatPage";

type Page = "chat" | "status" | "config" | "env";

const NAV: { id: Page; label: string; ready: boolean }[] = [
  { id: "chat", label: "Chat", ready: true },
  { id: "status", label: "Status", ready: false },
  { id: "config", label: "Config", ready: false },
  { id: "env", label: "Env", ready: false },
];

export default function App() {
  const [page, setPage] = useState<Page>("chat");

  return (
    <div className="flex h-full">
      <aside className="flex w-52 flex-col border-r border-neutral-800 bg-neutral-950 p-3">
        <div className="mb-6 px-2 pt-2 text-lg font-semibold tracking-tight">
          🪶 Garudust
        </div>
        <nav className="flex flex-col gap-1">
          {NAV.map((item) => (
            <button
              key={item.id}
              onClick={() => item.ready && setPage(item.id)}
              disabled={!item.ready}
              className={
                "rounded-lg px-3 py-2 text-left text-sm " +
                (page === item.id
                  ? "bg-neutral-800 text-neutral-50"
                  : item.ready
                    ? "text-neutral-300 hover:bg-neutral-900"
                    : "cursor-not-allowed text-neutral-600")
              }
            >
              {item.label}
              {!item.ready && <span className="ml-1 text-[10px] uppercase">soon</span>}
            </button>
          ))}
        </nav>
        <div className="mt-auto px-2 text-xs text-neutral-600">v0.13.2</div>
      </aside>

      <main className="flex-1 overflow-hidden">
        {page === "chat" ? (
          <ChatPage />
        ) : (
          <div className="flex h-full items-center justify-center text-neutral-500">
            {page} page — coming in a later phase
          </div>
        )}
      </main>
    </div>
  );
}
