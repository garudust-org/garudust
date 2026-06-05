import { useEffect, useState } from "react";
import { getConfig, putConfig } from "../lib/api";

// The most frequently edited fields get inputs. The full config object is
// preserved and round-tripped, so fields not surfaced here are never lost on
// save — and saving hot-reloads the running agent (the server watches
// config.yaml).
const STRING_FIELDS: { key: string; label: string }[] = [
  { key: "model", label: "Model" },
  { key: "base_url", label: "Base URL" },
  { key: "reflection_model", label: "Reflection model" },
];
const NUMBER_FIELDS: { key: string; label: string }[] = [
  { key: "max_iterations", label: "Max iterations" },
  { key: "nudge_interval", label: "Memory nudge interval" },
  { key: "auto_skill_threshold", label: "Auto-skill threshold" },
  { key: "max_history_pairs", label: "Max history pairs" },
];

// Known built-in providers (you can still type a custom one). Order: common first.
const PROVIDERS = [
  "anthropic", "openai", "gemini", "groq", "mistral", "deepseek", "ollama",
  "openrouter", "vllm", "bedrock", "xai", "together", "fireworks", "cerebras",
  "perplexity", "cohere", "nvidia", "alibaba", "doubao", "zhipu", "moonshot",
  "baidu", "thaillm", "codex",
];
const APPROVAL_MODES = ["auto", "smart", "deny", "interactive"];
const SANDBOX_MODES = ["none", "docker", "ssh"];

const inputCls =
  "rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500";

export default function ConfigPage() {
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getConfig()
      .then(setConfig)
      .catch((e) => setError(String(e)));
  }, []);

  function set(key: string, value: unknown) {
    setConfig((c) => (c ? { ...c, [key]: value } : c));
  }

  // Merge into the nested `security` object without dropping its other fields.
  function setSecurity(key: string, value: unknown) {
    setConfig((c) => {
      if (!c) return c;
      const sec = (c.security as Record<string, unknown>) ?? {};
      return { ...c, security: { ...sec, [key]: value } };
    });
  }

  async function save() {
    if (!config) return;
    setSaving(true);
    setStatus(null);
    setError(null);
    try {
      await putConfig(config);
      setStatus("Saved — agent will hot-reload.");
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  if (!config) {
    return (
      <div className="mx-auto max-w-2xl px-6 py-8 text-neutral-500">
        {error ?? "Loading config…"}
      </div>
    );
  }

  const security = (config.security as Record<string, unknown>) ?? {};

  return (
    <div className="mx-auto max-w-2xl px-6 py-8">
      <h1 className="mb-6 text-xl font-semibold">Config</h1>

      <div className="flex flex-col gap-4">
        <Field label="Provider">
          <input
            list="known-providers"
            className={`w-full ${inputCls}`}
            value={(config.provider as string) ?? ""}
            onChange={(e) => set("provider", e.target.value)}
          />
          <datalist id="known-providers">
            {PROVIDERS.map((p) => (
              <option key={p} value={p} />
            ))}
          </datalist>
        </Field>

        {STRING_FIELDS.map((f) => (
          <Field key={f.key} label={f.label}>
            <input
              className={`w-full ${inputCls}`}
              value={(config[f.key] as string) ?? ""}
              onChange={(e) => set(f.key, e.target.value || null)}
            />
          </Field>
        ))}

        <Field label="Approval mode">
          <select
            className={`w-56 ${inputCls}`}
            value={(security.approval_mode as string) ?? "smart"}
            onChange={(e) => setSecurity("approval_mode", e.target.value)}
          >
            {APPROVAL_MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Terminal sandbox">
          <select
            className={`w-56 ${inputCls}`}
            value={(security.terminal_sandbox as string) ?? "none"}
            onChange={(e) => setSecurity("terminal_sandbox", e.target.value)}
          >
            {SANDBOX_MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </Field>

        {NUMBER_FIELDS.map((f) => (
          <Field key={f.key} label={f.label}>
            <input
              type="number"
              className={`w-40 ${inputCls}`}
              value={Number(config[f.key] ?? 0)}
              onChange={(e) => set(f.key, Number(e.target.value))}
            />
          </Field>
        ))}
      </div>

      <div className="mt-6 flex items-center gap-3">
        <button
          className="rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
          onClick={save}
          disabled={saving}
        >
          {saving ? "Saving…" : "Save"}
        </button>
        {status && <span className="text-sm text-emerald-400">{status}</span>}
        {error && <span className="text-sm text-red-400">{error}</span>}
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex flex-col gap-1">
      <span className="text-sm text-neutral-400">{label}</span>
      {children}
    </label>
  );
}
