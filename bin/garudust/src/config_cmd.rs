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
    // server
    "server.port",
    // cron
    "cron.memory_consolidation",
    "cron.memory_expiry",
    // platforms (empty string clears/disables)
    "platforms.line.enabled",
    "platforms.line.port",
    "platforms.line.webhook_path",
    "platforms.whatsapp.enabled",
    "platforms.whatsapp.port",
    "platforms.whatsapp.webhook_path",
    "platforms.webhook.enabled",
    "platforms.webhook.port",
    "platforms.webhook.webhook_path",
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

    println!("Server (config.yaml → server.*)");
    println!("{}", "─".repeat(48));
    println!("port            : {}", config.server.port);
    println!();

    println!("Cron (config.yaml → cron.*)");
    println!("{}", "─".repeat(48));
    println!("jobs            : {}", config.cron.jobs.len());
    for (i, job) in config.cron.jobs.iter().enumerate() {
        println!("  [{i}] {} — {}", job.schedule, job.task);
    }
    println!(
        "memory_consolidation: {}",
        config
            .cron
            .memory_consolidation
            .as_deref()
            .unwrap_or("(disabled)")
    );
    println!(
        "memory_expiry       : {}",
        config.cron.memory_expiry.as_deref().unwrap_or("(disabled)")
    );
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

fn platform_or_insert<'a>(
    slot: &'a mut Option<WebhookPlatformConfig>,
    default_port: u16,
    default_path: &str,
) -> &'a mut WebhookPlatformConfig {
    slot.get_or_insert_with(|| WebhookPlatformConfig {
        enabled: false,
        port: default_port,
        webhook_path: default_path.into(),
    })
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
        "server.port" => config.server.port = value.parse()?,
        "cron.memory_consolidation" => {
            config.cron.memory_consolidation = if value.is_empty() {
                None
            } else {
                Some(value.into())
            };
        }
        "cron.memory_expiry" => {
            config.cron.memory_expiry = if value.is_empty() {
                None
            } else {
                Some(value.into())
            };
        }
        "platforms.line.enabled" => {
            platform_or_insert(&mut config.platforms.line, 3002, "/line").enabled =
                value.parse()?;
        }
        "platforms.line.port" => {
            platform_or_insert(&mut config.platforms.line, 3002, "/line").port = value.parse()?;
        }
        "platforms.line.webhook_path" => {
            platform_or_insert(&mut config.platforms.line, 3002, "/line").webhook_path =
                value.into();
        }
        "platforms.whatsapp.enabled" => {
            platform_or_insert(&mut config.platforms.whatsapp, 3003, "/whatsapp").enabled =
                value.parse()?;
        }
        "platforms.whatsapp.port" => {
            platform_or_insert(&mut config.platforms.whatsapp, 3003, "/whatsapp").port =
                value.parse()?;
        }
        "platforms.whatsapp.webhook_path" => {
            platform_or_insert(&mut config.platforms.whatsapp, 3003, "/whatsapp").webhook_path =
                value.into();
        }
        "platforms.webhook.enabled" => {
            platform_or_insert(&mut config.platforms.webhook, 8080, "/webhook").enabled =
                value.parse()?;
        }
        "platforms.webhook.port" => {
            platform_or_insert(&mut config.platforms.webhook, 8080, "/webhook").port =
                value.parse()?;
        }
        "platforms.webhook.webhook_path" => {
            platform_or_insert(&mut config.platforms.webhook, 8080, "/webhook").webhook_path =
                value.into();
        }
        _ => unreachable!(),
    }

    config.save_yaml()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use garudust_core::config::AgentConfig;

    fn parse(yaml: &str) -> AgentConfig {
        serde_yaml::from_str(yaml).expect("valid yaml")
    }

    #[test]
    fn platforms_line_roundtrip() {
        let cfg = parse(
            "platforms:\n  line:\n    enabled: true\n    port: 3002\n    webhook_path: /line\n",
        );
        let line = cfg.platforms.line.unwrap();
        assert!(line.enabled);
        assert_eq!(line.port, 3002);
        assert_eq!(line.webhook_path, "/line");
    }

    #[test]
    fn platforms_absent_is_none() {
        let cfg = parse("model: test\n");
        assert!(cfg.platforms.line.is_none());
        assert!(cfg.platforms.whatsapp.is_none());
    }

    #[test]
    fn cron_consolidation_roundtrip() {
        let cfg = parse("cron:\n  memory_consolidation: \"0 3 * * *\"\n");
        assert_eq!(cfg.cron.memory_consolidation.as_deref(), Some("0 3 * * *"));
    }

    #[test]
    fn cron_jobs_roundtrip() {
        let cfg =
            parse("cron:\n  jobs:\n    - schedule: \"0 9 * * *\"\n      task: morning report\n");
        assert_eq!(cfg.cron.jobs.len(), 1);
        assert_eq!(cfg.cron.jobs[0].schedule, "0 9 * * *");
        assert_eq!(cfg.cron.jobs[0].task, "morning report");
    }

    #[test]
    fn server_port_roundtrip() {
        let cfg = parse("server:\n  port: 4000\n");
        assert_eq!(cfg.server.port, 4000);
    }

    #[test]
    fn server_port_default() {
        let cfg = parse("model: test\n");
        assert_eq!(cfg.server.port, 3000);
    }
}
