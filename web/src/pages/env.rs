use leptos::prelude::*;

use crate::api::{self, EnvEntry};

const COMMON_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY", "OPENAI_API_KEY", "GROQ_API_KEY", "OPENROUTER_API_KEY",
    "GOOGLE_AI_API_KEY", "GARUDUST_API_KEY", "TELEGRAM_TOKEN", "DISCORD_TOKEN",
];

#[component]
pub fn EnvPage() -> impl IntoView {
    let entries = RwSignal::new(Vec::<EnvEntry>::new());
    let key = RwSignal::new(String::new());
    let value = RwSignal::new(String::new());
    let status = RwSignal::new(Option::<String>::None);
    let error = RwSignal::new(Option::<String>::None);
    let saving = RwSignal::new(false);

    let refresh = move || {
        leptos::task::spawn_local(async move {
            if let Ok(e) = api::get_env().await {
                entries.set(e);
            }
        });
    };
    Effect::new(move |_| refresh());

    let save = move |_: web_sys::MouseEvent| {
        let k = key.get_untracked();
        let v = value.get_untracked();
        if k.trim().is_empty() || v.is_empty() { return; }
        saving.set(true);
        status.set(None);
        error.set(None);
        let k2 = k.clone();
        leptos::task::spawn_local(async move {
            match api::set_env(&k2, &v).await {
                Ok(()) => {
                    status.set(Some(format!("Saved {k2}. Restart the server to apply.")));
                    value.set(String::new());
                    refresh();
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let remove = move |k: String| {
        let confirm = web_sys::window()
            .and_then(|w| w.confirm_with_message(&format!("Remove {k}?")).ok())
            .unwrap_or(false);
        if !confirm { return; }
        leptos::task::spawn_local(async move {
            if let Err(e) = api::delete_env(&k).await {
                error.set(Some(e));
            } else {
                refresh();
            }
        });
    };

    view! {
        <div class="mx-auto max-w-2xl px-6 py-8">
            <h1 class="mb-1 text-xl font-semibold">"Secrets"</h1>
            <p class="mb-6 text-sm text-neutral-500">
                "Values are write-only — existing secrets are shown masked and cannot be read back."
            </p>

            // ── Set a secret ───────────────────────────────────────────────
            <div class="mb-8 rounded-xl border border-neutral-800 bg-neutral-900/50 p-4">
                <h2 class="mb-3 text-sm font-medium text-neutral-400">"Set a secret"</h2>
                <div class="flex flex-col gap-3">
                    <input
                        list="common-keys"
                        class="rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm uppercase outline-none focus:border-amber-500"
                        placeholder="KEY  (e.g. ANTHROPIC_API_KEY)"
                        prop:value=move || key.get()
                        on:input=move |e| key.set(event_target_value(&e).to_uppercase())
                    />
                    <datalist id="common-keys">
                        {COMMON_KEYS.iter().map(|&k| view! { <option value=k /> }).collect_view()}
                    </datalist>
                    <input
                        type="password"
                        class="rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500"
                        placeholder="value"
                        prop:value=move || value.get()
                        on:input=move |e| value.set(event_target_value(&e))
                    />
                    <div class="flex items-center gap-3">
                        <button
                            class="w-fit rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
                            on:click=save
                            prop:disabled=move || saving.get() || key.get().trim().is_empty() || value.get().is_empty()
                        >
                            {move || if saving.get() { "Saving…" } else { "Save secret" }}
                        </button>
                        <Show when=move || status.get().is_some()>
                            <span class="text-sm text-emerald-400">
                                {move || status.get().unwrap_or_default()}
                            </span>
                        </Show>
                        <Show when=move || error.get().is_some()>
                            <span class="text-sm text-red-400">
                                {move || error.get().unwrap_or_default()}
                            </span>
                        </Show>
                    </div>
                </div>
            </div>

            // ── Configured keys ────────────────────────────────────────────
            <h2 class="mb-2 text-sm font-medium text-neutral-400">
                "Configured (" {move || entries.get().len()} ")"
            </h2>
            <div class="divide-y divide-neutral-800 rounded-xl border border-neutral-800">
                <Show when=move || entries.get().is_empty()>
                    <div class="px-4 py-3 text-sm text-neutral-500">"No secrets set."</div>
                </Show>
                <For
                    each=move || entries.get()
                    key=|e| e.key.clone()
                    children=move |e| {
                        let k = e.key.clone();
                        let k2 = k.clone();
                        view! {
                            <div class="flex items-center justify-between px-4 py-2.5">
                                <span class="font-mono text-sm">{k.clone()}</span>
                                <div class="flex items-center gap-3">
                                    <span class="font-mono text-sm text-neutral-500">{e.masked}</span>
                                    <button
                                        class="rounded-md border border-neutral-700 px-2 py-1 text-xs text-neutral-400 hover:border-red-700 hover:text-red-400"
                                        title=format!("Remove {k}")
                                        on:click=move |_| remove(k2.clone())
                                    >
                                        "✕"
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
