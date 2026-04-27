mod config_cmd;
mod doctor;
mod setup;
mod tui;

use std::sync::Arc;

use anyhow::Result;
use clap::{Parser, Subcommand};
use garudust_agent::{Agent, AutoApprover};
use garudust_core::config::AgentConfig;
use garudust_core::config::McpServerConfig;
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_tools::{
    toolsets::{
        browser::BrowserTool,
        delegate::DelegateTask,
        files::{ReadFile, WriteFile},
        mcp::connect_mcp_server,
        memory::MemoryTool,
        search::SessionSearch,
        skills::{SkillView, SkillsList},
        terminal::Terminal,
        web::{WebFetch, WebSearch},
    },
    ToolRegistry,
};
use garudust_transport::build_transport;
use tokio::sync::mpsc;

use tokio::sync::RwLock;
use tui::{AgentEvent, TuiEvent};

#[derive(Subcommand)]
enum ConfigCmd {
    /// Show current configuration
    Show,
    /// Set a configuration value
    ///
    /// Secret keys (OPENROUTER_API_KEY, ANTHROPIC_API_KEY, …) are saved to ~/.garudust/.env.
    /// Other keys (model, provider, base_url, max_iterations, tool_delay_ms) go to config.yaml.
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum Cmd {
    /// Interactive first-time setup wizard
    Setup,

    /// Check environment and configuration
    Doctor,

    /// View or update configuration
    Config {
        #[command(subcommand)]
        sub: ConfigCmd,
    },
}

#[derive(Parser)]
#[command(name = "garudust", about = "Garudust AI Agent", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// One-shot task (omit to start interactive TUI)
    task: Option<String>,

    /// Override model (env: GARUDUST_MODEL)
    #[arg(long, env = "GARUDUST_MODEL")]
    model: Option<String>,

    /// Override OpenRouter API key (env: OPENROUTER_API_KEY)
    #[arg(long, env = "OPENROUTER_API_KEY")]
    api_key: Option<String>,

    /// Override Anthropic API key — sets provider=anthropic (env: ANTHROPIC_API_KEY)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    anthropic_key: Option<String>,

    /// Override base URL (env: GARUDUST_BASE_URL)
    #[arg(long, env = "GARUDUST_BASE_URL")]
    base_url: Option<String>,
}

fn build_config(cli: &Cli) -> Arc<AgentConfig> {
    let mut config = AgentConfig::load();

    // CLI flags override whatever was loaded from config files / env
    if let Some(m) = &cli.model {
        config.model.clone_from(m);
    }
    if let Some(u) = &cli.base_url {
        config.base_url = Some(u.clone());
    }
    if let Some(k) = &cli.anthropic_key {
        config.api_key = Some(k.clone());
        config.provider = "anthropic".into();
    } else if let Some(k) = &cli.api_key {
        config.api_key = Some(k.clone());
    }

    Arc::new(config)
}

fn build_agent(config: Arc<AgentConfig>) -> Arc<Agent> {
    let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
    let transport = build_transport(&config);

    let mut registry = ToolRegistry::new();
    registry.register(WebFetch);
    registry.register(WebSearch);
    registry.register(ReadFile);
    registry.register(WriteFile);
    registry.register(Terminal);
    registry.register(MemoryTool);
    registry.register(SessionSearch);
    registry.register(SkillsList);
    registry.register(SkillView);
    registry.register(DelegateTask);
    registry.register(BrowserTool::new());

    let db = SessionDb::open(&config.home_dir).ok().map(Arc::new);
    let agent = Agent::new(transport, Arc::new(registry), memory, config);
    let agent = match db {
        Some(db) => agent.with_session_db(db),
        None => agent,
    };
    Arc::new(agent)
}

async fn attach_mcp_servers(
    registry: &mut ToolRegistry,
    servers: &[McpServerConfig],
) -> Vec<Box<dyn std::any::Any + Send>> {
    let mut handles: Vec<Box<dyn std::any::Any + Send>> = Vec::new();
    for srv in servers {
        match connect_mcp_server(&srv.command, &srv.args).await {
            Ok((tools, handle)) => {
                tracing::info!(server = %srv.name, tools = tools.len(), "MCP server connected");
                for t in tools {
                    registry.register_arc(t);
                }
                handles.push(handle);
            }
            Err(e) => {
                tracing::warn!(server = %srv.name, "failed to connect MCP server: {e}");
            }
        }
    }
    handles
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "warn".into()))
        .init();
    dotenvy::dotenv().ok(); // load .env from current dir (development override)

    let cli = Cli::parse();

    // ── Subcommands that don't need a running agent ───────────────────────────
    match &cli.cmd {
        Some(Cmd::Setup) => {
            return setup::run().await;
        }

        Some(Cmd::Doctor) => {
            let config = build_config(&cli);
            doctor::run(&config).await;
            return Ok(());
        }

        Some(Cmd::Config {
            sub: ConfigCmd::Show,
        }) => {
            let config = build_config(&cli);
            config_cmd::show(&config);
            return Ok(());
        }

        Some(Cmd::Config {
            sub: ConfigCmd::Set { key, value },
        }) => {
            let config = build_config(&cli);
            config_cmd::set(key, value, &config.home_dir)?;
            return Ok(());
        }

        None => {}
    }

    // ── Agent modes ───────────────────────────────────────────────────────────
    let config = build_config(&cli);
    let agent = {
        let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
        let transport = build_transport(&config);
        let mut registry = ToolRegistry::new();
        registry.register(WebFetch);
        registry.register(WebSearch);
        registry.register(ReadFile);
        registry.register(WriteFile);
        registry.register(Terminal);
        registry.register(MemoryTool);
        registry.register(SessionSearch);
        registry.register(SkillsList);
        registry.register(SkillView);
        let mcp_handles = attach_mcp_servers(&mut registry, &config.mcp_servers).await;
        let db = SessionDb::open(&config.home_dir).ok().map(Arc::new);
        let a = Agent::new(transport, Arc::new(registry), memory, config.clone());
        let a = match db {
            Some(db) => a.with_session_db(db),
            None => a,
        };
        // leak mcp_handles so the MCP processes stay alive for the entire run
        std::mem::forget(mcp_handles);
        Arc::new(a)
    };

    if let Some(task) = &cli.task {
        // ── One-shot mode ─────────────────────────────────────────────────────
        let approver = Arc::new(AutoApprover);
        let result = agent.run(task, approver, "cli").await?;
        println!("{}", result.output);
        eprintln!(
            "[{} iter | {}in {}out tokens]",
            result.iterations, result.usage.input_tokens, result.usage.output_tokens
        );
    } else {
        // Interactive TUI mode
        let approver = Arc::new(AutoApprover);

        let (tx_event, mut rx_event) = mpsc::channel::<TuiEvent>(32);
        let (tx_agent, rx_agent) = mpsc::channel::<AgentEvent>(64);

        let shared_agent = Arc::new(RwLock::new(agent.clone()));
        let shared_config = config.clone();
        let approver2 = approver.clone();
        let tx_agent2 = tx_agent.clone();

        tokio::spawn(async move {
            while let Some(ev) = rx_event.recv().await {
                match ev {
                    TuiEvent::Quit => break,
                    TuiEvent::NewSession => {} // agent is stateless per-call; UI already cleared
                    TuiEvent::ChangeModel(model) => {
                        let mut new_cfg = (*shared_config).clone();
                        new_cfg.model = model;
                        let new_agent = build_agent(Arc::new(new_cfg));
                        *shared_agent.write().await = new_agent;
                    }
                    TuiEvent::Submit(task) => {
                        let _ = tx_agent2.send(AgentEvent::Thinking).await;
                        let current_agent = shared_agent.read().await.clone();

                        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<String>();
                        let tx_agent3 = tx_agent2.clone();
                        tokio::spawn(async move {
                            while let Some(delta) = chunk_rx.recv().await {
                                let _ = tx_agent3.send(AgentEvent::OutputChunk(delta)).await;
                            }
                        });

                        match current_agent
                            .run_streaming(&task, approver2.clone(), "cli", chunk_tx)
                            .await
                        {
                            Ok(r) => {
                                let _ = tx_agent2
                                    .send(AgentEvent::Done {
                                        iterations: r.iterations,
                                        input_tokens: r.usage.input_tokens,
                                        output_tokens: r.usage.output_tokens,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx_agent2.send(AgentEvent::Error(e.to_string())).await;
                            }
                        }
                    }
                }
            }
        });

        tui::Tui::run(tx_event, rx_agent).await?;
    }

    Ok(())
}
