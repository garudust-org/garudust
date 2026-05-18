use std::io::{self, Write};
use std::path::Path;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{Attribute, Print, SetAttribute},
    terminal::{self, ClearType},
};
use garudust_core::config::{
    AgentConfig, BuiltinProvider, ProviderProfile, WebhookPlatformConfig, BUILTIN_PROVIDERS,
};

const C_RESET: &str = "\x1b[0m";
const C_BOLD: &str = "\x1b[1m";
const C_DIM: &str = "\x1b[2m";
const C_CYAN: &str = "\x1b[38;5;117m";
const C_GREEN: &str = "\x1b[38;5;82m";
const C_YELLOW: &str = "\x1b[38;5;220m";
const C_GRAY: &str = "\x1b[38;5;245m";
const C_WHITE: &str = "\x1b[38;5;255m";

fn print_banner() {
    println!("{C_BOLD}{C_WHITE}  ██████╗  █████╗ ██████╗ ██╗   ██╗██████╗ ██╗   ██╗███████╗████████╗{C_RESET}");
    println!("{C_BOLD}{C_CYAN} ██╔════╝ ██╔══██╗██╔══██╗██║   ██║██╔══██╗██║   ██║██╔════╝╚══██╔══╝{C_RESET}");
    println!("{C_BOLD}{C_CYAN} ██║  ███╗███████║██████╔╝██║   ██║██║  ██║██║   ██║███████╗   ██║{C_RESET}   ");
    println!("{C_BOLD}{C_CYAN} ██║   ██║██╔══██║██╔══██╗██║   ██║██║  ██║██║   ██║╚════██║   ██║{C_RESET}   ");
    println!("{C_BOLD}{C_CYAN} ╚██████╔╝██║  ██║██║  ██║╚██████╔╝██████╔╝╚██████╔╝███████║   ██║{C_RESET}   ");
    println!("{C_BOLD}{C_CYAN}  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝  ╚═════╝ ╚══════╝   ╚═╝{C_RESET}   ");
    println!();
    println!("{C_BOLD}{C_WHITE}Welcome to Garudust setup!{C_RESET}");
    println!("{C_GRAY}This will create ~/.garudust/config.yaml and ~/.garudust/.env{C_RESET}");
    println!();
}

const PLATFORMS: &[(&str, &[(&str, &str)])] = &[
    ("Telegram", &[("Telegram bot token", "TELEGRAM_TOKEN")]),
    ("Discord", &[("Discord bot token", "DISCORD_TOKEN")]),
    (
        "Slack",
        &[
            ("Slack bot token (xoxb-...)", "SLACK_BOT_TOKEN"),
            ("Slack app token (xapp-...)", "SLACK_APP_TOKEN"),
        ],
    ),
    (
        "Matrix",
        &[
            ("Matrix homeserver URL", "MATRIX_HOMESERVER"),
            ("Matrix user (@bot:example.com)", "MATRIX_USER"),
            ("Matrix password", "MATRIX_PASSWORD"),
        ],
    ),
    (
        "LINE",
        &[
            ("LINE channel access token", "LINE_CHANNEL_TOKEN"),
            ("LINE channel secret", "LINE_CHANNEL_SECRET"),
        ],
    ),
    (
        "WhatsApp",
        &[
            ("WhatsApp access token", "WHATSAPP_ACCESS_TOKEN"),
            ("WhatsApp phone number ID", "WHATSAPP_PHONE_NUMBER_ID"),
            (
                "WhatsApp app secret (for signature verification)",
                "WHATSAPP_APP_SECRET",
            ),
            (
                "WhatsApp verify token (for webhook setup)",
                "WHATSAPP_VERIFY_TOKEN",
            ),
        ],
    ),
];

pub async fn run() -> anyhow::Result<()> {
    let home_dir = AgentConfig::garudust_dir();
    std::fs::create_dir_all(&home_dir)?;

    let existing = AgentConfig::load();
    let is_reconfigure = home_dir.join("config.yaml").exists();

    print_banner();
    if is_reconfigure {
        println!("{C_YELLOW}Existing configuration found.{C_RESET}");
        println!("{C_GRAY}Press Enter to keep the current value, or type a new one.{C_RESET}\n");
    } else {
        println!("{C_GRAY}Press Enter to accept the [default] value.{C_RESET}\n");
    }

    // ── Mode ──────────────────────────────────────────────────────────────────
    println!("{C_BOLD}{C_YELLOW}Setup mode:{C_RESET}");
    println!("  {C_CYAN}1){C_RESET}  Quick {C_DIM}— provider + model only{C_RESET}");
    println!(
        "  {C_CYAN}2){C_RESET}  Full  {C_DIM}— provider, model, and platform adapters{C_RESET}"
    );
    let mode = prompt("Choose mode", Some("1"));
    let full = matches!(mode.trim(), "2" | "full");
    println!();

    // ── Provider ──────────────────────────────────────────────────────────────
    let ollama_detected = std::net::TcpStream::connect("127.0.0.1:11434").is_ok();
    let ollama_hint = if ollama_detected { " ✓ detected" } else { "" };

    // Infer current selection from providers.default, then legacy field.
    let existing_provider = existing
        .providers
        .get("default")
        .and_then(|p| p.name.as_deref())
        .unwrap_or(existing.provider.as_str());
    let current_choice = match existing_provider {
        "anthropic" => "2",
        "openrouter" => "3",
        "groq" => "4",
        "deepseek" => "5",
        "gemini" => "6",
        "openai" => "7",
        _ => "1",
    };

    println!("{C_BOLD}{C_YELLOW}LLM Provider:{C_RESET}");
    println!("  {C_CYAN}1){C_RESET}  ollama      {C_DIM}— local, no API key{ollama_hint:<15}{C_RESET}  {C_GRAY}llama3.2{C_RESET}");
    println!("  {C_CYAN}2){C_RESET}  anthropic   {C_DIM}— Claude (native API)          {C_RESET}  {C_GRAY}claude-sonnet-4-6{C_RESET}");
    println!("  {C_CYAN}3){C_RESET}  openrouter  {C_DIM}— 200+ hosted models           {C_RESET}  {C_GRAY}any model{C_RESET}");
    println!("  {C_CYAN}4){C_RESET}  groq        {C_DIM}— fast inference               {C_RESET}  {C_GRAY}llama-3.3-70b-versatile{C_RESET}");
    println!("  {C_CYAN}5){C_RESET}  deepseek    {C_DIM}— DeepSeek                     {C_RESET}  {C_GRAY}deepseek-chat{C_RESET}");
    println!("  {C_CYAN}6){C_RESET}  gemini      {C_DIM}— Google Gemini                {C_RESET}  {C_GRAY}gemini-2.5-flash{C_RESET}");
    println!("  {C_CYAN}7){C_RESET}  openai      {C_DIM}— OpenAI GPT                   {C_RESET}  {C_GRAY}gpt-4o-mini{C_RESET}");
    println!("  {C_DIM}other       — type provider name  (mistral / xai / together / …){C_RESET}");
    println!("  {C_DIM}custom      — custom base URL{C_RESET}");
    let choice = prompt("Choose provider", Some(current_choice));
    println!();

    // Remove stale legacy env vars — provider/url now live in providers.default.
    for var in &["VLLM_BASE_URL", "OLLAMA_BASE_URL", "GARUDUST_MODEL"] {
        remove_env_var(&home_dir, var)?;
    }

    let mut env_vars: Vec<(String, String)> = Vec::new();

    // Collect (provider_name, api_key_env_var, custom_url) from the user's choice.
    let (provider_name, api_key_env, custom_url): (String, String, Option<String>) = match choice
        .trim()
    {
        "1" | "ollama" => {
            let cur_url = existing
                .providers
                .get("default")
                .and_then(|p| p.url.as_deref())
                .or(existing.base_url.as_deref())
                .unwrap_or("http://localhost:11434");
            let url = prompt("Base URL (Ollama server)", Some(cur_url));
            let url = if url.is_empty() {
                cur_url.to_string()
            } else {
                url
            };
            ("ollama".into(), String::new(), Some(url))
        }
        "2" | "anthropic" => {
            let cur = read_env_file(&home_dir, "ANTHROPIC_API_KEY");
            if let Some(v) =
                prompt_secret("ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY", cur.as_deref())?
            {
                env_vars.push(("ANTHROPIC_API_KEY".into(), v));
            }
            ("anthropic".into(), "ANTHROPIC_API_KEY".into(), None)
        }
        "3" | "openrouter" => {
            let cur = read_env_file(&home_dir, "OPENROUTER_API_KEY");
            if let Some(v) =
                prompt_secret("OPENROUTER_API_KEY", "OPENROUTER_API_KEY", cur.as_deref())?
            {
                env_vars.push(("OPENROUTER_API_KEY".into(), v));
            }
            ("openrouter".into(), "OPENROUTER_API_KEY".into(), None)
        }
        "4" | "groq" => {
            let cur = read_env_file(&home_dir, "GROQ_API_KEY");
            if let Some(v) = prompt_secret("GROQ_API_KEY", "GROQ_API_KEY", cur.as_deref())? {
                env_vars.push(("GROQ_API_KEY".into(), v));
            }
            ("groq".into(), "GROQ_API_KEY".into(), None)
        }
        "5" | "deepseek" => {
            let cur = read_env_file(&home_dir, "DEEPSEEK_API_KEY");
            if let Some(v) = prompt_secret("DEEPSEEK_API_KEY", "DEEPSEEK_API_KEY", cur.as_deref())?
            {
                env_vars.push(("DEEPSEEK_API_KEY".into(), v));
            }
            ("deepseek".into(), "DEEPSEEK_API_KEY".into(), None)
        }
        "6" | "gemini" => {
            let cur = read_env_file(&home_dir, "GEMINI_API_KEY");
            if let Some(v) = prompt_secret("GEMINI_API_KEY", "GEMINI_API_KEY", cur.as_deref())? {
                env_vars.push(("GEMINI_API_KEY".into(), v));
            }
            ("gemini".into(), "GEMINI_API_KEY".into(), None)
        }
        "7" | "openai" => {
            let cur = read_env_file(&home_dir, "OPENAI_API_KEY");
            if let Some(v) = prompt_secret("OPENAI_API_KEY", "OPENAI_API_KEY", cur.as_deref())? {
                env_vars.push(("OPENAI_API_KEY".into(), v));
            }
            ("openai".into(), "OPENAI_API_KEY".into(), None)
        }
        "custom" => {
            let cur_url = existing
                .providers
                .get("default")
                .and_then(|p| p.url.as_deref())
                .or(existing.base_url.as_deref());
            let url = prompt("Base URL (e.g. http://localhost:8000/v1)", cur_url);
            let url = if url.is_empty() {
                cur_url.unwrap_or("").to_string()
            } else {
                url
            };
            let key_env = prompt("API key env var (e.g. MY_API_KEY, Enter to skip)", None);
            if !key_env.is_empty() {
                let cur = read_env_file(&home_dir, &key_env);
                if let Some(v) = prompt_secret(&key_env, &key_env, cur.as_deref())? {
                    env_vars.push((key_env.clone(), v));
                }
            }
            let url = if url.is_empty() { None } else { Some(url) };
            ("custom".into(), key_env, url)
        }
        other => {
            // "other" → ask name; or user typed a provider name directly.
            let name = if other == "other" {
                prompt("Provider name (e.g. mistral, xai, together, thaillm)", None)
            } else {
                other.to_string()
            };
            let builtin: Option<&BuiltinProvider> =
                BUILTIN_PROVIDERS.iter().find(|p| p.name == name.as_str());
            let key_env = builtin
                .map(|p| p.api_key_env.to_string())
                .unwrap_or_default();
            if !key_env.is_empty() {
                let cur = read_env_file(&home_dir, &key_env);
                if let Some(v) = prompt_secret(&key_env, &key_env, cur.as_deref())? {
                    env_vars.push((key_env.clone(), v));
                }
            }
            // Providers whose default base_url is localhost need a URL prompt.
            let needs_url = builtin.is_some_and(|p| p.base_url.starts_with("http://localhost"));
            let url = if needs_url {
                let default_url = builtin.map(|p| p.base_url);
                let u = prompt("Base URL", default_url);
                if u.is_empty() {
                    default_url.map(str::to_string)
                } else {
                    Some(u)
                }
            } else {
                None
            };
            (name, key_env, url)
        }
    };
    println!();

    // ── Model ─────────────────────────────────────────────────────────────────
    let existing_model = existing
        .providers
        .get("default")
        .and_then(|p| p.model.as_deref())
        .unwrap_or(existing.model.as_str());
    let default_model = if is_reconfigure && existing_provider == provider_name.as_str() {
        existing_model
    } else {
        match provider_name.as_str() {
            "ollama" => "llama3.2",
            "anthropic" => "claude-sonnet-4-6",
            "openrouter" => "anthropic/claude-sonnet-4-6",
            "groq" => "llama-3.3-70b-versatile",
            "deepseek" => "deepseek-chat",
            "gemini" => "gemini-2.5-flash",
            "openai" => "gpt-4o-mini",
            "thaillm" => "typhoon-s-thaillm-8b-instruct",
            _ => "",
        }
    };
    let model_input = prompt(
        "Model",
        if default_model.is_empty() {
            None
        } else {
            Some(default_model)
        },
    );
    let model = if model_input.is_empty() {
        default_model.to_string()
    } else {
        model_input
    };
    println!();

    // ── Optional tools + platform adapters (Full mode) ───────────────────────
    // Tracks LINE/WhatsApp selections so the post-yaml-load step can flip
    // `enabled` on the corresponding `platforms.*` block. `None` in Quick mode
    // means "don't touch the yaml's platforms section at all".
    let mut webhook_platform_selections: Option<Vec<(&'static str, bool)>> = None;
    let mut custom_profiles: Vec<(String, ProviderProfile)> = Vec::new();
    if full {
        println!("{C_BOLD}{C_YELLOW}Optional Tools{C_RESET} {C_GRAY}(Enter to keep current / skip){C_RESET}");
        let cur_brave = read_env_file(&home_dir, "BRAVE_SEARCH_API_KEY");
        if let Some(v) = prompt_secret(
            "BRAVE_SEARCH_API_KEY",
            "Brave Search API key (web_search tool)",
            cur_brave.as_deref(),
        )? {
            env_vars.push(("BRAVE_SEARCH_API_KEY".into(), v));
        }
        println!();

        // Pre-tick platforms that already have at least one token in .env
        let preselected: Vec<bool> = PLATFORMS
            .iter()
            .map(|(_, fields)| {
                fields
                    .iter()
                    .any(|(_, var)| read_env_file(&home_dir, var).is_some())
            })
            .collect();

        println!("{C_BOLD}{C_YELLOW}Platform Adapters:{C_RESET}");
        println!("  {C_GRAY}↑↓ to move  ·  Space to select  ·  Enter to confirm{C_RESET}\n");

        let names: Vec<&str> = PLATFORMS.iter().map(|(name, _)| *name).collect();
        let selected = multi_select(&names, &preselected)?;
        println!();

        webhook_platform_selections = Some(
            PLATFORMS
                .iter()
                .zip(selected.iter())
                .map(|((name, _), sel)| (*name, *sel))
                .collect(),
        );

        for (i, (_, fields)) in PLATFORMS.iter().enumerate() {
            if !selected[i] {
                continue;
            }
            for (label, var) in *fields {
                let cur = read_env_file(&home_dir, var);
                if let Some(v) = prompt_secret(var, label, cur.as_deref())? {
                    env_vars.push(((*var).into(), v));
                }
            }
        }
        println!();

        // ── Custom Provider Profiles ──────────────────────────────────────────
        println!("{C_BOLD}{C_YELLOW}Custom Provider Profiles{C_RESET} {C_GRAY}(optional — for routing: or tool model overrides){C_RESET}");
        println!("  {C_GRAY}Leave profile name blank to skip.{C_RESET}\n");
        loop {
            let alias = prompt("Profile alias (e.g. groq-backup, local)", None);
            if alias.is_empty() {
                break;
            }
            let provider_name = prompt(
                "Provider name (e.g. groq, openai) or leave blank for custom URL",
                None,
            );
            let url_input = if provider_name.is_empty() {
                prompt("Base URL", None)
            } else {
                prompt("Base URL (Enter to use provider default)", None)
            };
            let key_env = prompt("API key env var (e.g. GROQ_API_KEY_2)", None);
            let model_input = prompt("Default model for this profile (Enter to skip)", None);

            let profile = ProviderProfile {
                name: if provider_name.is_empty() {
                    None
                } else {
                    Some(provider_name)
                },
                url: if url_input.is_empty() {
                    None
                } else {
                    Some(url_input)
                },
                key: if key_env.is_empty() {
                    None
                } else {
                    Some(format!("${{{key_env}}}"))
                },
                model: if model_input.is_empty() {
                    None
                } else {
                    Some(model_input)
                },
            };
            custom_profiles.push((alias, profile));
            println!();
        }
        println!();
    }

    // ── Persist ───────────────────────────────────────────────────────────────
    for (var, val) in &env_vars {
        AgentConfig::set_env_var(&home_dir, var, val)?;
    }

    // Preserve existing YAML settings (compression, context_window, disabled_toolsets, etc.)
    // and only overwrite the fields that setup controls.
    let yaml_path = home_dir.join("config.yaml");
    let mut new_config: AgentConfig = if yaml_path.exists() {
        let src = std::fs::read_to_string(&yaml_path).unwrap_or_default();
        serde_yaml::from_str(&src).unwrap_or_default()
    } else {
        AgentConfig::default()
    };
    new_config.home_dir = home_dir.clone();

    // Write the primary provider as providers.default (profile-based path).
    let default_profile = ProviderProfile {
        name: if provider_name == "custom" {
            None
        } else {
            Some(provider_name.clone())
        },
        url: custom_url.clone(),
        key: if api_key_env.is_empty() {
            None
        } else {
            Some(format!("${{{api_key_env}}}"))
        },
        model: if model.is_empty() {
            None
        } else {
            Some(model.clone())
        },
    };
    new_config
        .providers
        .insert("default".into(), default_profile);

    // Keep legacy fields in sync so older code paths and doctor still work.
    new_config.provider = provider_name.clone();
    new_config.model = model.clone();
    new_config.base_url = custom_url.clone();

    // Sync webhook-based platform adapters (LINE, WhatsApp) into yaml. Tokens
    // live in .env; this block controls whether the adapter starts and on which
    // port/path. Existing port/webhook_path is preserved when present.
    if let Some(selections) = webhook_platform_selections {
        for (name, is_selected) in selections {
            match name {
                "LINE" => sync_platform(
                    &mut new_config.platforms.line,
                    is_selected,
                    WebhookPlatformConfig::default_line,
                ),
                "WhatsApp" => sync_platform(
                    &mut new_config.platforms.whatsapp,
                    is_selected,
                    WebhookPlatformConfig::default_whatsapp,
                ),
                _ => {} // Telegram/Discord/Slack/Matrix don't use webhook config
            }
        }
    }

    for (alias, profile) in custom_profiles {
        new_config.providers.insert(alias, profile);
    }

    new_config.save_yaml()?;

    println!("{C_GREEN}✓{C_RESET} Wrote {C_BOLD}providers.default{C_RESET} profile to {C_GRAY}{}/.garudust/config.yaml{C_RESET}", home_dir.display());
    if !env_vars.is_empty() {
        for (var, _) in &env_vars {
            println!("{C_GREEN}✓{C_RESET} Wrote {C_BOLD}{var}{C_RESET} to {C_GRAY}~/.garudust/.env{C_RESET}");
        }
    }
    println!();

    // ── Doctor ────────────────────────────────────────────────────────────────
    let api_key = env_vars
        .iter()
        .find(|(v, _)| v == &api_key_env)
        .map(|(_, k)| k.clone())
        .or(existing.api_key);
    if let Some(key) = api_key {
        new_config.api_key = Some(key);
    }
    super::doctor::run(&new_config).await;

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Sync a webhook-based platform's yaml block with the wizard's selection.
/// - Selected + block exists → flip `enabled = true`, keep port/path.
/// - Selected + no block → create one from `make_default`.
/// - Unselected + block exists → flip `enabled = false`, keep port/path so a
///   later re-enable preserves the user's customizations.
/// - Unselected + no block → no-op.
fn sync_platform(
    slot: &mut Option<WebhookPlatformConfig>,
    is_selected: bool,
    make_default: fn() -> WebhookPlatformConfig,
) {
    match (is_selected, slot.as_mut()) {
        (true, Some(cfg)) => cfg.enabled = true,
        (true, None) => *slot = Some(make_default()),
        (false, Some(cfg)) => cfg.enabled = false,
        (false, None) => {}
    }
}

/// Validate a token value against known format rules.
/// Returns an error hint string when the value looks wrong, or `None` when OK.
fn validate_token(var: &str, val: &str) -> Option<&'static str> {
    match var {
        "ANTHROPIC_API_KEY" if !val.starts_with("sk-ant-") => {
            return Some("expected format: sk-ant-… (starts with 'sk-ant-')");
        }
        "OPENROUTER_API_KEY" if !val.starts_with("sk-or-") => {
            return Some("expected format: sk-or-… (starts with 'sk-or-')");
        }
        "TELEGRAM_TOKEN" => {
            let mut parts = val.splitn(2, ':');
            let digits_ok = parts
                .next()
                .is_some_and(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()));
            let suffix_ok = parts.next().is_some_and(|p| p.len() >= 30);
            if !digits_ok || !suffix_ok {
                return Some("expected format: 123456789:AAFxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx");
            }
        }
        "DISCORD_TOKEN" if val.split('.').count() != 3 || val.len() < 50 => {
            return Some("expected format: three Base64 segments separated by '.' (~70 chars)");
        }
        "SLACK_BOT_TOKEN" if !val.starts_with("xoxb-") => {
            return Some("expected format: xoxb-… (starts with 'xoxb-')");
        }
        "SLACK_APP_TOKEN" if !val.starts_with("xapp-") => {
            return Some("expected format: xapp-… (starts with 'xapp-')");
        }
        "MATRIX_HOMESERVER" if !val.starts_with("https://") && !val.starts_with("http://") => {
            return Some("expected format: https://matrix.example.com");
        }
        "MATRIX_USER" if !val.starts_with('@') || !val.contains(':') => {
            return Some("expected format: @username:server.com");
        }
        "LINE_CHANNEL_TOKEN" if val.len() < 20 => {
            return Some("expected: non-empty string, at least 20 characters");
        }
        "LINE_CHANNEL_SECRET" if val.len() != 32 || !val.chars().all(|c| c.is_ascii_hexdigit()) => {
            return Some("expected format: 32-character hex string");
        }
        "WHATSAPP_PHONE_NUMBER_ID" if !val.chars().all(|c| c.is_ascii_digit()) => {
            return Some("expected format: numeric ID (e.g. 123456789012345)");
        }
        _ => {}
    }
    None
}

/// Remove a key from ~/.garudust/.env if present.
fn remove_env_var(home_dir: &Path, key: &str) -> std::io::Result<()> {
    let env_path = home_dir.join(".env");
    if !env_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&env_path)?;
    let prefix = format!("{key}=");
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| !l.trim().starts_with(prefix.as_str()))
        .collect();
    std::fs::write(&env_path, filtered.join("\n") + "\n")
}

/// Read a raw value from ~/.garudust/.env without going through the OnceLock cache.
fn read_env_file(home_dir: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(home_dir.join(".env")).ok()?;
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(val) = line.trim().strip_prefix(&prefix) {
            let val = val.trim().trim_matches('"').trim_matches('\'');
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }
    None
}

/// Mask a secret: `sk-ant-api03-abcdef…xyz` → `sk-an••••wxyz`.
fn mask_secret(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 8 {
        return "••••".to_string();
    }
    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars
        .iter()
        .rev()
        .take(4)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}••••{suffix}")
}

/// Read a secret from stdin with masking — each character is displayed as `●`.
/// Backspace erases the last character. Enter confirms. Ctrl+C aborts.
fn read_secret() -> anyhow::Result<String> {
    let mut buf = String::new();
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    loop {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match code {
                KeyCode::Enter => break,
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    terminal::disable_raw_mode()?;
                    writeln!(stdout)?;
                    return Err(anyhow::anyhow!("interrupted"));
                }
                KeyCode::Backspace => {
                    if buf.pop().is_some() {
                        queue!(
                            stdout,
                            cursor::MoveLeft(1),
                            terminal::Clear(ClearType::UntilNewLine)
                        )?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    queue!(stdout, Print("●"))?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }
    terminal::disable_raw_mode()?;
    writeln!(stdout)?;
    Ok(buf)
}

/// Prompt for a potentially-sensitive value, with one-time format validation.
/// Shows `[current: ••••]` when an existing value is present.
/// Returns `None` (keep existing) if the user presses Enter with no input.
/// Returns `Some(new_value)` when the user types a new value.
/// On a format mismatch: warns inline and re-prompts once; accepts whatever the
/// user enters on the second attempt so valid non-standard tokens still work.
fn prompt_secret(var: &str, label: &str, existing: Option<&str>) -> anyhow::Result<Option<String>> {
    if let Some(cur) = existing {
        print!("  {label} [current: {}]: ", mask_secret(cur));
    } else {
        print!("  {label}: ");
    }
    io::stdout().flush()?;

    let first = read_secret()?;
    if first.is_empty() {
        return Ok(None);
    }

    if let Some(hint) = validate_token(var, &first) {
        println!("  ✗ {hint}");
        print!("  {label} (press Enter to use as-is): ");
        io::stdout().flush()?;
        let second = read_secret()?;
        return Ok(Some(if second.is_empty() { first } else { second }));
    }

    Ok(Some(first))
}

/// Prompt for a non-secret value. Shows `[default]` in brackets.
/// Returns the default if the user presses Enter with no input.
fn prompt(label: &str, default: Option<&str>) -> String {
    match default {
        Some(d) if !d.is_empty() => print!("  {label} [{d}]: "),
        _ => print!("  {label}: "),
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

/// Render an interactive checkbox list. `preselected` sets the initial state.
/// Returns a bool vec (same length as `items`) indicating which are selected.
fn multi_select(items: &[&str], preselected: &[bool]) -> anyhow::Result<Vec<bool>> {
    let mut selected = preselected.to_vec();
    selected.resize(items.len(), false);
    let mut cursor_pos: usize = 0;
    let mut stdout = io::stdout();

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;
    draw_checkboxes(&mut stdout, items, &selected, cursor_pos)?;

    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') if cursor_pos + 1 < items.len() => {
                    cursor_pos += 1;
                }
                KeyCode::Char(' ') => {
                    selected[cursor_pos] = !selected[cursor_pos];
                }
                KeyCode::Enter => break,
                KeyCode::Char('q') | KeyCode::Esc => {
                    selected.fill(false);
                    break;
                }
                _ => {}
            }
            draw_checkboxes(&mut stdout, items, &selected, cursor_pos)?;
        }
    }

    terminal::disable_raw_mode()?;
    execute!(stdout, cursor::Show)?;
    writeln!(stdout)?;

    Ok(selected)
}

fn draw_checkboxes(
    stdout: &mut io::Stdout,
    items: &[&str],
    selected: &[bool],
    cursor_pos: usize,
) -> anyhow::Result<()> {
    if items.len() > 1 {
        queue!(
            stdout,
            cursor::MoveUp(u16::try_from(items.len() - 1).unwrap_or(u16::MAX)),
            cursor::MoveToColumn(0),
        )?;
    } else {
        queue!(stdout, cursor::MoveToColumn(0))?;
    }

    for (i, item) in items.iter().enumerate() {
        let checkbox = if selected[i] { "[✓]" } else { "[ ]" };
        queue!(stdout, terminal::Clear(ClearType::CurrentLine))?;

        if i == cursor_pos {
            queue!(
                stdout,
                SetAttribute(Attribute::Bold),
                Print(format!("  {checkbox} {item}")),
                SetAttribute(Attribute::Reset),
            )?;
        } else {
            queue!(stdout, Print(format!("  {checkbox} {item}")))?;
        }

        if i + 1 < items.len() {
            queue!(stdout, Print("\r\n"))?;
        }
    }

    stdout.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_token;

    #[test]
    fn anthropic_key_valid() {
        assert!(validate_token("ANTHROPIC_API_KEY", "sk-ant-api03-abc").is_none());
    }

    #[test]
    fn anthropic_key_invalid() {
        assert!(validate_token("ANTHROPIC_API_KEY", "sk-abc-wrongprefix").is_some());
    }

    #[test]
    fn openrouter_key_valid() {
        assert!(validate_token("OPENROUTER_API_KEY", "sk-or-v1-abc123").is_none());
    }

    #[test]
    fn openrouter_key_invalid() {
        assert!(validate_token("OPENROUTER_API_KEY", "sk-ant-abc").is_some());
    }

    #[test]
    fn telegram_token_valid() {
        assert!(validate_token(
            "TELEGRAM_TOKEN",
            "123456789:AAFabcdefghijklmnopqrstuvwxyz012"
        )
        .is_none());
    }

    #[test]
    fn telegram_token_invalid_no_colon() {
        assert!(validate_token("TELEGRAM_TOKEN", "123456789AAFabc").is_some());
    }

    #[test]
    fn telegram_token_invalid_non_digit_id() {
        assert!(
            validate_token("TELEGRAM_TOKEN", "abcde:AAFabcdefghijklmnopqrstuvwxyz012").is_some()
        );
    }

    #[test]
    fn slack_bot_token_valid() {
        assert!(validate_token("SLACK_BOT_TOKEN", "xoxb-123-abc").is_none());
    }

    #[test]
    fn slack_bot_token_invalid() {
        assert!(validate_token("SLACK_BOT_TOKEN", "xoxp-123-abc").is_some());
    }

    #[test]
    fn slack_app_token_valid() {
        assert!(validate_token("SLACK_APP_TOKEN", "xapp-1-abc").is_none());
    }

    #[test]
    fn matrix_homeserver_valid() {
        assert!(validate_token("MATRIX_HOMESERVER", "https://matrix.example.com").is_none());
    }

    #[test]
    fn matrix_homeserver_invalid() {
        assert!(validate_token("MATRIX_HOMESERVER", "matrix.example.com").is_some());
    }

    #[test]
    fn matrix_user_valid() {
        assert!(validate_token("MATRIX_USER", "@bot:example.com").is_none());
    }

    #[test]
    fn matrix_user_invalid() {
        assert!(validate_token("MATRIX_USER", "bot_example_com").is_some());
    }

    #[test]
    fn line_channel_token_valid() {
        assert!(validate_token("LINE_CHANNEL_TOKEN", "a".repeat(20).as_str()).is_none());
    }

    #[test]
    fn line_channel_token_too_short() {
        assert!(validate_token("LINE_CHANNEL_TOKEN", "short").is_some());
    }

    #[test]
    fn line_channel_secret_valid() {
        assert!(
            validate_token("LINE_CHANNEL_SECRET", "abcdef1234567890abcdef1234567890").is_none()
        );
    }

    #[test]
    fn line_channel_secret_invalid_length() {
        assert!(validate_token("LINE_CHANNEL_SECRET", "tooshort").is_some());
    }

    #[test]
    fn line_channel_secret_invalid_non_hex() {
        assert!(
            validate_token("LINE_CHANNEL_SECRET", "zbcdef1234567890abcdef123456789z").is_some()
        );
    }

    #[test]
    fn unknown_var_always_passes() {
        assert!(validate_token("SOME_UNKNOWN_VAR", "anything").is_none());
    }

    #[test]
    fn discord_token_valid() {
        let token = "MTIzNDU2Nzg5.ABCDEF.ghijklmnopqrstuvwxyz1234567890abcdefghij";
        assert!(validate_token("DISCORD_TOKEN", token).is_none());
    }

    #[test]
    fn discord_token_invalid_segments() {
        assert!(validate_token("DISCORD_TOKEN", "only.two").is_some());
    }
}
