use std::path::Path;

use anyhow::Result;
use garudust_core::config::AgentConfig;
use garudust_tools::hub;

pub async fn list(tools_dir: &Path, offline: bool) -> Result<()> {
    println!(
        "Fetching tool list{}...",
        if offline { "" } else { " (+ hub)" }
    );
    let statuses = hub::list_tools(tools_dir, !offline).await;

    if statuses.is_empty() {
        println!("No tools installed. Run `garudust tool install <name>` to install from the hub.");
        return Ok(());
    }

    let name_w = statuses
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let req_w = statuses
        .iter()
        .map(|s| s.requires.len())
        .max()
        .unwrap_or(7)
        .max(7);
    let ver_w = 9usize;

    println!(
        "{:<name_w$}  {:<ver_w$}  {:<ver_w$}  {:<req_w$}  DESCRIPTION",
        "NAME", "INSTALLED", "AVAILABLE", "REQUIRES"
    );
    println!("{}", "-".repeat(name_w + ver_w * 2 + req_w + 40));

    for s in &statuses {
        let installed = s.installed_version.as_deref().unwrap_or("—");
        let available = s.hub_version.as_deref().unwrap_or("—");
        let update_marker = match (&s.installed_version, &s.hub_version) {
            (Some(iv), Some(hv)) if iv != hv => "*",
            _ => "",
        };
        let available_col = if update_marker.is_empty() {
            available.to_string()
        } else {
            format!("{available}{update_marker}")
        };

        // Truncate description to keep output readable
        let desc = if s.description.chars().count() > 48 {
            format!("{}…", s.description.chars().take(47).collect::<String>())
        } else {
            s.description.clone()
        };

        println!(
            "{:<name_w$}  {:<ver_w$}  {:<ver_w$}  {:<req_w$}  {}",
            s.name, installed, available_col, s.requires, desc
        );
    }

    let upgradeable = statuses
        .iter()
        .filter(
            |s| matches!((&s.installed_version, &s.hub_version), (Some(iv), Some(hv)) if iv != hv),
        )
        .count();
    if upgradeable > 0 {
        println!("\n* update available — run `garudust tool update` to upgrade");
    }

    Ok(())
}

pub async fn install(tool_name: &str, tools_dir: &Path, hub: &str) -> Result<()> {
    println!("Installing '{tool_name}' from {hub}...");
    let requires = hub::install_tool(hub, tool_name, tools_dir).await?;
    println!("Installed '{tool_name}' successfully.");
    if requires != "—" && !hub::runtime_in_path(requires) {
        eprintln!("Warning: this tool requires '{requires}' which was not found on PATH.");
    }

    // Write default model hints from tool.yaml into config.yaml (only if not already set).
    let (model, fallback_model) = hub::read_tool_model_defaults(tools_dir, tool_name).await;
    let mut cfg = AgentConfig::load();
    if !model.is_empty() || !fallback_model.is_empty() {
        let slots = cfg.tools.entry(tool_name.to_string()).or_default();
        let mut updated = false;
        if !model.is_empty() && !slots.contains_key("model") {
            slots.insert(
                "model".to_string(),
                garudust_core::config::ProviderProfile {
                    model: Some(model.clone()),
                    ..Default::default()
                },
            );
            updated = true;
        }
        if !fallback_model.is_empty() && !slots.contains_key("fallback") {
            slots.insert(
                "fallback".to_string(),
                garudust_core::config::ProviderProfile {
                    model: Some(fallback_model.clone()),
                    ..Default::default()
                },
            );
            updated = true;
        }
        if updated && cfg.save_yaml().is_ok() {
            println!("Configured default model hints for '{tool_name}' in config.yaml.");
            println!("  Add key: and name:/url: fields to each slot as needed.");
        }
    }

    // Warn about any env_required keys not yet present in ~/.garudust/.env.
    let env_required = hub::read_tool_env_required(tools_dir, tool_name).await;
    if !env_required.is_empty() {
        let env_path = cfg.home_dir.join(".env");
        let existing: std::collections::HashSet<String> = std::fs::read_to_string(&env_path)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let l = l.trim();
                if l.is_empty() || l.starts_with('#') {
                    return None;
                }
                l.split_once('=').map(|(k, _)| k.trim().to_string())
            })
            .collect();

        let missing: Vec<&str> = env_required
            .iter()
            .filter(|k| !existing.contains(*k))
            .map(String::as_str)
            .collect();

        if missing.is_empty() {
            println!("All required API keys are already configured.");
        } else {
            println!("\nThis tool requires API keys not yet set — add them to ~/.garudust/.env:");
            for k in &missing {
                println!("  garudust config set {k} <your-key>");
            }
        }
    }

    Ok(())
}

pub async fn uninstall(tool_name: &str, tools_dir: &Path) -> Result<()> {
    hub::uninstall_tool(tool_name, tools_dir).await?;
    println!("Uninstalled '{tool_name}'.");
    Ok(())
}

pub async fn update(tool_name: Option<&str>, tools_dir: &Path) -> Result<()> {
    let updated = hub::update_tool(tool_name, tools_dir).await?;
    if updated.is_empty() {
        println!("Nothing to update.");
    } else {
        for name in &updated {
            println!("Updated '{name}'.");
        }
    }
    Ok(())
}
