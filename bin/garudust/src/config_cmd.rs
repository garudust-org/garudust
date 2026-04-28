use std::path::Path;

use garudust_core::config::AgentConfig;

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
    "VLLM_BASE_URL",
    "OLLAMA_BASE_URL",
    "GARUDUST_MODEL",
    "GARUDUST_BASE_URL",
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

pub fn show(config: &AgentConfig) {
    println!("Garudust Config");
    println!("{}", "─".repeat(48));
    println!("home_dir        : {}", config.home_dir.display());
    println!("provider        : {}", config.provider);
    println!("model           : {}", config.model);
    println!("max_iterations  : {}", config.max_iterations);
    println!("tool_delay_ms   : {}", config.tool_delay_ms);
    println!(
        "base_url        : {}",
        config.base_url.as_deref().unwrap_or("(default)")
    );
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
