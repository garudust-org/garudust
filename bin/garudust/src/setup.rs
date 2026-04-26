use std::io::{self, Write};

use garudust_core::config::AgentConfig;

pub async fn run() -> anyhow::Result<()> {
    let home_dir = AgentConfig::garudust_dir();
    std::fs::create_dir_all(&home_dir)?;

    println!("Garudust Setup");
    println!("{}", "─".repeat(48));
    println!("This wizard will configure Garudust for first use.");
    println!("Press Enter to accept the [default] value.");
    println!();

    // ── Provider ─────────────────────────────────────────────────────────────
    println!("Providers:");
    println!("  1) openrouter  — 200+ models (OpenRouter)");
    println!("  2) anthropic   — Claude directly");
    println!("  3) custom      — self-hosted or other endpoint");
    let provider_choice = prompt("Choose provider", Some("1"));
    let provider = match provider_choice.trim() {
        "2" | "anthropic" => "anthropic",
        "3" | "custom" => "custom",
        _ => "openrouter",
    };
    println!();

    // ── API key ───────────────────────────────────────────────────────────────
    let key_var = match provider {
        "anthropic" => "ANTHROPIC_API_KEY",
        _ => "OPENROUTER_API_KEY",
    };
    let api_key = prompt(key_var, None);

    // ── Base URL (custom only) ────────────────────────────────────────────────
    let base_url: Option<String> = if provider == "custom" {
        let url = prompt("Base URL (e.g. http://localhost:8000/v1)", None);
        if url.is_empty() {
            None
        } else {
            Some(url)
        }
    } else {
        None
    };

    // ── Model ─────────────────────────────────────────────────────────────────
    let default_model = match provider {
        "anthropic" => "claude-sonnet-4-6",
        _ => "anthropic/claude-sonnet-4-6",
    };
    let model = prompt("Model", Some(default_model));
    let model = if model.is_empty() {
        default_model.to_string()
    } else {
        model
    };
    println!();

    // ── Persist ───────────────────────────────────────────────────────────────
    if !api_key.is_empty() {
        AgentConfig::set_env_var(&home_dir, key_var, &api_key)?;
    }

    let mut config = AgentConfig {
        home_dir: home_dir.clone(),
        provider: provider.to_string(),
        model,
        base_url,
        ..AgentConfig::default()
    };
    config.save_yaml()?;

    println!("Configuration saved to {}", home_dir.display());
    println!();

    // ── Doctor check ─────────────────────────────────────────────────────────
    if !api_key.is_empty() {
        config.api_key = Some(api_key);
    }
    super::doctor::run(&config).await;

    Ok(())
}

fn prompt(label: &str, default: Option<&str>) -> String {
    if let Some(d) = default {
        print!("  {label} [{d}]: ");
    } else {
        print!("  {label}: ");
    }
    io::stdout().flush().ok();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap_or(0);
    let trimmed = buf.trim().to_string();

    if trimmed.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        trimmed
    }
}
