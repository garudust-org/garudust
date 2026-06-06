use leptos::prelude::*;
use uuid::Uuid;

use crate::api;

#[derive(Clone, PartialEq)]
struct Msg {
    is_user: bool,
    content: String,
}

fn md_to_html(s: &str) -> String {
    let parser = pulldown_cmark::Parser::new(s);
    let mut out = String::new();
    pulldown_cmark::html::push_html(&mut out, parser);
    out
}

// A single message bubble rendered as a plain function (not a component) to
// avoid the lifetime/Send dance with Leptos For's children constraint.
fn message_view(m: &Msg) -> impl IntoView {
    if m.is_user {
        view! {
            <div class="flex justify-end">
                <div class="max-w-[80%] rounded-2xl bg-amber-500/90 px-4 py-2 text-neutral-950">
                    {m.content.clone()}
                </div>
            </div>
        }
        .into_any()
    } else {
        let html = md_to_html(&m.content);
        let empty = m.content.is_empty();
        view! {
            <div class="flex justify-start">
                <div class="markdown max-w-[80%] rounded-2xl bg-neutral-800/80 px-4 py-2 text-neutral-100">
                    {if empty {
                        view! { <span class="text-neutral-500">"…"</span> }.into_any()
                    } else {
                        view! { <span inner_html=html/> }.into_any()
                    }}
                </div>
            </div>
        }
        .into_any()
    }
}

#[component]
pub fn ChatPage() -> impl IntoView {
    let session_key = RwSignal::new(Uuid::new_v4().to_string());
    let messages = RwSignal::new(Vec::<Msg>::new());
    let input = RwSignal::new(String::new());
    let streaming = RwSignal::new(false);
    let error_msg = RwSignal::new(Option::<String>::None);
    let routing = RwSignal::new(Vec::<(String, String)>::new());
    let default_model = RwSignal::new(String::new());
    let hint = RwSignal::new(String::new());
    let stop_fn: RwSignal<Option<js_sys::Function>> = RwSignal::new(None);

    // Load routing/model.
    Effect::new(move |_| {
        leptos::task::spawn_local(async move {
            if let Ok(cfg) = api::get_config().await {
                if let Some(m) = cfg["model"].as_str() {
                    default_model.set(m.to_string());
                }
                if let Some(r) = cfg["routing"].as_object() {
                    let rows: Vec<(String, String)> = r
                        .iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect();
                    routing.set(rows);
                }
            }
        });
    });

    // Auto-scroll.
    let bottom_ref = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| {
        let _ = messages.get();
        if let Some(el) = bottom_ref.get() {
            el.scroll_into_view_with_bool(false);
        }
    });

    let do_stop = move || {
        if let Some(f) = stop_fn.get_untracked() {
            let _ = f.call0(&wasm_bindgen::JsValue::NULL);
        }
        stop_fn.set(None);
        streaming.set(false);
    };

    let do_send = move || {
        let text = input.get_untracked().trim().to_string();
        if text.is_empty() || streaming.get_untracked() {
            return;
        }
        error_msg.set(None);
        input.set(String::new());
        messages.update(|m| {
            m.push(Msg { is_user: true, content: text.clone() });
            m.push(Msg { is_user: false, content: String::new() });
        });
        streaming.set(true);

        let h = hint.get_untracked();
        let sk = session_key.get_untracked();

        let on_delta = {
            let messages = messages;
            wasm_bindgen::closure::Closure::wrap(Box::new(move |delta: wasm_bindgen::JsValue| {
                if let Some(s) = delta.as_string() {
                    messages.update(|m| {
                        if let Some(last) = m.last_mut() {
                            last.content.push_str(&s);
                        }
                    });
                }
            }) as Box<dyn Fn(wasm_bindgen::JsValue)>)
        };
        let on_done = {
            wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                streaming.set(false);
            }) as Box<dyn Fn()>)
        };
        let on_error = {
            wasm_bindgen::closure::Closure::wrap(Box::new(move |e: wasm_bindgen::JsValue| {
                error_msg.set(Some(e.as_string().unwrap_or("error".to_string())));
                streaming.set(false);
            }) as Box<dyn Fn(wasm_bindgen::JsValue)>)
        };

        use wasm_bindgen::JsCast;
        let f = api::chat_stream_js(
            text, sk,
            if h.is_empty() { None } else { Some(h) },
            on_delta.into_js_value().unchecked_into(),
            on_done.into_js_value().unchecked_into(),
            on_error.into_js_value().unchecked_into(),
        );
        stop_fn.set(Some(f));
    };

    let new_session = move || {
        do_stop();
        messages.set(Vec::new());
        error_msg.set(None);
        session_key.set(Uuid::new_v4().to_string());
    };

    let on_keydown = move |e: web_sys::KeyboardEvent| {
        if e.key() == "Enter" && !e.shift_key() {
            e.prevent_default();
            do_send();
        }
    };

    // Derive view of messages as a non-reactive snapshot for rendering.
    let msgs_view = move || {
        messages
            .get()
            .iter()
            .enumerate()
            .map(|(i, m)| (i, message_view(m)))
            .collect::<Vec<_>>()
    };

    view! {
        <div class="flex h-full flex-col">
            // Messages pane
            <div class="flex-1 overflow-y-auto px-4 py-6">
                <div class="mx-auto flex max-w-3xl flex-col gap-4">
                    {move || {
                        let msgs = msgs_view();
                        if msgs.is_empty() {
                            view! {
                                <div class="mt-20 text-center text-neutral-500">
                                    "Ask Garudust anything."
                                </div>
                            }.into_any()
                        } else {
                            msgs.into_iter()
                                .map(|(_, v)| v)
                                .collect::<Vec<_>>()
                                .into_any()
                        }
                    }}
                    {move || error_msg.get().map(|e| view! {
                        <div class="rounded-lg border border-red-800 bg-red-950/50 px-3 py-2 text-sm text-red-300">
                            {e}
                        </div>
                    })}
                    <div node_ref=bottom_ref/>
                </div>
            </div>

            // Input bar
            <div class="border-t border-neutral-800 bg-neutral-950/80 px-4 py-3">
                // Model picker + New chat
                <div class="mx-auto mb-2 flex max-w-3xl items-center gap-2">
                    <span class="text-xs text-neutral-500">"Model"</span>
                    {move || {
                        let routes = routing.get();
                        let dm = default_model.get();
                        let default_label = if dm.is_empty() {
                            "Default".to_string()
                        } else {
                            format!("Default · {dm}")
                        };
                        view! {
                            <select
                                class="rounded-lg border border-neutral-700 bg-neutral-900 px-2 py-1 text-xs outline-none focus:border-amber-500"
                                on:change=move |e| hint.set(event_target_value(&e))
                            >
                                <option value="">{default_label}</option>
                                {routes.into_iter().map(|(name, target)| {
                                    let label = format!("{name} · {target}");
                                    view! { <option value=name>{label}</option> }
                                }).collect::<Vec<_>>()}
                            </select>
                        }
                    }}
                    {move || routing.get().is_empty().then(|| view! {
                        <span class="text-xs text-neutral-600">"(add routing hints in Config)"</span>
                    })}
                    <button
                        class="ml-auto rounded-lg border border-neutral-700 px-3 py-1 text-xs text-neutral-300 hover:border-amber-500"
                        on:click=move |_| new_session()
                    >
                        "New chat"
                    </button>
                </div>

                // Text input + send/stop
                <div class="mx-auto flex max-w-3xl items-end gap-2">
                    <textarea
                        class="flex-1 resize-none rounded-xl border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none focus:border-amber-500"
                        rows=1
                        placeholder="Message Garudust…  (Enter to send, Shift+Enter for newline)"
                        prop:value=move || input.get()
                        prop:disabled=move || streaming.get()
                        on:input=move |e| input.set(event_target_value(&e))
                        on:keydown=on_keydown
                    />
                    {move || {
                        if streaming.get() {
                            view! {
                                <button
                                    class="rounded-xl border border-neutral-600 px-4 py-2 text-sm font-medium text-neutral-200 hover:border-red-600 hover:text-red-400"
                                    on:click=move |_| do_stop()
                                >
                                    "Stop"
                                </button>
                            }.into_any()
                        } else {
                            let disabled = move || input.get().trim().is_empty();
                            view! {
                                <button
                                    class="rounded-xl bg-amber-500 px-4 py-2 text-sm font-medium text-neutral-950 disabled:opacity-40"
                                    prop:disabled=disabled
                                    on:click=move |_| do_send()
                                >
                                    "Send"
                                </button>
                            }.into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
