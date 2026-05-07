use std::path::Path;

use anyhow::Result;
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
