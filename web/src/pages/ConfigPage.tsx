import { useEffect, useState } from "react";
import { getConfig, getEnv, putConfig } from "../lib/api";

// The most frequently edited fields get inputs. The full config object is
// preserved and round-tripped, so fields not surfaced here are never lost on
// save — and saving hot-reloads the running agent (the server watches
// config.yaml). `model` and `provider` are rendered separately (linked).
const STRING_FIELDS: { key: string; label: string }[] = [
  { key: "base_url", label: "Base URL" },
  { key: "reflection_model", label: "Reflection model" },
];
const NUMBER_FIELDS: { key: string; label: string }[] = [
  { key: "max_iterations", label: "Max iterations" },
  { key: "nudge_interval", label: "Memory nudge interval" },
  { key: "auto_skill_threshold", label: "Auto-skill threshold" },
  { key: "max_history_pairs", label: "Max history pairs" },
];

// Per-provider metadata. `keyEnv` mirrors the backend (BUILTIN_PROVIDERS +
// special transports) and drives the "which secret to set" hint. `model` is a
// sensible editable default applied when you switch providers — empty means
// "don't touch the current model" (e.g. self-hosted vllm/ollama). Models drift,
// so these are starting points, not validated lists.
const PROVIDER_META: Record<string, { keyEnv: string; model: string }> = {
  anthropic: { keyEnv: "ANTHROPIC_API_KEY", model: "claude-sonnet-4-6" },
  openai: { keyEnv: "OPENAI_API_KEY", model: "gpt-4o" },
  gemini: { keyEnv: "GEMINI_API_KEY", model: "gemini-2.0-flash" },
  groq: { keyEnv: "GROQ_API_KEY", model: "llama-3.3-70b-versatile" },
  mistral: { keyEnv: "MISTRAL_API_KEY", model: "mistral-large-latest" },
  deepseek: { keyEnv: "DEEPSEEK_API_KEY", model: "deepseek-chat" },
  ollama: { keyEnv: "", model: "llama3.2" },
  openrouter: { keyEnv: "OPENROUTER_API_KEY", model: "anthropic/claude-sonnet-4-6" },
  vllm: { keyEnv: "VLLM_API_KEY", model: "" },
  bedrock: { keyEnv: "AWS_ACCESS_KEY_ID", model: "anthropic.claude-3-5-sonnet-20241022-v2:0" },
  xai: { keyEnv: "XAI_API_KEY", model: "grok-2-latest" },
  together: { keyEnv: "TOGETHER_API_KEY", model: "meta-llama/Llama-3.3-70B-Instruct-Turbo" },
  fireworks: { keyEnv: "FIREWORKS_API_KEY", model: "accounts/fireworks/models/llama-v3p3-70b-instruct" },
  cerebras: { keyEnv: "CEREBRAS_API_KEY", model: "llama-3.3-70b" },
  perplexity: { keyEnv: "PERPLEXITY_API_KEY", model: "sonar" },
  cohere: { keyEnv: "COHERE_API_KEY", model: "command-r-plus" },
  nvidia: { keyEnv: "NVIDIA_API_KEY", model: "meta/llama-3.3-70b-instruct" },
  alibaba: { keyEnv: "DASHSCOPE_API_KEY", model: "qwen-max" },
  doubao: { keyEnv: "ARK_API_KEY", model: "" },
  zhipu: { keyEnv: "ZHIPU_API_KEY", model: "glm-4-plus" },
  moonshot: { keyEnv: "MOONSHOT_API_KEY", model: "moonshot-v1-8k" },
  baidu: { keyEnv: "QIANFAN_API_KEY", model: "" },
  thaillm: { keyEnv: "THAILLM_API_KEY", model: "" },
  codex: { keyEnv: "", model: "" },
};
const PROVIDERS = Object.keys(PROVIDER_META);
const APPROVAL_MODES = ["auto", "smart", "deny", "interactive"];
const SANDBOX_MODES = ["none", "docker", "ssh"];

const inputCls =
  "rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500";

export default function ConfigPage() {
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [envKeys, setEnvKeys] = useState<Set<string>>(new Set());
  // Routing edited as ordered rows (object keys are awkward to rename live);
  // synced back into config.routing on every change.
  const [routingRows, setRoutingRows] = useState<{ hint: string; target: string }[]>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getConfig()
      .then((c) => {
        setConfig(c);
        const r = (c.routing as Record<string, string>) ?? {};
        setRoutingRows(Object.entries(r).map(([hint, target]) => ({ hint, target })));
      })
      .catch((e) => setError(String(e)));
    // Used to tell the user whether the selected provider's key is set.
    getEnv()
      .then((entries) => setEnvKeys(new Set(entries.map((e) => e.key))))
      .catch(() => {});
  }, []);

  function set(key: string, value: unknown) {
    setConfig((c) => (c ? { ...c, [key]: value } : c));
  }

  // Switching provider applies that provider's default model (when known), so
  // you don't end up with a model string that only worked for the old provider.
  function changeProvider(p: string) {
    setConfig((c) => {
      if (!c) return c;
      const next: Record<string, unknown> = { ...c, provider: p };
      const def = PROVIDER_META[p]?.model;
      if (def) next.model = def;
      return next;
    });
  }

  // Rebuild config.routing from the edited rows (drop blank hints; last wins).
  function commitRoutingRows(rows: { hint: string; target: string }[]) {
    setRoutingRows(rows);
    const obj: Record<string, string> = {};
    for (const { hint, target } of rows) {
      const h = hint.trim();
      if (h) obj[h] = target.trim();
    }
    set("routing", obj);
  }

  function updateRoutingRow(i: number, field: "hint" | "target", value: string) {
    const rows = routingRows.map((r, j) => (j === i ? { ...r, [field]: value } : r));
    commitRoutingRows(rows);
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
  const provider = (config.provider as string) ?? "";

  return (
    <div className="mx-auto max-w-2xl px-6 py-8">
      <h1 className="mb-6 text-xl font-semibold">Config</h1>

      <div className="flex flex-col gap-4">
        <Field label="Provider">
          <select
            className={`w-56 ${inputCls}`}
            value={provider}
            onChange={(e) => changeProvider(e.target.value)}
          >
            {/* keep a custom provider (e.g. a named profile) selectable */}
            {provider && !PROVIDERS.includes(provider) && (
              <option value={provider}>{provider} (custom)</option>
            )}
            {PROVIDERS.map((p) => (
              <option key={p} value={p}>
                {p}
              </option>
            ))}
          </select>
        </Field>

        <Field label="Model">
          <input
            className={`w-full ${inputCls}`}
            value={(config.model as string) ?? ""}
            onChange={(e) => set("model", e.target.value)}
          />
          <KeyHint provider={provider} envKeys={envKeys} />
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

        <div className="flex flex-col gap-2">
          <span className="text-sm text-neutral-400">Routing (model hints)</span>
          <p className="text-xs text-neutral-500">
            Each hint maps to <code>provider/model</code>. They appear in the chat Model picker
            so you can switch models per message.
          </p>
          {routingRows.map((row, i) => (
            <div key={i} className="flex items-center gap-2">
              <input
                className={`w-32 ${inputCls}`}
                placeholder="fast"
                value={row.hint}
                onChange={(e) => updateRoutingRow(i, "hint", e.target.value)}
              />
              <span className="text-neutral-600">→</span>
              <input
                className={`flex-1 ${inputCls}`}
                placeholder="groq/llama-3.3-70b-versatile"
                value={row.target}
                onChange={(e) => updateRoutingRow(i, "target", e.target.value)}
              />
              <button
                className="rounded-lg border border-neutral-700 px-2 py-2 text-xs text-neutral-400 hover:border-red-700 hover:text-red-400"
                onClick={() => commitRoutingRows(routingRows.filter((_, j) => j !== i))}
                title="Remove"
              >
                ✕
              </button>
            </div>
          ))}
          <button
            className="w-fit rounded-lg border border-neutral-700 px-3 py-1.5 text-xs text-neutral-300 hover:border-amber-500"
            onClick={() => commitRoutingRows([...routingRows, { hint: "", target: "" }])}
          >
            + Add hint
          </button>
        </div>
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

// Tells the user which secret the chosen provider needs and whether it's set.
function KeyHint({ provider, envKeys }: { provider: string; envKeys: Set<string> }) {
  const meta = PROVIDER_META[provider];
  if (!meta) return null; // custom provider — unknown requirements
  if (!meta.keyEnv) {
    return <span className="text-xs text-neutral-500">Local provider — no API key needed.</span>;
  }
  const isSet = envKeys.has(meta.keyEnv);
  return isSet ? (
    <span className="text-xs text-emerald-500">
      ✓ <code>{meta.keyEnv}</code> is set
    </span>
  ) : (
    <span className="text-xs text-amber-500">
      ⚠ needs <code>{meta.keyEnv}</code> — set it on the Secrets page
    </span>
  );
}
