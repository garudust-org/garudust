import { useEffect, useState } from "react";
import { deleteEnv, getEnv, setEnv, type EnvEntry } from "../lib/api";

// Common secret keys offered as quick picks; any valid KEY can still be typed.
const COMMON_KEYS = [
  "ANTHROPIC_API_KEY",
  "OPENAI_API_KEY",
  "GROQ_API_KEY",
  "OPENROUTER_API_KEY",
  "GOOGLE_AI_API_KEY",
  "GARUDUST_API_KEY",
  "TELEGRAM_TOKEN",
  "DISCORD_TOKEN",
];

export default function EnvPage() {
  const [entries, setEntries] = useState<EnvEntry[]>([]);
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function refresh() {
    try {
      setEntries(await getEnv());
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function save() {
    if (!key.trim() || !value) return;
    setSaving(true);
    setStatus(null);
    setError(null);
    try {
      await setEnv(key.trim(), value);
      setStatus(`Saved ${key.trim()}. Restart the server to apply.`);
      setValue("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mx-auto max-w-2xl px-6 py-8">
      <h1 className="mb-1 text-xl font-semibold">Secrets</h1>
      <p className="mb-6 text-sm text-neutral-500">
        Values are write-only — existing secrets are shown masked and can never be read back.
      </p>

      <div className="mb-8 rounded-xl border border-neutral-800 bg-neutral-900/50 p-4">
        <h2 className="mb-3 text-sm font-medium text-neutral-400">Set a secret</h2>
        <div className="flex flex-col gap-3">
          <input
            list="common-keys"
            className="rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm uppercase outline-none focus:border-amber-500"
            placeholder="KEY  (e.g. ANTHROPIC_API_KEY)"
            value={key}
            onChange={(e) => setKey(e.target.value.toUpperCase())}
          />
          <datalist id="common-keys">
            {COMMON_KEYS.map((k) => (
              <option key={k} value={k} />
            ))}
          </datalist>
          <input
            type="password"
            className="rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500"
            placeholder="value"
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
          <div className="flex items-center gap-3">
            <button
              className="w-fit rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
              onClick={save}
              disabled={saving || !key.trim() || !value}
            >
              {saving ? "Saving…" : "Save secret"}
            </button>
            {status && <span className="text-sm text-emerald-400">{status}</span>}
            {error && <span className="text-sm text-red-400">{error}</span>}
          </div>
        </div>
      </div>

      <h2 className="mb-2 text-sm font-medium text-neutral-400">Configured ({entries.length})</h2>
      <div className="divide-y divide-neutral-800 rounded-xl border border-neutral-800">
        {entries.length === 0 && (
          <div className="px-4 py-3 text-sm text-neutral-500">No secrets set.</div>
        )}
        {entries.map((e) => (
          <div key={e.key} className="flex items-center justify-between px-4 py-2.5">
            <span className="font-mono text-sm">{e.key}</span>
            <div className="flex items-center gap-3">
              <span className="font-mono text-sm text-neutral-500">{e.masked}</span>
              <button
                className="rounded-md border border-neutral-700 px-2 py-1 text-xs text-neutral-400 hover:border-red-700 hover:text-red-400"
                title={`Remove ${e.key}`}
                onClick={async () => {
                  if (!confirm(`Remove ${e.key}?`)) return;
                  try {
                    await deleteEnv(e.key);
                    await refresh();
                  } catch (err) {
                    setError(String(err));
                  }
                }}
              >
                ✕
              </button>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
