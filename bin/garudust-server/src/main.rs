use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use garudust_agent::Agent;
use garudust_core::{config::AgentConfig, platform::PlatformAdapter};
use garudust_gateway::{create_router, AppState, GatewayHandler, SessionRegistry};
use garudust_memory::{FileMemoryStore, SessionDb};
use garudust_platforms::{discord::DiscordAdapter, telegram::TelegramAdapter};
use garudust_tools::{
    toolsets::{
        files::{ReadFile, WriteFile},
        memory::MemoryTool,
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

    /// Override model (env: GARUDUST_MODEL)
    #[arg(long, env = "GARUDUST_MODEL")]
    model: Option<String>,

    /// Override OpenRouter API key (env: OPENROUTER_API_KEY)
    #[arg(long, env = "OPENROUTER_API_KEY")]
    api_key: Option<String>,

    /// Override Anthropic API key — sets provider=anthropic (env: ANTHROPIC_API_KEY)
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    anthropic_key: Option<String>,

    #[arg(long, env = "TELEGRAM_TOKEN")]
    telegram_token: Option<String>,

    #[arg(long, env = "DISCORD_TOKEN")]
    discord_token: Option<String>,
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

fn build_agent(config: Arc<AgentConfig>, db: Arc<SessionDb>) -> Arc<Agent> {
    let memory = Arc::new(FileMemoryStore::new(&config.home_dir));
    let transport = build_transport(&config);

    let mut registry = ToolRegistry::new();
    registry.register(WebFetch);
    registry.register(WebSearch);
    registry.register(ReadFile);
    registry.register(WriteFile);
    registry.register(Terminal);
    registry.register(MemoryTool);
    registry.register(SkillsList);
    registry.register(SkillView);

    Arc::new(Agent::new(transport, Arc::new(registry), memory, config).with_session_db(db))
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
    let agent = build_agent(config.clone(), db.clone());
    let sessions = SessionRegistry::new();

    // ── Platform adapters ────────────────────────────────────────────────────
    if let Some(token) = &cli.telegram_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(TelegramAdapter::new(token.clone()));
        start_platform(platform, agent.clone(), sessions.clone()).await?;
    }

    if let Some(token) = &cli.discord_token {
        let platform: Arc<dyn PlatformAdapter> = Arc::new(DiscordAdapter::new(token.clone()));
        start_platform(platform, agent.clone(), sessions.clone()).await?;
    }

    // ── HTTP gateway ─────────────────────────────────────────────────────────
    let state = AppState {
        config,
        session_db: db,
    };
    let router = create_router(state);
    let addr = format!("0.0.0.0:{}", cli.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("garudust-server listening on {addr}");
    axum::serve(listener, router).await?;

    Ok(())
}
