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

pub enum Cmd {
    Chat { text: String, hint: Option<String> },
    Stop,
    Reload,
}

pub enum Evt {
    Delta(String),
    Done,
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
                let session = uuid::Uuid::new_v4().to_string();
                // Abort handle for the in-flight chat, so Stop can cancel it.
                let mut current: Option<tokio::task::AbortHandle> = None;

                while let Some(cmd) = cmd_rx.recv().await {
                    match cmd {
                        Cmd::Reload => agent = build_agent(),
                        Cmd::Stop => {
                            if let Some(h) = current.take() {
                                h.abort();
                            }
                            let _ = evt_tx.send(Evt::Done);
                            ctx.request_repaint();
                        }
                        Cmd::Chat { text, hint } => {
                            // Run in a spawned task so the loop stays free to receive Stop.
                            let a = agent.clone();
                            let ap = approver.clone();
                            let s = session.clone();
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
                                let res = a
                                    .run_streaming(
                                        &text,
                                        ap,
                                        "egui",
                                        chunk_tx,
                                        None,
                                        hint.as_deref(),
                                        Some(&s),
                                    )
                                    .await;
                                let _ = fwd.await;
                                match res {
                                    Ok(_) => {
                                        let _ = evt.send(Evt::Done);
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
