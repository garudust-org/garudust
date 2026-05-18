mod config_cmd;
mod doctor;
mod setup;
mod skill_cmd;
mod tool_cmd;
mod tui;

use std::io::Write as _;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use clap::{Parser, Subcommand};
use garudust_agent::{Agent, AutoApprover};
use garudust_core::config::AgentConfig;
use garudust_core::config::McpServerConfig;
use garudust_core::pricing::estimate_cost_usd;
use garudust_memory::{DocStore, FileMemoryStore, SessionDb};
use garudust_tools::{
    load_script_tools, register_standard_tools, security::docker_available,
    toolsets::mcp::connect_mcp_server, ToolRegistry,
};
use garudust_transport::build_transport;
use tokio::sync::mpsc;

use tokio::sync::RwLock;
use tui::{AgentEvent, TuiEvent};

type McpHandles = Vec<Box<dyn std::any::Any + Send>>;

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
enum ToolCmd {
    /// List installed tools (and available hub tools)
    List {
        /// Skip fetching the hub — show only locally installed tools
        #[arg(long)]
        offline: bool,
    },
    /// Install a tool from the hub
    Install {
        /// Tool name as listed in the hub index
        name: String,
        /// Hub repository (default: garudust-org/garudust-hub)
        #[arg(long, default_value = garudust_tools::hub::DEFAULT_HUB)]
        hub: String,
    },
    /// Remove an installed tool
    Uninstall {
        /// Tool name to remove
        name: String,
    },
    /// Update installed hub tools to the latest version
    Update {
        /// Specific tool to update (omit to update all)
        name: Option<String>,
    },
}

#[derive(Subcommand)]
enum SkillCmd {
    /// List installed skills (and available hub skills)
    List {
        /// Skip fetching the hub — show only locally installed skills
        #[arg(long)]
        offline: bool,
    },
    /// Install a skill from the hub, GitHub, a direct URL, or a well-known endpoint
    ///
    /// Sources:
    ///   git-workflow         — short name (resolved via hub index)
    ///   owner/repo/path      — GitHub (raw.githubusercontent.com)
    ///   https://…/SKILL.md   — direct URL
    ///   well-known:https://… — /.well-known/skills/<name>/SKILL.md
    Install {
        /// Skill name (short) or full source path / URL
        source: String,
        /// Skill name to use when saving (inferred from source if omitted)
        #[arg(long, default_value = "")]
        name: String,
        /// Hub repository (default: garudust-org/garudust-hub)
        #[arg(long, default_value = garudust_tools::hub::DEFAULT_HUB)]
        hub: String,
    },
    /// Remove an installed skill
    Uninstall {
        /// Skill name as shown in `garudust skill list`
        name: String,
    },
    /// Update installed hub skills to the latest version
    Update {
        /// Specific skill to update (omit to update all)
        name: Option<String>,
    },
    /// Validate SKILL.md files — report malformed frontmatter or missing required fields
    Validate {
        /// Path to a specific SKILL.md file or directory to scan (default: ~/.garudust/skills/)
        path: Option<std::path::PathBuf>,
    },
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

    /// Get or set the active model
    Model {
        /// Model name to switch to (omit for interactive prompt)
        name: Option<String>,
    },

    /// Manage script tools (install, uninstall, update, list)
    Tool {
        #[command(subcommand)]
        sub: ToolCmd,
    },

    /// Manage skills (install, uninstall, update, list)
    Skill {
        #[command(subcommand)]
        sub: SkillCmd,
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

    /// Routing hint — selects an alternative provider/model from the `routing`
    /// table in config.yaml (e.g. `--hint cheap` or `--hint reason`)
    #[arg(long)]
    hint: Option<String>,
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

/// Single source of truth for the CLI agent — registers all tools and MCP servers.
/// Returns the agent and MCP process handles; caller must keep handles alive for
/// as long as the agent is in use (dropping them terminates the MCP processes).
async fn build_agent(config: Arc<AgentConfig>) -> (Arc<Agent>, McpHandles) {
    let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
    let transport = build_transport(&config);

    if config.security.terminal_sandbox == garudust_core::config::TerminalSandbox::Docker
        && !docker_available()
    {
        tracing::warn!(
            "terminal_sandbox is set to 'docker' but Docker is not installed or not in PATH. \
             Terminal commands will fail. Set `terminal_sandbox: none` or install Docker."
        );
    }

    let db = SessionDb::open(&config.home_dir).ok().map(Arc::new);
    let doc_store = DocStore::open(&config.home_dir).ok().map(Arc::new);

    let mut registry = ToolRegistry::new();
    register_standard_tools(&mut registry, db.clone(), doc_store, None);

    let mcp_handles = attach_mcp_servers(&mut registry, &config.mcp_servers).await;

    for tool in load_script_tools(&config.home_dir).await {
        registry.register(tool);
    }

    if !config.disabled_toolsets.is_empty() {
        registry.remove_toolsets(&config.disabled_toolsets);
        tracing::info!(disabled = ?config.disabled_toolsets, "toolsets disabled via config");
    }
    if !config.disabled_tools.is_empty() {
        registry.remove_tools(&config.disabled_tools);
        tracing::info!(disabled = ?config.disabled_tools, "tools disabled via config");
    }

    let agent = Agent::new(transport, Arc::new(registry), memory, config);
    let agent = match db {
        Some(db) => agent.with_session_db(db),
        None => agent,
    };
    (Arc::new(agent), mcp_handles)
}

async fn attach_mcp_servers(
    registry: &mut ToolRegistry,
    servers: &[McpServerConfig],
) -> McpHandles {
    let mut handles: McpHandles = Vec::new();
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
        .with_writer(std::io::stderr)
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

        Some(Cmd::Model { name }) => {
            let config = build_config(&cli);
            config_cmd::set_model(name.as_deref(), &config)?;
            return Ok(());
        }

        Some(Cmd::Tool { sub }) => {
            let config = build_config(&cli);
            let tools_dir = config.home_dir.join("tools");
            tokio::fs::create_dir_all(&tools_dir).await?;
            match sub {
                ToolCmd::List { offline } => {
                    tool_cmd::list(&tools_dir, *offline).await?;
                }
                ToolCmd::Install { name, hub } => {
                    tool_cmd::install(name, &tools_dir, hub).await?;
                }
                ToolCmd::Uninstall { name } => {
                    tool_cmd::uninstall(name, &tools_dir).await?;
                }
                ToolCmd::Update { name } => {
                    tool_cmd::update(name.as_deref(), &tools_dir).await?;
                }
            }
            return Ok(());
        }

        Some(Cmd::Skill { sub }) => {
            let config = build_config(&cli);
            let skills_dir = config.home_dir.join("skills");
            tokio::fs::create_dir_all(&skills_dir).await?;
            match sub {
                SkillCmd::List { offline } => {
                    skill_cmd::list(&skills_dir, *offline).await?;
                }
                SkillCmd::Install { source, name, hub } => {
                    skill_cmd::install(source, name, hub, &skills_dir).await?;
                }
                SkillCmd::Uninstall { name } => {
                    skill_cmd::uninstall(name, &skills_dir).await?;
                }
                SkillCmd::Update { name } => {
                    skill_cmd::update(name.as_deref(), &skills_dir).await?;
                }
                SkillCmd::Validate { path } => {
                    skill_cmd::validate(path.as_ref(), &skills_dir).await?;
                }
            }
            return Ok(());
        }

        None => {}
    }

    // ── Agent modes ───────────────────────────────────────────────────────────
    let config = build_config(&cli);
    let (agent, mcp_handles) = build_agent(config.clone()).await;

    if let Some(task) = &cli.task {
        // ── One-shot mode (streaming) ─────────────────────────────────────────
        let _handles = mcp_handles;
        let approver = Arc::new(AutoApprover);

        // Print routing hint before running.
        if let Some(hint) = &cli.hint {
            let target = config.routing.get(hint.as_str()).map(String::as_str).unwrap_or("—");
            eprintln!("\x1b[2m  ▸ routing: {hint} → {target}\x1b[0m");
        }
        eprint!("\x1b[2mthinking...\x1b[0m");
        std::io::stderr().flush().ok();

        let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let (tool_tx, mut tool_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

        // Forward tool names to stderr as "  ▸ name".
        tokio::spawn(async move {
            while let Some(name) = tool_rx.recv().await {
                eprint!("\r\x1b[K\x1b[2m  ▸ {name}\x1b[0m\n");
                std::io::stderr().flush().ok();
            }
        });

        // Forward text chunks to stdout.
        let print_task = tokio::spawn(async move {
            let mut started = false;
            while let Some(chunk) = chunk_rx.recv().await {
                if !started {
                    // Clear the "thinking..." line before first text output.
                    eprint!("\r\x1b[K");
                    std::io::stderr().flush().ok();
                    started = true;
                }
                print!("{chunk}");
                std::io::stdout().flush().ok();
            }
        });

        let started_at = Instant::now();
        let result = agent
            .run_streaming(task, approver, "cli", chunk_tx, Some(tool_tx), cli.hint.as_deref(), None)
            .await?;
        print_task.await.ok();

        // Newline after streamed output.
        println!();

        if !config.show_usage_footer {
            let elapsed = started_at.elapsed().as_secs_f32();
            let input_tokens = result.usage.input_tokens;
            let output_tokens = result.usage.output_tokens;
            let cost_part = estimate_cost_usd(&config.model, input_tokens, output_tokens)
                .map(|c| format!(" | ~${c:.3}"))
                .unwrap_or_default();
            // Show effective model when a routing hint was used.
            let model_suffix = cli.hint.as_deref()
                .and_then(|h| config.routing.get(h))
                .map(|m| format!(" · {m}"))
                .unwrap_or_default();
            eprintln!(
                "\x1b[2mtokens: {input_tokens} in / {output_tokens} out · {elapsed:.1}s{cost_part}{model_suffix}\x1b[0m"
            );
        }
    } else {
        // ── Interactive TUI mode ──────────────────────────────────────────────
        // shared_state holds both the agent and its MCP handles together so that
        // dropping the old state on /model switch reaps the old MCP processes.
        let approver = Arc::new(AutoApprover);

        let (tx_event, mut rx_event) = mpsc::channel::<TuiEvent>(32);
        let (tx_agent, rx_agent) = mpsc::channel::<AgentEvent>(64);

        // agent is in a RwLock (Arc<Agent>: Sync — fine for concurrent reads in Submit).
        // MCP handles are in a Mutex: Vec<Box<dyn Any+Send>> is Send but not Sync,
        // so RwLock won't work; Mutex<T> is Sync whenever T: Send.
        let shared_agent = Arc::new(RwLock::new(agent.clone()));
        let shared_handles = Arc::new(tokio::sync::Mutex::new(mcp_handles));
        let shared_config = config.clone();
        let approver2 = approver.clone();
        let tx_agent2 = tx_agent.clone();

        // Send TuiEvent::Quit on SIGTERM so the TUI can restore the terminal cleanly.
        #[cfg(unix)]
        {
            let tx_quit = tx_event.clone();
            tokio::spawn(async move {
                if let Ok(mut sig) =
                    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                {
                    sig.recv().await;
                    let _ = tx_quit.send(TuiEvent::Quit).await;
                }
            });
        }

        tokio::spawn(async move {
            while let Some(ev) = rx_event.recv().await {
                match ev {
                    TuiEvent::Quit => break,
                    TuiEvent::NewSession => {
                        shared_agent.read().await.clear_session("cli:tui");
                    }
                    TuiEvent::ChangeModel(model) => {
                        let mut new_cfg = (*shared_config).clone();
                        new_cfg.model = model;
                        let (new_agent, new_handles) = build_agent(Arc::new(new_cfg)).await;
                        // Drop old handles first — terminates previous MCP child processes.
                        *shared_handles.lock().await = new_handles;
                        *shared_agent.write().await = new_agent;
                    }
                    TuiEvent::Submit(task) => {
                        let _ = tx_agent2.send(AgentEvent::Thinking).await;
                        let current_agent = shared_agent.read().await.clone();

                        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel::<String>();
                        let (tool_tx, mut tool_rx) = mpsc::unbounded_channel::<String>();
                        let tx_agent3 = tx_agent2.clone();
                        let tx_agent4 = tx_agent2.clone();
                        tokio::spawn(async move {
                            while let Some(delta) = chunk_rx.recv().await {
                                let _ = tx_agent3.send(AgentEvent::OutputChunk(delta)).await;
                            }
                        });
                        tokio::spawn(async move {
                            while let Some(name) = tool_rx.recv().await {
                                let _ = tx_agent4.send(AgentEvent::ToolCall(name)).await;
                            }
                        });

                        match current_agent
                            .run_streaming(
                                &task,
                                approver2.clone(),
                                "cli",
                                chunk_tx,
                                Some(tool_tx),
                                None,
                                Some("cli:tui"),
                            )
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

        let toolsets = agent.tool_names_by_toolset();
        let skill_names =
            garudust_tools::toolsets::skills::load_skills_from_dir(&config.home_dir.join("skills"))
                .await
                .into_iter()
                .map(|s| s.name)
                .collect::<Vec<_>>();
        tui::Tui::run(
            tx_event,
            rx_agent,
            toolsets,
            skill_names,
            config.model.clone(),
        )
        .await?;
    }

    Ok(())
}
