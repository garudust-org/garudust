use leptos::prelude::*;

use crate::api::{self, HealthResponse};

#[component]
pub fn StatusPage() -> impl IntoView {
    let health = RwSignal::new(Option::<HealthResponse>::None);
    let config = RwSignal::new(Option::<serde_json::Value>::None);
    let error = RwSignal::new(Option::<String>::None);

    let refresh = move || {
        leptos::task::spawn_local(async move {
            error.set(None);
            let (h, c) = futures::join!(api::get_health(), api::get_config());
            match h {
                Ok(v) => health.set(Some(v)),
                Err(e) => error.set(Some(e)),
            }
            if let Ok(v) = c {
                config.set(Some(v));
            }
        });
    };

    // Initial load + recursive poll every 5 s (avoids Send constraint on Interval).
    fn poll(refresh: impl Fn() + Clone + 'static) {
        leptos::task::spawn_local({
            let refresh = refresh.clone();
            async move {
                gloo_timers::future::TimeoutFuture::new(5_000).await;
                refresh();
                poll(refresh);
            }
        });
    }
    Effect::new(move |_| {
        refresh();
        poll(move || refresh());
    });

    view! {
        <div class="mx-auto max-w-3xl px-6 py-8">
            <h1 class="mb-6 text-xl font-semibold">"Status"</h1>
            <Show when=move || error.get().is_some()>
                <div class="mb-4 rounded-lg border border-red-800 bg-red-950/50 px-3 py-2 text-sm text-red-300">
                    {move || error.get().unwrap_or_default()}
                </div>
            </Show>
            <div class="grid grid-cols-2 gap-3">
                <Card label="Gateway".to_string()>
                    {move || health.get().map(|h| {
                        let ok = h.status == "ok";
                        view! {
                            <span class=if ok { "text-emerald-400" } else { "text-amber-400" }>
                                {h.status}
                            </span>
                        }.into_any()
                    }).unwrap_or_else(|| view! { <span class="text-neutral-500">"…"</span> }.into_any())}
                </Card>
                <Card label="Database".to_string()>
                    {move || health.get().map(|h| h.checks.db).unwrap_or_else(|| "…".to_string())}
                </Card>
                <Card label="Model".to_string()>
                    {move || config.get()
                        .and_then(|c| c["model"].as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "…".to_string())}
                </Card>
                <Card label="Provider".to_string()>
                    {move || config.get()
                        .and_then(|c| c["provider"].as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "…".to_string())}
                </Card>
            </div>
            // Platform adapters (only when any are running)
            {move || {
                let platforms: Vec<(String, String)> = health.get()
                    .map(|h| h.checks.platforms.into_iter().collect())
                    .unwrap_or_default();
                if platforms.is_empty() {
                    view! { <span/> }.into_any()
                } else {
                    view! {
                        <div class="mt-6">
                            <h2 class="mb-2 text-sm font-medium text-neutral-400">"Platforms"</h2>
                            <div class="grid grid-cols-2 gap-3">
                                <For
                                    each=move || platforms.clone()
                                    key=|(k, _)| k.clone()
                                    children=|(name, st)| {
                                        let ok = st == "ok";
                                        view! {
                                            <Card label=name.clone()>
                                                <span class=if ok { "text-emerald-400" } else { "text-red-400" }>
                                                    {st}
                                                </span>
                                            </Card>
                                        }
                                    }
                                />
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn Card(label: String, children: Children) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-neutral-800 bg-neutral-900/50 p-4">
            <div class="text-xs uppercase tracking-wide text-neutral-500">{label}</div>
            <div class="mt-1 text-lg">{children()}</div>
        </div>
    }
}
