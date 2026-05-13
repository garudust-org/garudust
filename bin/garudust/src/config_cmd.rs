use std::io::{self, Write};
use std::path::Path;

use garudust_core::config::{AgentConfig, WebhookPlatformConfig};

const SECRET_KEYS: &[&str] = &[
    "OPENROUTER_API_KEY",
    "ANTHROPIC_API_KEY",
    "VLLM_API_KEY",
    "BRAVE_SEARCH_API_KEY",
    "GARUDUST_API_KEY",
    "TELEGRAM_TOKEN",
    "DISCORD_TOKEN",
    "SLACK_BOT_TOKEN",
    "SLACK_APP_TOKEN",
    "MATRIX_PASSWORD",
];

const ENV_KEYS: &[&str] = &[
    "GARUDUST_APPROVAL_MODE",
    "GARUDUST_RATE_LIMIT",
    "MATRIX_HOMESERVER",
    "MATRIX_USER",
];

const YAML_KEYS: &[&str] = &[
    "model",
    "provider",
    "base_url",
    "max_iterations",
    "tool_delay_ms",
];

fn print_platform(label: &str, cfg: Option<&WebhookPlatformConfig>) {
    match cfg {
        Some(c) => println!(
            "{label}: enabled={}, port={}, path={}",
            c.enabled, c.port, c.webhook_path
        ),
        None => println!("{label}: not configured"),
    }
}

pub fn show(config: &AgentConfig) {
    println!("Garudust Config");
    println!("{}", "─".repeat(48));
    println!("home_dir        : {}", config.home_dir.display());
    println!("provider        : {}", config.provider);
    println!("model           : {}", config.model);
    println!("max_iterations  : {}", config.max_iterations);
    println!("tool_delay_ms   : {}", config.tool_delay_ms);
    let effective_url = config
        .base_url
        .as_deref()
        .unwrap_or(match config.provider.as_str() {
            "anthropic" => "https://api.anthropic.com/v1/messages (native)",
            "openrouter" => "https://openrouter.ai/api/v1",
            "ollama" => "http://localhost:11434/v1",
            "vllm" => "http://localhost:8000/v1",
            "thaillm" => "http://thaillm.or.th/api/v1",
            "bedrock" => "AWS SDK (no base_url)",
            _ => "(default)",
        });
    println!("base_url        : {effective_url}");
    println!("approval_mode   : {}", config.security.approval_mode);
    let key_display = match &config.api_key {
        Some(k) if k.len() > 10 => format!("{}…{}", &k[..6], &k[k.len() - 4..]),
        Some(_) => "set".into(),
        None => "not set".into(),
    };
    println!("api_key         : {key_display}");
    println!(
        "compression     : enabled={}, threshold={}",
        config.compression.enabled, config.compression.threshold_fraction
    );
    println!();

    println!("Webhook Platforms (config.yaml → platforms.*)");
    println!("{}", "─".repeat(48));
    print_platform("webhook ", config.platforms.webhook.as_ref());
    print_platform("line    ", config.platforms.line.as_ref());
    print_platform("whatsapp", config.platforms.whatsapp.as_ref());
    println!();

    let yaml_path = config.home_dir.join("config.yaml");
    let env_path = config.home_dir.join(".env");
    println!(
        "config.yaml : {}",
        if yaml_path.exists() {
            yaml_path.display().to_string()
        } else {
            format!("{} (not yet created)", yaml_path.display())
        }
    );
    println!(
        ".env        : {}",
        if env_path.exists() {
            env_path.display().to_string()
        } else {
            format!("{} (not yet created)", env_path.display())
        }
    );
    println!();
    println!("Tip: run 'garudust setup' to configure interactively.");
}

pub fn set(key: &str, value: &str, home_dir: &Path) -> anyhow::Result<()> {
    let upper = key.to_uppercase();

    if SECRET_KEYS.contains(&upper.as_str()) {
        AgentConfig::set_env_var(home_dir, &upper, value)?;
        println!("[✓] {} saved to {}", upper, home_dir.join(".env").display());
        return Ok(());
    }

    if ENV_KEYS.contains(&upper.as_str()) {
        AgentConfig::set_env_var(home_dir, &upper, value)?;
        println!("[✓] {} saved to {}", upper, home_dir.join(".env").display());
        return Ok(());
    }

    if YAML_KEYS.contains(&key) {
        update_yaml(key, value, home_dir)?;
        println!(
            "[✓] {key} = {value} saved to {}",
            home_dir.join("config.yaml").display()
        );
        return Ok(());
    }

    anyhow::bail!(
        "Unknown key: '{key}'\n\nSecret keys (saved to .env):\n  {}\n\nEnv keys (saved to .env):\n  {}\n\nConfig keys (saved to config.yaml):\n  {}",
        SECRET_KEYS.join(", "),
        ENV_KEYS.join(", "),
        YAML_KEYS.join(", "),
    )
}

pub fn set_model(name: Option<&str>, config: &AgentConfig) -> anyhow::Result<()> {
    let new_model = if let Some(n) = name {
        n.to_string()
    } else {
        println!("Current model: {}", config.model);
        print!("  New model [{}]: ", config.model);
        io::stdout().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let input = buf.trim().to_string();
        if input.is_empty() {
            println!("Model unchanged.");
            return Ok(());
        }
        input
    };
    update_yaml("model", &new_model, &config.home_dir)?;
    println!("[✓] model = {new_model}");
    Ok(())
}

fn update_yaml(key: &str, value: &str, home_dir: &Path) -> anyhow::Result<()> {
    let yaml_path = home_dir.join("config.yaml");
    let mut config: AgentConfig = if yaml_path.exists() {
        let src = std::fs::read_to_string(&yaml_path)?;
        serde_yaml::from_str(&src).unwrap_or_default()
    } else {
        AgentConfig::default()
    };
    config.home_dir = home_dir.to_path_buf();

    match key {
        "model" => config.model = value.into(),
        "provider" => config.provider = value.into(),
        "base_url" => {
            config.base_url = if value.is_empty() {
                None
            } else {
                Some(value.into())
            }
        }
        "max_iterations" => config.max_iterations = value.parse()?,
        "tool_delay_ms" => config.tool_delay_ms = value.parse()?,
        _ => unreachable!(),
    }

    config.save_yaml()?;
    Ok(())
}
