use leptos::prelude::*;

use crate::api;

const PROVIDERS: &[&str] = &[
    "anthropic", "openai", "gemini", "groq", "mistral", "deepseek", "ollama",
    "openrouter", "vllm", "bedrock", "xai", "together", "fireworks", "cerebras",
    "perplexity", "cohere", "nvidia", "alibaba", "doubao", "zhipu", "moonshot",
    "baidu", "thaillm", "codex",
];
const APPROVAL_MODES: &[&str] = &["auto", "smart", "deny", "interactive"];
const SANDBOX_MODES: &[&str] = &["none", "docker", "ssh"];

// Default model per provider (editable; blank = don't override current model).
fn default_model(provider: &str) -> &'static str {
    match provider {
        "anthropic"  => "claude-sonnet-4-6",
        "openai"     => "gpt-4o",
        "gemini"     => "gemini-2.0-flash",
        "groq"       => "llama-3.3-70b-versatile",
        "mistral"    => "mistral-large-latest",
        "deepseek"   => "deepseek-chat",
        "ollama"     => "llama3.2",
        "openrouter" => "anthropic/claude-sonnet-4-6",
        "xai"        => "grok-2-latest",
        "together"   => "meta-llama/Llama-3.3-70B-Instruct-Turbo",
        "perplexity" => "sonar",
        "cohere"     => "command-r-plus",
        "nvidia"     => "meta/llama-3.3-70b-instruct",
        "cerebras"   => "llama-3.3-70b",
        _            => "",
    }
}
fn provider_key_env(provider: &str) -> &'static str {
    match provider {
        "anthropic"  => "ANTHROPIC_API_KEY",
        "openai"     => "OPENAI_API_KEY",
        "gemini"     => "GEMINI_API_KEY",
        "groq"       => "GROQ_API_KEY",
        "mistral"    => "MISTRAL_API_KEY",
        "deepseek"   => "DEEPSEEK_API_KEY",
        "xai"        => "XAI_API_KEY",
        "together"   => "TOGETHER_API_KEY",
        "fireworks"  => "FIREWORKS_API_KEY",
        "cerebras"   => "CEREBRAS_API_KEY",
        "perplexity" => "PERPLEXITY_API_KEY",
        "cohere"     => "COHERE_API_KEY",
        "nvidia"     => "NVIDIA_API_KEY",
        "alibaba"    => "DASHSCOPE_API_KEY",
        "doubao"     => "ARK_API_KEY",
        "zhipu"      => "ZHIPU_API_KEY",
        "moonshot"   => "MOONSHOT_API_KEY",
        "baidu"      => "QIANFAN_API_KEY",
        "thaillm"    => "THAILLM_API_KEY",
        "vllm"       => "VLLM_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        _            => "",
    }
}

#[derive(Clone)]
struct RoutingRow {
    hint: String,
    target: String,
}

#[component]
pub fn ConfigPage() -> impl IntoView {
    let config = RwSignal::new(Option::<serde_json::Value>::None);
    let env_keys = RwSignal::new(std::collections::HashSet::<String>::new());
    let routing_rows = RwSignal::new(Vec::<RoutingRow>::new());
    let status = RwSignal::new(Option::<String>::None);
    let error = RwSignal::new(Option::<String>::None);
    let saving = RwSignal::new(false);

    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            let (cfg, env) = futures::join!(api::get_config(), api::get_env());
            if let Ok(c) = cfg {
                let rows = c["routing"]
                    .as_object()
                    .map(|m| {
                        m.iter()
                            .map(|(k, v)| RoutingRow {
                                hint: k.clone(),
                                target: v.as_str().unwrap_or("").to_string(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                routing_rows.set(rows);
                config.set(Some(c));
            }
            if let Ok(entries) = env {
                env_keys.set(entries.into_iter().map(|e| e.key).collect());
            }
        });
    });

    let sync_routing = move || {
        config.update(|c| {
            if let Some(c) = c {
                let mut map = serde_json::Map::new();
                for row in routing_rows.get_untracked() {
                    let h = row.hint.trim().to_string();
                    if !h.is_empty() {
                        map.insert(h, serde_json::Value::String(row.target.trim().to_string()));
                    }
                }
                c["routing"] = serde_json::Value::Object(map);
            }
        });
    };

    let save = move |_: web_sys::MouseEvent| {
        let Some(cfg) = config.get_untracked() else { return };
        saving.set(true);
        status.set(None);
        error.set(None);
        leptos::task::spawn_local(async move {
            match api::put_config(&cfg).await {
                Ok(()) => status.set(Some("Saved — agent will hot-reload.".into())),
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let field_val = move |key: &'static str| {
        move || {
            config.get()
                .and_then(|c| c[key].as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        }
    };

    let set_field = move |key: &'static str, val: String| {
        config.update(|c| {
            if let Some(c) = c {
                c[key] = if val.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::String(val)
                };
            }
        });
    };

    let num_field_val = move |key: &'static str| {
        move || config.get().and_then(|c| c[key].as_u64()).unwrap_or(0)
    };
    let set_num_field = move |key: &'static str, val: u64| {
        config.update(|c| {
            if let Some(c) = c { c[key] = serde_json::Value::Number(val.into()); }
        });
    };

    let security_val = move |key: &'static str| {
        move || {
            config.get()
                .and_then(|c| c["security"][key].as_str().map(|s| s.to_string()))
                .unwrap_or_default()
        }
    };
    let set_security = move |key: &'static str, val: String| {
        config.update(|c| {
            if let Some(c) = c {
                if let Some(sec) = c["security"].as_object_mut() {
                    sec.insert(key.to_string(), serde_json::Value::String(val));
                }
            }
        });
    };

    let input_cls = "rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500";

    view! {
        <div class="mx-auto max-w-2xl px-6 py-8">
            <h1 class="mb-6 text-xl font-semibold">"Config"</h1>
            <Show when=move || config.get().is_none()>
                <div class="text-neutral-500">{move || error.get().unwrap_or("Loading…".into())}</div>
            </Show>
            <Show when=move || config.get().is_some()>
                <div class="flex flex-col gap-4">
                    // Provider
                    <Field label="Provider">
                        <select
                            class=format!("w-56 {input_cls}")
                            on:change=move |e| {
                                let p = event_target_value(&e);
                                let def = default_model(&p);
                                if !def.is_empty() {
                                    set_field("model", def.to_string());
                                }
                                set_field("provider", p);
                            }
                        >
                            {move || {
                                let cur = field_val("provider")();
                                if !cur.is_empty() && !PROVIDERS.contains(&cur.as_str()) {
                                    Some(view! {
                                        <option value=cur.clone() selected=true>
                                            {format!("{cur} (custom)")}
                                        </option>
                                    })
                                } else { None }
                            }}
                            {PROVIDERS.iter().map(|&p| {
                                let cur = field_val("provider");
                                view! {
                                    <option value=p prop:selected=move || cur() == p>{p}</option>
                                }
                            }).collect_view()}
                        </select>
                    </Field>

                    // Model (with key hint)
                    <Field label="Model">
                        <input
                            class=format!("w-full {input_cls}")
                            prop:value=field_val("model")
                            on:input=move |e| set_field("model", event_target_value(&e))
                        />
                        // Key hint
                        {move || {
                            let p = field_val("provider")();
                            let key_env = provider_key_env(&p);
                            if key_env.is_empty() {
                                view! { <span class="text-xs text-neutral-500">"Local provider — no API key needed."</span> }.into_any()
                            } else if env_keys.get().contains(key_env) {
                                view! {
                                    <span class="text-xs text-emerald-500">
                                        "✓ " <code>{key_env}</code> " is set"
                                    </span>
                                }.into_any()
                            } else {
                                view! {
                                    <span class="text-xs text-amber-500">
                                        "⚠ needs " <code>{key_env}</code> " — set it on the Secrets page"
                                    </span>
                                }.into_any()
                            }
                        }}
                    </Field>

                    // String fields
                    {["base_url", "reflection_model"].iter().map(|&key| {
                        let label = match key { "base_url" => "Base URL", _ => "Reflection model" };
                        view! {
                            <Field label=label>
                                <input
                                    class=format!("w-full {input_cls}")
                                    prop:value=field_val(key)
                                    on:input=move |e| set_field(key, event_target_value(&e))
                                />
                            </Field>
                        }
                    }).collect_view()}

                    // Approval mode
                    <Field label="Approval mode">
                        <select
                            class=format!("w-56 {input_cls}")
                            on:change=move |e| set_security("approval_mode", event_target_value(&e))
                        >
                            {APPROVAL_MODES.iter().map(|&m| {
                                let cur = security_val("approval_mode");
                                view! { <option value=m prop:selected=move || cur() == m>{m}</option> }
                            }).collect_view()}
                        </select>
                    </Field>

                    // Terminal sandbox
                    <Field label="Terminal sandbox">
                        <select
                            class=format!("w-56 {input_cls}")
                            on:change=move |e| set_security("terminal_sandbox", event_target_value(&e))
                        >
                            {SANDBOX_MODES.iter().map(|&m| {
                                let cur = security_val("terminal_sandbox");
                                view! { <option value=m prop:selected=move || cur() == m>{m}</option> }
                            }).collect_view()}
                        </select>
                    </Field>

                    // Number fields
                    {[
                        ("max_iterations", "Max iterations"),
                        ("nudge_interval", "Memory nudge interval"),
                        ("auto_skill_threshold", "Auto-skill threshold"),
                        ("max_history_pairs", "Max history pairs"),
                    ].iter().map(|&(key, label)| {
                        view! {
                            <Field label=label>
                                <input
                                    type="number"
                                    class=format!("w-40 {input_cls}")
                                    prop:value=num_field_val(key)
                                    on:input=move |e| {
                                        if let Ok(n) = event_target_value(&e).parse::<u64>() {
                                            set_num_field(key, n);
                                        }
                                    }
                                />
                            </Field>
                        }
                    }).collect_view()}

                    // ── Routing hints ───────────────────────────────────────
                    <div class="flex flex-col gap-2">
                        <span class="text-sm text-neutral-400">"Routing (model hints)"</span>
                        <p class="text-xs text-neutral-500">
                            "Each hint maps to " <code>"provider/model"</code>
                            " and appears in the chat Model picker."
                        </p>
                        <For
                            each=move || routing_rows.get().into_iter().enumerate()
                            key=|(i, _)| *i
                            children=move |(i, row)| view! {
                                <div class="flex items-center gap-2">
                                    <input
                                        class=format!("w-32 {input_cls}")
                                        placeholder="fast"
                                        prop:value=row.hint.clone()
                                        on:input=move |e| {
                                            routing_rows.update(|rows| {
                                                if let Some(r) = rows.get_mut(i) {
                                                    r.hint = event_target_value(&e);
                                                }
                                            });
                                            sync_routing();
                                        }
                                    />
                                    <span class="text-neutral-600">"→"</span>
                                    <input
                                        class=format!("flex-1 {input_cls}")
                                        placeholder="groq/llama-3.3-70b-versatile"
                                        prop:value=row.target.clone()
                                        on:input=move |e| {
                                            routing_rows.update(|rows| {
                                                if let Some(r) = rows.get_mut(i) {
                                                    r.target = event_target_value(&e);
                                                }
                                            });
                                            sync_routing();
                                        }
                                    />
                                    <button
                                        class="rounded-lg border border-neutral-700 px-2 py-2 text-xs text-neutral-400 hover:border-red-700 hover:text-red-400"
                                        on:click=move |_| {
                                            routing_rows.update(|rows| { rows.remove(i); });
                                            sync_routing();
                                        }
                                    >
                                        "✕"
                                    </button>
                                </div>
                            }
                        />
                        <button
                            class="w-fit rounded-lg border border-neutral-700 px-3 py-1.5 text-xs text-neutral-300 hover:border-amber-500"
                            on:click=move |_| {
                                routing_rows.update(|rows| rows.push(RoutingRow { hint: String::new(), target: String::new() }));
                            }
                        >
                            "+ Add hint"
                        </button>
                    </div>
                </div>

                <div class="mt-6 flex items-center gap-3">
                    <button
                        class="rounded-lg bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
                        on:click=save
                        prop:disabled=move || saving.get()
                    >
                        {move || if saving.get() { "Saving…" } else { "Save" }}
                    </button>
                    <Show when=move || status.get().is_some()>
                        <span class="text-sm text-emerald-400">{move || status.get().unwrap_or_default()}</span>
                    </Show>
                    <Show when=move || error.get().is_some()>
                        <span class="text-sm text-red-400">{move || error.get().unwrap_or_default()}</span>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

#[component]
fn Field(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1">
            <span class="text-sm text-neutral-400">{label}</span>
            {children()}
        </label>
    }
}
