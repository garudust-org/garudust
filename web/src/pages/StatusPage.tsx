import { useEffect, useState } from "react";
import { getConfig, getHealth, type HealthResponse } from "../lib/api";

export default function StatusPage() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [config, setConfig] = useState<Record<string, unknown> | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setError(null);
    try {
      const [h, c] = await Promise.all([getHealth(), getConfig()]);
      setHealth(h);
      setConfig(c);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    refresh();
    const t = setInterval(refresh, 5000);
    return () => clearInterval(t);
  }, []);

  const ok = health?.status === "ok";

  return (
    <div className="mx-auto max-w-3xl px-6 py-8">
      <h1 className="mb-6 text-xl font-semibold">Status</h1>
      {error && (
        <div className="mb-4 rounded-lg border border-red-800 bg-red-950/50 px-3 py-2 text-sm text-red-300">
          {error}
        </div>
      )}

      <div className="grid grid-cols-2 gap-3">
        <Card label="Gateway">
          <span className={ok ? "text-emerald-400" : "text-amber-400"}>
            {health?.status ?? "…"}
          </span>
        </Card>
        <Card label="Database">{health?.checks.db ?? "…"}</Card>
        <Card label="Model">{String(config?.model ?? "…")}</Card>
        <Card label="Provider">{String(config?.provider ?? "…")}</Card>
      </div>

      {health && Object.keys(health.checks.platforms).length > 0 && (
        <div className="mt-6">
          <h2 className="mb-2 text-sm font-medium text-neutral-400">Platforms</h2>
          <div className="grid grid-cols-2 gap-3">
            {Object.entries(health.checks.platforms).map(([name, st]) => (
              <Card key={name} label={name}>
                <span className={st === "ok" ? "text-emerald-400" : "text-red-400"}>{st}</span>
              </Card>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

function Card({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="rounded-xl border border-neutral-800 bg-neutral-900/50 p-4">
      <div className="text-xs uppercase tracking-wide text-neutral-500">{label}</div>
      <div className="mt-1 text-lg">{children}</div>
    </div>
  );
}
