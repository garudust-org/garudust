use std::sync::{Arc, OnceLock};

use anyhow::Result;
use arc_swap::ArcSwap;
use clap::Parser;
use garudust_agent::{Agent, AutoApprover, ConstitutionalApprover, DenyApprover};
use garudust_core::config::{get_secret, McpServerConfig, WebhookPlatformConfig};
use garudust_core::{config::AgentConfig, platform::PlatformAdapter, tool::CommandApprover};
use garudust_cron::CronScheduler;
use garudust_gateway::{create_router, AppState, GatewayHandler, Metrics, SessionRegistry};
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_platforms::{
    discord::DiscordAdapter, line::LineAdapter, matrix::MatrixAdapter, slack::SlackAdapter,
    telegram::TelegramAdapter, webhook::WebhookAdapter, whatsapp::WhatsAppAdapter,
};
use garudust_tools::{
    load_script_tools, register_standard_tools, security::docker_available,
    toolsets::mcp::connect_mcp_server, ToolRegistry,
};
use garudust_transport::build_transport;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

// Each element is held only for its Drop impl — dropping terminates the MCP child process.
type McpHandles = Vec<Box<dyn std::any::Any + Send>>;

#[derive(Parser)]
#[command(
    name = "garudust-server",
    about = "Garudust headless gateway server",
    version
)]
struct Cli {
    /// HTTP gateway port. Falls back to `server.port` in config.yaml or
    /// `GARUDUST_PORT` in `~/.garudust/.env` (default `3000`).
    #[arg(long)]
    port: Option<u16>,

    /// Override model. Falls back to `model` in config.yaml or `GARUDUST_MODEL`
    /// in `~/.garudust/.env`.
    #[arg(long)]
    model: Option<String>,

    /// Override LLM API key. Flag-only — the server reads the key for the
    /// configured provider from `~/.garudust/.env` via strict provider→env
    /// binding (see `AgentConfig::load`). Use this flag for ad-hoc one-off runs.
    #[arg(long)]
    api_key: Option<String>,

    /// Sets provider=anthropic and overrides the LLM API key. Flag-only.
    #[arg(long)]
    anthropic_key: Option<String>,

    /// Telegram bot token override. Falls back to `TELEGRAM_TOKEN` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    telegram_token: Option<String>,

    /// Discord bot token override. Falls back to `DISCORD_TOKEN` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    discord_token: Option<String>,

    /// Slack bot token override. Falls back to `SLACK_BOT_TOKEN` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    slack_bot_token: Option<String>,

    /// Slack app token override. Falls back to `SLACK_APP_TOKEN` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    slack_app_token: Option<String>,

    /// Matrix homeserver URL override. Falls back to `MATRIX_HOMESERVER` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    matrix_homeserver: Option<String>,

    /// Matrix user override. Falls back to `MATRIX_USER` in `~/.garudust/.env`.
    #[arg(long)]
    matrix_user: Option<String>,

    /// Matrix password override. Falls back to `MATRIX_PASSWORD` in
    /// `~/.garudust/.env`.
    #[arg(long)]
    matrix_password: Option<String>,

    /// Comma-separated list of cron jobs: "cron_expr=task" pairs
    /// e.g. "0 9 * * *=Good morning report". Falls back to `cron.jobs` in
    /// config.yaml or `GARUDUST_CRON_JOBS` in `~/.garudust/.env`.
    #[arg(long)]
    cron_jobs: Option<String>,

    /// Cron expression for automatic memory consolidation (default disabled).
    /// Example: "0 3 * * *" runs daily at 03:00. Falls back to
    /// `cron.memory_consolidation` in config.yaml or `GARUDUST_MEMORY_CRON`.
    #[arg(long)]
    memory_cron: Option<String>,

    /// Cron expression for automatic memory expiry (default disabled).
    /// Example: "0 4 * * *" runs daily at 04:00. Falls back to
    /// `cron.memory_expiry` in config.yaml or `GARUDUST_MEMORY_EXPIRY_CRON`.
    #[arg(long)]
    memory_expiry_cron: Option<String>,

    /// Command approval mode for tool execution
    #[arg(long, env = "GARUDUST_APPROVAL_MODE", default_value = "smart")]
    approval_mode: ApprovalMode,
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum ApprovalMode {
    /// Approve all commands without logging (use with caution)
    Auto,
    /// Constitutional approval: audit-log every destructive tool call;
    /// the system prompt's constitutional constraints are the primary gate
    Smart,
    /// Deny all destructive tool calls unconditionally
    Deny,
}

fn build_approver(mode: &ApprovalMode) -> Arc<dyn CommandApprover> {
    match mode {
        ApprovalMode::Auto => Arc::new(AutoApprover),
        ApprovalMode::Smart => Arc::new(ConstitutionalApprover),
        ApprovalMode::Deny => Arc::new(DenyApprover),
    }
}

fn build_config(cli: &Cli) -> Arc<AgentConfig> {
    let mut config = AgentConfig::load();
    if let Some(m) = &cli.model {
        config.model.clone_from(m);
    }
    if let Some(k) = &cli.anthropic_key {
        config.api_key = Some(k.clone());
        config.provider = "anthropic".into();
    } else if let Some(k) = &cli.api_key {
        config.api_key = Some(k.clone());
    }
    Arc::new(config)
}

static WARN_SANDBOX_NONE: OnceLock<()> = OnceLock::new();
static WARN_DOCKER_MISSING: OnceLock<()> = OnceLock::new();

async fn build_agent(config: Arc<AgentConfig>, db: Arc<SessionDb>) -> (Arc<Agent>, McpHandles) {
    let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
    let transport = build_transport(&config);

    if config.security.terminal_sandbox == garudust_core::config::TerminalSandbox::Docker
        && !docker_available()
    {
        WARN_DOCKER_MISSING.get_or_init(|| {
            tracing::warn!(
                "terminal_sandbox is set to 'docker' but Docker is not installed or not in PATH. \
                 Terminal commands will fail. Set `terminal_sandbox: none` or install Docker."
            );
        });
    }

    if config.security.terminal_sandbox == garudust_core::config::TerminalSandbox::None {
        WARN_SANDBOX_NONE.get_or_init(|| {
            tracing::warn!(
                "terminal_sandbox is 'none' — terminal commands run directly on the host. \
                 Set GARUDUST_TERMINAL_SANDBOX=docker to isolate execution in a container."
            );
        });
    }

    let mut registry = ToolRegistry::new();
    register_standard_tools(&mut registry, Some(db.clone()));

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

    let agent =
        Arc::new(Agent::new(transport, Arc::new(registry), memory, config).with_session_db(db));
    (agent, mcp_handles)
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

/// Returns the platform config only when present and `enabled = true`. Logs
/// at info level so operators can see why an expected adapter did not start.
fn enabled_platform<'a>(
    cfg: Option<&'a WebhookPlatformConfig>,
    name: &str,
) -> Option<&'a WebhookPlatformConfig> {
    match cfg {
        Some(c) if c.enabled => Some(c),
        Some(_) => {
            tracing::info!("{name} platform present in config but enabled=false — skipping");
            None
        }
        None => None,
    }
}

async fn start_platform(
    platform: Arc<dyn PlatformAdapter>,
    agent: Arc<Agent>,
    sessions: Arc<SessionRegistry>,
    approver: Arc<dyn CommandApprover>,
    config: Arc<AgentConfig>,
) -> Result<()> {
    let name = platform.name();
    let handler = Arc::new(GatewayHandler::new(
        agent,
        platform.clone(),
        sessions,
        approver,
        config,
    ));
    platform.start(handler).await?;
    tracing::info!("{name} adapter started");
    Ok(())
}

fn spawn_config_watcher(
    config_path: std::path::PathBuf,
    agent_swap: Arc<ArcSwap<Agent>>,
    db: Arc<SessionDb>,
    handles_lock: Arc<tokio::sync::Mutex<McpHandles>>,
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    // Filename to match against incoming events. We watch the parent dir (so atomic
    // write+rename saves are visible) but must filter to ONLY this filename —
    // otherwise unrelated writes in ~/.garudust/ (state.db, state.db-wal, conversation
    // files) trigger reload loops every few hundred ms as the agent writes to its DB.
    let config_filename = config_path
        .file_name()
        .map(std::ffi::OsString::from)
        .unwrap_or_default();

    tokio::spawn(async move {
        let tx2 = tx.clone();
        let filename = config_filename.clone();
        let mut watcher: RecommendedWatcher =
            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    let matches = event
                        .paths
                        .iter()
                        .any(|p| p.file_name() == Some(filename.as_os_str()));
                    if matches {
                        let _ = tx2.send(());
                    }
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("config watcher init failed: {e}");
                    return;
                }
            };

        // Watch the parent dir so we catch atomic saves (write+rename)
        let watch_dir = config_path
            .parent()
            .map_or_else(|| config_path.clone(), std::path::Path::to_path_buf);

        if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::NonRecursive) {
            tracing::warn!("could not watch config dir {}: {e}", watch_dir.display());
            return;
        }

        tracing::info!("hot-reload: watching {} for changes", watch_dir.display());

        while rx.recv().await.is_some() {
            // debounce: wait for quiet period
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            while rx.try_recv().is_ok() {}

            tracing::info!("config changed — reloading agent");
            let new_config = Arc::new(AgentConfig::load());
            let (new_agent, new_handles) = build_agent(new_config, db.clone()).await;
            // Swap agent first so new requests immediately use the new config, then
            // drop old handles. This narrows (but does not eliminate) the window where
            // in-flight MCP tool calls from the old agent hit terminated child processes;
            // fully quiescing the old agent would require request-level draining which is
            // not yet implemented. The race is acceptable for the hot-reload use case.
            agent_swap.store(new_agent);
            *handles_lock.lock().await = new_handles;
            tracing::info!("agent hot-reloaded successfully");
        }

        drop(watcher);
    });
}

/// Resolves when SIGINT (Ctrl-C) or SIGTERM is received.
/// If a signal handler cannot be installed, falls back to pending() for that
/// signal so the server degrades gracefully (Ctrl-C only) rather than shutting
/// down immediately at startup.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::warn!("Ctrl-C handler unavailable: {e}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let sigterm: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => Box::pin(async move {
                // Some(()) = SIGTERM received; None = stream closed — both resolve the select.
                s.recv().await;
            }),
            Err(e) => {
                tracing::warn!("SIGTERM handler unavailable, falling back to Ctrl-C only: {e}");
                Box::pin(std::future::pending())
            }
        };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c  => { tracing::info!("received SIGINT, initiating graceful shutdown"); }
        () = sigterm => { tracing::info!("received SIGTERM, initiating graceful shutdown"); }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .with_writer(std::io::stderr)
        .init();
    // Secrets and non-secret env vars from `~/.garudust/.env` are loaded into an
    // in-memory map by `AgentConfig::load` (see config.rs), then read on demand
    // via `get_secret`. We deliberately do not call `dotenvy::from_path` — that
    // would dump every secret into the process environment where any spawned
    // tool subprocess could read it.

    let cli = Cli::parse();
    let config = build_config(&cli);
    let port = cli.port.unwrap_or(config.server.port);

    tracing::info!(
        "garudust-server {}  |  model: {}  |  provider: {}  |  port: {}",
        env!("CARGO_PKG_VERSION"),
        config.model,
        config.provider,
        port,
    );
    let db = Arc::new(SessionDb::open(&config.home_dir)?);
    let (agent_inner, mcp_handles) = build_agent(config.clone(), db.clone()).await;
    let agent = Arc::new(ArcSwap::from(agent_inner));
    let mcp_handles = Arc::new(tokio::sync::Mutex::new(mcp_handles));
    let sessions = SessionRegistry::new();
    let approver = build_approver(&cli.approval_mode);

    if config.security.gateway_api_key.is_none() {
        tracing::warn!(
            "GARUDUST_API_KEY is not set — HTTP gateway is open to all callers. \
             Set this variable to enable Bearer token authentication."
        );
    }

    // ── Hot-reload watcher ────────────────────────────────────────────────────
    let config_file = config.home_dir.join("config.yaml");
    spawn_config_watcher(config_file, agent.clone(), db.clone(), mcp_handles.clone());

    // ── Platform adapters ─────────────────────────────────────────────────────
    // For each adapter, the CLI flag takes precedence; otherwise the secret is
    // read from `~/.garudust/.env` via `get_secret` (in-memory map — never
    // exposed to subprocess tools). This mirrors the LINE/WhatsApp wiring
    // below and avoids leaking secrets into the process environment.
    if let Some(token) = cli
        .telegram_token
        .clone()
        .or_else(|| get_secret("TELEGRAM_TOKEN"))
    {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(TelegramAdapter::new(token));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let Some(token) = cli
        .discord_token
        .clone()
        .or_else(|| get_secret("DISCORD_TOKEN"))
    {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(DiscordAdapter::new(token));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let Some(cfg) = enabled_platform(config.platforms.webhook.as_ref(), "webhook") {
        let platform: Arc<dyn PlatformAdapter> =
            Arc::new(WebhookAdapter::new(cfg.port, cfg.webhook_path.clone()));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    let slack_bot = cli
        .slack_bot_token
        .clone()
        .or_else(|| get_secret("SLACK_BOT_TOKEN"));
    let slack_app = cli
        .slack_app_token
        .clone()
        .or_else(|| get_secret("SLACK_APP_TOKEN"));
    if let (Some(bot_token), Some(app_token)) = (slack_bot, slack_app) {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(SlackAdapter::new(bot_token, app_token));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    let matrix_hs = cli
        .matrix_homeserver
        .clone()
        .or_else(|| get_secret("MATRIX_HOMESERVER"));
    let matrix_user = cli
        .matrix_user
        .clone()
        .or_else(|| get_secret("MATRIX_USER"));
    let matrix_pw = cli
        .matrix_password
        .clone()
        .or_else(|| get_secret("MATRIX_PASSWORD"));
    if let (Some(homeserver), Some(user), Some(password)) = (matrix_hs, matrix_user, matrix_pw) {
        let platform: Arc<dyn PlatformAdapter> =
            Arc::new(MatrixAdapter::new(homeserver, user, password));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let Some(cfg) = enabled_platform(config.platforms.line.as_ref(), "line") {
        if let (Some(token), Some(secret)) = (
            get_secret("LINE_CHANNEL_TOKEN"),
            get_secret("LINE_CHANNEL_SECRET"),
        ) {
            let platform: Arc<dyn PlatformAdapter> = Arc::new(LineAdapter::new(
                token,
                secret,
                cfg.port,
                cfg.webhook_path.clone(),
            ));
            start_platform(
                platform,
                agent.load_full(),
                sessions.clone(),
                approver.clone(),
                config.clone(),
            )
            .await?;
        } else {
            tracing::error!(
                "LINE platform enabled in config but LINE_CHANNEL_TOKEN / \
                 LINE_CHANNEL_SECRET missing in ~/.garudust/.env — adapter not started"
            );
        }
    }

    if let Some(cfg) = enabled_platform(config.platforms.whatsapp.as_ref(), "whatsapp") {
        if let (Some(token), Some(phone_id)) = (
            get_secret("WHATSAPP_ACCESS_TOKEN"),
            get_secret("WHATSAPP_PHONE_NUMBER_ID"),
        ) {
            let platform: Arc<dyn PlatformAdapter> = Arc::new(WhatsAppAdapter::new(
                token,
                phone_id,
                get_secret("WHATSAPP_APP_SECRET").unwrap_or_default(),
                get_secret("WHATSAPP_VERIFY_TOKEN").unwrap_or_default(),
                cfg.port,
                cfg.webhook_path.clone(),
            ));
            start_platform(
                platform,
                agent.load_full(),
                sessions.clone(),
                approver.clone(),
                config.clone(),
            )
            .await?;
        } else {
            tracing::error!(
                "WhatsApp platform enabled in config but WHATSAPP_ACCESS_TOKEN / \
                 WHATSAPP_PHONE_NUMBER_ID missing in ~/.garudust/.env — adapter not started"
            );
        }
    }

    // ── Cron scheduler ────────────────────────────────────────────────────────
    // Precedence: CLI flag / env var (already merged by clap) > yaml. For
    // `cron_jobs`, the CLI form is a comma-separated string and the yaml form
    // is a structured list — we materialize both into the same Vec<(expr, task)>.
    let cron_jobs: Vec<(String, String)> = match &cli.cron_jobs {
        Some(s) => garudust_cron::parse_job_pairs(s),
        None => config
            .cron
            .jobs
            .iter()
            .map(|j| (j.schedule.clone(), j.task.clone()))
            .collect(),
    };
    let memory_cron = cli
        .memory_cron
        .clone()
        .or_else(|| config.cron.memory_consolidation.clone());
    let memory_expiry_cron = cli
        .memory_expiry_cron
        .clone()
        .or_else(|| config.cron.memory_expiry.clone());

    let needs_cron = !cron_jobs.is_empty() || memory_cron.is_some() || memory_expiry_cron.is_some();
    if needs_cron {
        let scheduler = CronScheduler::new(agent.load_full(), approver.clone()).await?;

        for (expr, task) in &cron_jobs {
            scheduler.add_job(expr, task.clone()).await?;
            tracing::info!(cron = %expr, task = %task, "cron job registered");
        }

        if let Some(expr) = &memory_expiry_cron {
            let expiry_config = config.memory_expiry.clone();
            let home_dir = config.home_dir.clone();
            scheduler
                .add_fn_job(expr.trim(), move || {
                    let expiry_config = expiry_config.clone();
                    let home_dir = home_dir.clone();
                    async move {
                        let store = FileMemoryStore::new(&home_dir);
                        match store.expire_entries(&expiry_config).await {
                            Ok(0) => tracing::info!("memory expiry: no entries expired"),
                            Ok(n) => {
                                tracing::info!(removed = n, "memory expiry: removed old entries");
                            }
                            Err(e) => tracing::error!("memory expiry failed: {e}"),
                        }
                    }
                })
                .await?;
            tracing::info!(cron = %expr.trim(), "memory expiry cron registered");
        }

        if let Some(expr) = &memory_cron {
            const CONSOLIDATION_TASK: &str =
                "Review and consolidate your memory. Use the `memory` tool to read all current \
                 entries. Then rewrite them: remove exact duplicates, merge entries that say the \
                 same thing, discard facts that are clearly outdated or no longer relevant, and \
                 keep the result to 50 entries or fewer. Write the consolidated entries back \
                 using `memory` tool with 'replace' or 'remove' + 'add' actions. \
                 Do not add any new information — only reorganise what is already there.";
            scheduler
                .add_job(expr.trim(), CONSOLIDATION_TASK.to_string())
                .await?;
            tracing::info!(cron = %expr.trim(), "memory consolidation cron registered");
        }

        scheduler.start().await?;
    }

    // ── HTTP gateway ──────────────────────────────────────────────────────────
    let shutdown_secs = config.shutdown_timeout_secs;
    let state = AppState {
        config,
        session_db: db,
        agent,
        metrics: Arc::new(Metrics::default()),
        approver,
    };
    let router = create_router(state);
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("garudust-server listening on {addr}");

    // Signal the drain-timeout task only after shutdown_signal() resolves so the
    // countdown starts when the signal fires, not when the server starts listening.
    let (drain_tx, mut drain_rx) = tokio::sync::watch::channel(false);
    let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
        shutdown_signal().await;
        tracing::info!(drain_secs = shutdown_secs, "draining in-flight requests");
        let _ = drain_tx.send(true);
    });
    tokio::spawn(async move {
        // Wait until the graceful-shutdown future has fired the signal.
        let _ = drain_rx.wait_for(|v| *v).await;
        if shutdown_secs > 0 {
            tokio::time::sleep(tokio::time::Duration::from_secs(shutdown_secs)).await;
            tracing::warn!(
                drain_secs = shutdown_secs,
                "drain timeout exceeded — forcing exit; MCP child processes may need manual cleanup"
            );
            std::process::exit(1);
        }
    });
    serve.await?;

    // Explicit drop ensures MCP child processes exit before the server process does.
    drop(mcp_handles);
    tracing::info!("shutdown complete");

    Ok(())
}
