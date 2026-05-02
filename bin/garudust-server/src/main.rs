use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use clap::Parser;
use garudust_agent::{Agent, AutoApprover, ConstitutionalApprover, DenyApprover};
use garudust_core::config::McpServerConfig;
use garudust_core::{config::AgentConfig, platform::PlatformAdapter, tool::CommandApprover};
use garudust_cron::CronScheduler;
use garudust_gateway::{create_router, AppState, GatewayHandler, Metrics, SessionRegistry};
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_platforms::{
    discord::DiscordAdapter, line::LineAdapter, matrix::MatrixAdapter, slack::SlackAdapter,
    telegram::TelegramAdapter, webhook::WebhookAdapter,
};
use garudust_tools::{
    security::docker_available,
    toolsets::{
        browser::BrowserTool,
        delegate::DelegateTask,
        files::{ReadFile, WriteFile},
        mcp::connect_mcp_server,
        memory::{MemoryTool, UserProfileTool},
        search::SessionSearch,
        skills::{SkillView, SkillsList, WriteSkill},
        terminal::Terminal,
        web::{WebFetch, WebSearch},
    },
    ToolRegistry,
};
use garudust_transport::build_transport;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Parser)]
#[command(
    name = "garudust-server",
    about = "Garudust headless gateway server",
    version
)]
struct Cli {
    #[arg(long, env = "GARUDUST_PORT", default_value = "3000")]
    port: u16,

    /// Port for the webhook adapter (0 = disabled)
    #[arg(long, env = "GARUDUST_WEBHOOK_PORT", default_value = "3001")]
    webhook_port: u16,

    /// Override model
    #[arg(long, env = "GARUDUST_MODEL")]
    model: Option<String>,

    #[arg(long, env = "OPENROUTER_API_KEY")]
    api_key: Option<String>,

    /// Sets provider=anthropic when provided
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    anthropic_key: Option<String>,

    #[arg(long, env = "TELEGRAM_TOKEN")]
    telegram_token: Option<String>,

    #[arg(long, env = "DISCORD_TOKEN")]
    discord_token: Option<String>,

    #[arg(long, env = "SLACK_BOT_TOKEN")]
    slack_bot_token: Option<String>,

    #[arg(long, env = "SLACK_APP_TOKEN")]
    slack_app_token: Option<String>,

    #[arg(long, env = "MATRIX_HOMESERVER")]
    matrix_homeserver: Option<String>,

    #[arg(long, env = "MATRIX_USER")]
    matrix_user: Option<String>,

    #[arg(long, env = "MATRIX_PASSWORD")]
    matrix_password: Option<String>,

    #[arg(long, env = "LINE_CHANNEL_TOKEN")]
    line_channel_token: Option<String>,

    #[arg(long, env = "LINE_CHANNEL_SECRET")]
    line_channel_secret: Option<String>,

    /// Port for the LINE webhook receiver (0 = disabled)
    #[arg(long, env = "GARUDUST_LINE_PORT", default_value = "3002")]
    line_port: u16,

    /// Comma-separated list of cron jobs: "cron_expr=task" pairs
    /// e.g. "0 9 * * *=Good morning report"
    #[arg(long, env = "GARUDUST_CRON_JOBS")]
    cron_jobs: Option<String>,

    /// Cron expression for automatic memory consolidation (default disabled).
    /// Example: "0 3 * * *" runs daily at 03:00.
    #[arg(long, env = "GARUDUST_MEMORY_CRON")]
    memory_cron: Option<String>,

    /// Cron expression for automatic memory expiry (default disabled).
    /// Example: "0 4 * * *" runs daily at 04:00.
    #[arg(long, env = "GARUDUST_MEMORY_EXPIRY_CRON")]
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

async fn build_agent(config: Arc<AgentConfig>, db: Arc<SessionDb>) -> Arc<Agent> {
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

    let mut registry = ToolRegistry::new();
    registry.register(WebFetch);
    registry.register(WebSearch);
    registry.register(ReadFile);
    registry.register(WriteFile);
    registry.register(Terminal);
    registry.register(MemoryTool);
    registry.register(UserProfileTool);
    registry.register(SessionSearch);
    registry.register(SkillsList);
    registry.register(SkillView);
    registry.register(WriteSkill);
    registry.register(DelegateTask);
    registry.register(BrowserTool::new());

    let mcp_handles = attach_mcp_servers(&mut registry, &config.mcp_servers).await;
    std::mem::forget(mcp_handles);

    Arc::new(Agent::new(transport, Arc::new(registry), memory, config).with_session_db(db))
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
) {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    tokio::spawn(async move {
        let tx2 = tx.clone();
        let mut watcher: RecommendedWatcher =
            match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if res.is_ok() {
                    let _ = tx2.send(());
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
            let new_agent = build_agent(new_config, db.clone()).await;
            agent_swap.store(new_agent);
            tracing::info!("agent hot-reloaded successfully");
        }

        drop(watcher);
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    let config = build_config(&cli);

    tracing::info!(
        "garudust-server {}  |  model: {}  |  provider: {}  |  port: {}",
        env!("CARGO_PKG_VERSION"),
        config.model,
        config.provider,
        cli.port,
    );
    let db = Arc::new(SessionDb::open(&config.home_dir)?);
    let agent = Arc::new(ArcSwap::from(build_agent(config.clone(), db.clone()).await));
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
    spawn_config_watcher(config_file, agent.clone(), db.clone());

    // ── Platform adapters ─────────────────────────────────────────────────────
    if let Some(token) = &cli.telegram_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(TelegramAdapter::new(token.clone()));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let Some(token) = &cli.discord_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(DiscordAdapter::new(token.clone()));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if cli.webhook_port > 0 {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(WebhookAdapter::new(cli.webhook_port));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let (Some(bot_token), Some(app_token)) = (&cli.slack_bot_token, &cli.slack_app_token) {
        let platform: Arc<dyn PlatformAdapter> =
            Arc::new(SlackAdapter::new(bot_token.clone(), app_token.clone()));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let (Some(homeserver), Some(user), Some(password)) = (
        &cli.matrix_homeserver,
        &cli.matrix_user,
        &cli.matrix_password,
    ) {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(MatrixAdapter::new(
            homeserver.clone(),
            user.clone(),
            password.clone(),
        ));
        start_platform(
            platform,
            agent.load_full(),
            sessions.clone(),
            approver.clone(),
            config.clone(),
        )
        .await?;
    }

    if let (Some(token), Some(secret)) = (&cli.line_channel_token, &cli.line_channel_secret) {
        if cli.line_port > 0 {
            let platform: Arc<dyn PlatformAdapter> = Arc::new(LineAdapter::new(
                token.clone(),
                secret.clone(),
                cli.line_port,
            ));
            start_platform(
                platform,
                agent.load_full(),
                sessions.clone(),
                approver.clone(),
                config.clone(),
            )
            .await?;
        }
    }

    // ── Cron scheduler ────────────────────────────────────────────────────────
    let needs_cron =
        cli.cron_jobs.is_some() || cli.memory_cron.is_some() || cli.memory_expiry_cron.is_some();
    if needs_cron {
        let scheduler = CronScheduler::new(agent.load_full(), approver.clone()).await?;

        if let Some(jobs_str) = &cli.cron_jobs {
            for (expr, task) in garudust_cron::parse_job_pairs(jobs_str) {
                scheduler.add_job(&expr, task.clone()).await?;
                tracing::info!(cron = %expr, task = %task, "cron job registered");
            }
        }

        if let Some(expr) = &cli.memory_expiry_cron {
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

        if let Some(expr) = &cli.memory_cron {
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
    let state = AppState {
        config,
        session_db: db,
        agent,
        metrics: Arc::new(Metrics::default()),
        approver,
    };
    let router = create_router(state);
    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("garudust-server listening on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
