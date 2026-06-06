use leptos::prelude::*;

use crate::api;
use crate::pages::{chat::ChatPage, config::ConfigPage, env::EnvPage, status::StatusPage};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Chat,
    Status,
    Config,
    Env,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Conn {
    Connecting,
    Online,
    Offline,
}

// Poll /health every `ms` milliseconds using recursive spawn_local.
fn poll_health(conn: RwSignal<Conn>, ms: u32) {
    leptos::task::spawn_local(async move {
        gloo_timers::future::TimeoutFuture::new(ms).await;
        match api::get_health().await {
            Ok(_) => conn.set(Conn::Online),
            Err(_) => conn.update(|c| {
                if *c == Conn::Online {
                    *c = Conn::Offline;
                }
            }),
        }
        poll_health(conn, ms);
    });
}

#[component]
pub fn App() -> impl IntoView {
    let page = RwSignal::new(Page::Chat);
    let conn = RwSignal::new(Conn::Connecting);

    // First ping + start polling.
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(_) = api::get_health().await {
                conn.set(Conn::Online);
            }
        });
        poll_health(conn, 4_000);
    });

    view! {
        // Splash
        <Show when=move || conn.get() == Conn::Connecting>
            <div class="flex h-full flex-col items-center justify-center gap-3 text-neutral-400">
                <div class="text-2xl">"🪶"</div>
                <div class="animate-pulse text-sm">"Connecting to Garudust…"</div>
            </div>
        </Show>

        // App shell
        <Show when=move || conn.get() != Conn::Connecting>
            <div class="flex h-full flex-col">
                <Show when=move || conn.get() == Conn::Offline>
                    <div class="bg-red-900/70 px-4 py-1.5 text-center text-xs text-red-100">
                        "Lost connection to the agent server — retrying…"
                    </div>
                </Show>

                <div class="flex min-h-0 flex-1">
                    <aside class="flex w-52 flex-col border-r border-neutral-800 bg-neutral-950 p-3">
                        <div class="mb-6 px-2 pt-2 text-lg font-semibold tracking-tight">
                            "🪶 Garudust"
                        </div>
                        <nav class="flex flex-col gap-1">
                            {[
                                (Page::Chat,   "Chat"),
                                (Page::Status, "Status"),
                                (Page::Config, "Config"),
                                (Page::Env,    "Secrets"),
                            ]
                            .into_iter()
                            .map(|(p, label)| view! {
                                <button
                                    class=move || format!(
                                        "rounded-lg px-3 py-2 text-left text-sm {}",
                                        if page.get() == p {
                                            "bg-neutral-800 text-neutral-50"
                                        } else {
                                            "text-neutral-300 hover:bg-neutral-900"
                                        }
                                    )
                                    on:click=move |_| page.set(p)
                                >
                                    {label}
                                </button>
                            })
                            .collect_view()}
                        </nav>
                        <div class="mt-auto px-2 text-xs text-neutral-600">"v0.13.6"</div>
                    </aside>

                    <main class="flex-1 overflow-hidden">
                        {move || match page.get() {
                            Page::Chat   => view! { <ChatPage/> }.into_any(),
                            Page::Status => view! { <div class="h-full overflow-y-auto"><StatusPage/></div> }.into_any(),
                            Page::Config => view! { <div class="h-full overflow-y-auto"><ConfigPage/></div> }.into_any(),
                            Page::Env    => view! { <div class="h-full overflow-y-auto"><EnvPage/></div> }.into_any(),
                        }}
                    </main>
                </div>
            </div>
        </Show>
    }
}
