//! Background Tokio runtime that owns the embedded `Agent`. The UI talks to it
//! over channels; on a `Reload` it rebuilds the agent from fresh config (so
//! Config/Secrets edits take effect without restarting the app).

use std::sync::mpsc as std_mpsc;
use std::sync::Arc;

use garudust_agent::{Agent, ConstitutionalApprover};
use garudust_core::config::AgentConfig;
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_tools::{register_standard_tools, ToolRegistry};
use garudust_transport::build_transport;

/// Stable session key for the desktop, so the agent carries context across
/// launches and the chat view can be restored. `New chat` clears it.
const SESSION: &str = "egui-desktop";

pub enum Cmd {
    Chat {
        text: String,
        hint: Option<String>,
    },
    Stop,
    /// Abort any in-flight turn and wipe the persisted conversation.
    NewChat,
    Reload,
}

/// Token usage for one completed turn.
pub struct Usage {
    pub input: u32,
    pub output: u32,
}

pub enum Evt {
    /// Prior (user, assistant) pairs, sent once at startup to restore the view.
    History(Vec<(String, String)>),
    Delta(String),
    /// Name of a tool the agent just dispatched — drives the "working" indicator.
    Tool(String),
    /// Turn finished. Carries usage on a normal completion; `None` when stopped.
    Done(Option<Usage>),
    Error(String),
}

pub struct Backend {
    pub cmd_tx: tokio::sync::mpsc::UnboundedSender<Cmd>,
    pub evt_rx: std_mpsc::Receiver<Evt>,
}

fn build_agent() -> Arc<Agent> {
    let config = Arc::new(AgentConfig::load());
    let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
    let transport = build_transport(&config);
    let db = Arc::new(SessionDb::open(&config.home_dir).expect("open session db"));
    let mut registry = ToolRegistry::new();
    register_standard_tools(&mut registry, Some(db.clone()), None, None);
    Arc::new(Agent::new(transport, Arc::new(registry), memory, config).with_session_db(db))
}

impl Backend {
    pub fn spawn(ctx: eframe::egui::Context) -> Self {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel::<Cmd>();
        let (evt_tx, evt_rx) = std_mpsc::channel::<Evt>();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                let mut agent = build_agent();
                let approver = Arc::new(ConstitutionalApprover);
                // Abort handle for the in-flight chat, so Stop can cancel it.
                let mut current: Option<tokio::task::AbortHandle> = None;

                // Restore the prior conversation into the UI on launch.
                let prior = agent.history_pairs(SESSION);
                if !prior.is_empty() {
                    let _ = evt_tx.send(Evt::History(prior));
                    ctx.request_repaint();
                }

                while let Some(cmd) = cmd_rx.recv().await {
                    match cmd {
                        Cmd::Reload => agent = build_agent(),
                        Cmd::Stop => {
                            if let Some(h) = current.take() {
                                h.abort();
                            }
                            let _ = evt_tx.send(Evt::Done(None));
                            ctx.request_repaint();
                        }
                        Cmd::NewChat => {
                            if let Some(h) = current.take() {
                                h.abort();
                            }
                            agent.clear_session(SESSION);
                        }
                        Cmd::Chat { text, hint } => {
                            // Run in a spawned task so the loop stays free to receive Stop.
                            let a = agent.clone();
                            let ap = approver.clone();
                            let s = SESSION.to_string();
                            let evt = evt_tx.clone();
                            let ctx2 = ctx.clone();
                            let task = tokio::spawn(async move {
                                let (chunk_tx, mut chunk_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<String>();
                                // Forward chunks; self-terminates when chunk_tx drops.
                                let evt_fwd = evt.clone();
                                let ctx_fwd = ctx2.clone();
                                let fwd = tokio::spawn(async move {
                                    while let Some(d) = chunk_rx.recv().await {
                                        let _ = evt_fwd.send(Evt::Delta(d));
                                        ctx_fwd.request_repaint();
                                    }
                                });
                                // Forward tool-dispatch names so the UI can show
                                // what the agent is doing during a tool round.
                                let (tool_tx, mut tool_rx) =
                                    tokio::sync::mpsc::unbounded_channel::<String>();
                                let evt_tool = evt.clone();
                                let ctx_tool = ctx2.clone();
                                let tfwd = tokio::spawn(async move {
                                    while let Some(name) = tool_rx.recv().await {
                                        let _ = evt_tool.send(Evt::Tool(name));
                                        ctx_tool.request_repaint();
                                    }
                                });
                                let res = a
                                    .run_streaming(
                                        &text,
                                        ap,
                                        "egui",
                                        chunk_tx,
                                        Some(tool_tx),
                                        hint.as_deref(),
                                        Some(&s),
                                    )
                                    .await;
                                let _ = fwd.await;
                                let _ = tfwd.await;
                                match res {
                                    Ok(r) => {
                                        let _ = evt.send(Evt::Done(Some(Usage {
                                            input: r.usage.input_tokens,
                                            output: r.usage.output_tokens,
                                        })));
                                    }
                                    Err(e) => {
                                        let _ = evt.send(Evt::Error(e.to_string()));
                                    }
                                }
                                ctx2.request_repaint();
                            });
                            current = Some(task.abort_handle());
                        }
                    }
                }
            });
        });

        Self { cmd_tx, evt_rx }
    }
}
