use std::io::{self, Write};

use garudust_core::config::AgentConfig;

pub async fn run() -> anyhow::Result<()> {
    let home_dir = AgentConfig::garudust_dir();
    std::fs::create_dir_all(&home_dir)?;

    println!("Garudust Setup");
    println!("{}", "─".repeat(48));
    println!("Press Enter to accept the [default] value.\n");

    // ── Mode ──────────────────────────────────────────────────────────────────
    println!("Setup mode:");
    println!("  1) Quick — provider + model only");
    println!("  2) Full  — provider, model, and platform adapters");
    let mode = prompt("Choose mode", Some("1"));
    let full = matches!(mode.trim(), "2" | "full");
    println!();

    // ── Provider ──────────────────────────────────────────────────────────────
    println!("LLM Provider:");
    println!("  1) openrouter  — 200+ hosted models (openrouter.ai)");
    println!("  2) anthropic   — Claude directly");
    println!("  3) vllm        — self-hosted vLLM server");
    println!("  4) ollama      — local Ollama");
    println!("  5) custom      — any OpenAI-compatible endpoint");
    let choice = prompt("Choose provider", Some("1"));
    let provider = match choice.trim() {
        "2" | "anthropic" => "anthropic",
        "3" | "vllm" => "vllm",
        "4" | "ollama" => "ollama",
        "5" | "custom" => "custom",
        _ => "openrouter",
    };
    println!();

    // ── Credentials / endpoint ────────────────────────────────────────────────
    let mut env_vars: Vec<(&'static str, String)> = Vec::new();
    let mut custom_base_url: Option<String> = None;

    match provider {
        "anthropic" => {
            let k = prompt("ANTHROPIC_API_KEY", None);
            if !k.is_empty() {
                env_vars.push(("ANTHROPIC_API_KEY", k));
            }
        }
        "vllm" => {
            let url = prompt("VLLM_BASE_URL", Some("http://localhost:8000/v1"));
            let url = if url.is_empty() {
                "http://localhost:8000/v1".to_string()
            } else {
                url
            };
            env_vars.push(("VLLM_BASE_URL", url));
            let k = prompt("VLLM_API_KEY (Enter to skip)", Some(""));
            if !k.is_empty() {
                env_vars.push(("VLLM_API_KEY", k));
            }
        }
        "ollama" => {
            let url = prompt("OLLAMA_BASE_URL", Some("http://localhost:11434"));
            let url = if url.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                url
            };
            env_vars.push(("OLLAMA_BASE_URL", url));
        }
        "custom" => {
            let url = prompt("Base URL (e.g. http://localhost:8000/v1)", None);
            if !url.is_empty() {
                custom_base_url = Some(url);
            }
            let k = prompt("API key (Enter to skip)", Some(""));
            if !k.is_empty() {
                env_vars.push(("OPENROUTER_API_KEY", k));
            }
        }
        _ => {
            let k = prompt("OPENROUTER_API_KEY", None);
            if !k.is_empty() {
                env_vars.push(("OPENROUTER_API_KEY", k));
            }
        }
    }
    println!();

    // ── Model ─────────────────────────────────────────────────────────────────
    let default_model = match provider {
        "anthropic" => "claude-sonnet-4-6",
        "ollama" => "llama3.2",
        "openrouter" => "anthropic/claude-sonnet-4-6",
        _ => "",
    };
    let hint = if default_model.is_empty() {
        None
    } else {
        Some(default_model)
    };
    let model_input = prompt("Model", hint);
    let model = if model_input.is_empty() {
        default_model.to_string()
    } else {
        model_input
    };
    println!();

    // ── Optional tools + platform adapters (Full mode) ───────────────────────
    if full {
        println!("Optional Tools (Enter to skip each):");
        let tool_fields: &[(&str, &str)] = &[(
            "Brave Search API key (web_search tool)",
            "BRAVE_SEARCH_API_KEY",
        )];
        for (label, var) in tool_fields {
            let val = prompt(label, Some(""));
            if !val.is_empty() {
                env_vars.push((var, val));
            }
        }
        println!();

        println!("Platform Adapters (Enter to skip each):");
        let platform_fields: &[(&str, &str)] = &[
            ("Telegram bot token", "TELEGRAM_TOKEN"),
            ("Discord bot token", "DISCORD_TOKEN"),
            ("Slack bot token (xoxb-...)", "SLACK_BOT_TOKEN"),
            ("Slack app token (xapp-...)", "SLACK_APP_TOKEN"),
            ("Matrix homeserver URL", "MATRIX_HOMESERVER"),
            ("Matrix user (@bot:example.com)", "MATRIX_USER"),
            ("Matrix password", "MATRIX_PASSWORD"),
        ];
        for (label, var) in platform_fields {
            let val = prompt(label, Some(""));
            if !val.is_empty() {
                env_vars.push((var, val));
            }
        }
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    for (var, val) in &env_vars {
        AgentConfig::set_env_var(&home_dir, var, val)?;
    }

    let mut config = AgentConfig {
        home_dir: home_dir.clone(),
        provider: provider.to_string(),
        model,
        base_url: custom_base_url,
        ..AgentConfig::default()
    };
    config.save_yaml()?;

    println!("Configuration saved to {}", home_dir.display());
    println!();

    // ── Doctor ────────────────────────────────────────────────────────────────
    if let Some((_, key)) = env_vars.iter().find(|(v, _)| {
        matches!(
            *v,
            "ANTHROPIC_API_KEY" | "OPENROUTER_API_KEY" | "VLLM_API_KEY"
        )
    }) {
        config.api_key = Some(key.clone());
    }
    super::doctor::run(&config).await;

    Ok(())
}

fn prompt(label: &str, default: Option<&str>) -> String {
    if let Some(d) = default {
        if d.is_empty() {
            print!("  {label}: ");
        } else {
            print!("  {label} [{d}]: ");
        }
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
