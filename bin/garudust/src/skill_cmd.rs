use std::path::Path;

use anyhow::Result;
use garudust_tools::{hub, skill_hub};

pub async fn list(skills_dir: &Path, offline: bool) -> Result<()> {
    println!(
        "Fetching skill list{}...",
        if offline { "" } else { " (+ hub)" }
    );
    let statuses = hub::list_skills(skills_dir, !offline).await;

    let any_installed = statuses.iter().any(|s| s.installed_version.is_some());
    if statuses.is_empty() || !any_installed {
        println!(
            "No skills installed. Run `garudust skill install <name>` to install one.\n\
             Sources: name  |  owner/repo/path  |  https://…/SKILL.md  |  well-known:https://…"
        );
        return Ok(());
    }

    let name_w = statuses
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let ver_w = 9usize;

    println!(
        "{:<name_w$}  {:<ver_w$}  {:<ver_w$}  DESCRIPTION",
        "NAME", "INSTALLED", "AVAILABLE"
    );
    println!("{}", "-".repeat(name_w + ver_w * 2 + 18));

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

        let desc = if s.description.chars().count() > 48 {
            format!("{}…", s.description.chars().take(47).collect::<String>())
        } else {
            s.description.clone()
        };

        println!(
            "{:<name_w$}  {:<ver_w$}  {:<ver_w$}  {desc}",
            s.name, installed, available_col
        );
    }

    let upgradeable = statuses
        .iter()
        .filter(
            |s| matches!((&s.installed_version, &s.hub_version), (Some(iv), Some(hv)) if iv != hv),
        )
        .count();
    if upgradeable > 0 {
        println!("\n* update available — run `garudust skill update` to upgrade");
    }

    Ok(())
}

pub async fn install(source: &str, name: &str, hub_repo: &str, skills_dir: &Path) -> Result<()> {
    let is_short_name = !source.contains('/')
        && !source.starts_with("https://")
        && !source.starts_with("http://")
        && !source.starts_with("well-known:");

    if is_short_name {
        println!("Installing skill '{source}' from hub {hub_repo}...");
        hub::install_skill_from_hub(hub_repo, source, skills_dir).await?;
        println!("Installed skill '{source}'.");
    } else {
        println!("Installing skill from '{source}'...");
        let installed_name = skill_hub::install_skill(source, name, skills_dir).await?;
        println!("Installed skill '{installed_name}'.");
    }
    Ok(())
}

pub async fn uninstall(skill_name: &str, skills_dir: &Path) -> Result<()> {
    skill_hub::uninstall_skill(skill_name, skills_dir).await?;
    println!("Uninstalled skill '{skill_name}'.");
    Ok(())
}

pub async fn update(skill_name: Option<&str>, skills_dir: &Path) -> Result<()> {
    let updated = hub::update_skill(skill_name, skills_dir).await?;
    if updated.is_empty() {
        println!("Nothing to update.");
    } else {
        for name in &updated {
            println!("Updated '{name}'.");
        }
    }
    Ok(())
}
