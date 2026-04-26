use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use garudust_agent::{Agent, AutoApprover};
use garudust_core::config::McpServerConfig;
use garudust_core::{config::AgentConfig, platform::PlatformAdapter};
use garudust_cron::CronScheduler;
use garudust_gateway::{create_router, AppState, GatewayHandler, SessionRegistry};
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_platforms::{
    discord::DiscordAdapter, telegram::TelegramAdapter, webhook::WebhookAdapter,
};
use garudust_tools::{
    toolsets::{
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

#[derive(Parser)]
#[command(name = "garudust-server", about = "Garudust headless gateway server")]
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

    /// Comma-separated list of cron jobs: "cron_expr=task" pairs
    /// e.g. "0 9 * * *=Good morning report"
    #[arg(long, env = "GARUDUST_CRON_JOBS")]
    cron_jobs: Option<String>,
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
) -> Result<()> {
    let name = platform.name();
    let handler = Arc::new(GatewayHandler::new(agent, platform.clone(), sessions));
    platform.start(handler).await?;
    tracing::info!("{name} adapter started");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();
    dotenvy::dotenv().ok();

    let cli = Cli::parse();
    let config = build_config(&cli);
    let db = Arc::new(SessionDb::open(&config.home_dir)?);
    let agent = build_agent(config.clone(), db.clone()).await;
    let sessions = SessionRegistry::new();

    // ── Platform adapters ─────────────────────────────────────────────────────
    if let Some(token) = &cli.telegram_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(TelegramAdapter::new(token.clone()));
        start_platform(platform, agent.clone(), sessions.clone()).await?;
    }

    if let Some(token) = &cli.discord_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(DiscordAdapter::new(token.clone()));
        start_platform(platform, agent.clone(), sessions.clone()).await?;
    }

    if cli.webhook_port > 0 {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(WebhookAdapter::new(cli.webhook_port));
        start_platform(platform, agent.clone(), sessions.clone()).await?;
    }

    // ── Cron scheduler ────────────────────────────────────────────────────────
    if let Some(jobs_str) = &cli.cron_jobs {
        let scheduler = CronScheduler::new(agent.clone(), Arc::new(AutoApprover)).await?;
        for entry in jobs_str.split(',') {
            if let Some((expr, task)) = entry.trim().split_once('=') {
                scheduler
                    .add_job(expr.trim(), task.trim().to_string())
                    .await?;
                tracing::info!(cron = %expr.trim(), task = %task.trim(), "cron job registered");
            }
        }
        scheduler.start().await?;
    }

    // ── HTTP gateway ──────────────────────────────────────────────────────────
    let state = AppState {
        config,
        session_db: db,
        agent,
    };
    let router = create_router(state);
    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("garudust-server listening on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
