use std::path::Path;

use anyhow::Result;
use garudust_tools::skill_hub;

pub async fn list(skills_dir: &Path) -> Result<()> {
    let skills = skill_hub::list_installed(skills_dir).await;

    if skills.is_empty() {
        println!(
            "No skills installed. Run `garudust skill install <source>` to install one.\n\
             Sources: owner/repo/path  |  https://…/SKILL.md  |  well-known:https://…"
        );
        return Ok(());
    }

    let name_w = skills
        .iter()
        .map(|s| s.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let ver_w = 9usize;

    println!("{:<name_w$}  {:<ver_w$}  DESCRIPTION", "NAME", "VERSION");
    println!("{}", "-".repeat(name_w + ver_w + 16));

    for s in &skills {
        let desc = if s.description.chars().count() > 56 {
            format!("{}…", s.description.chars().take(55).collect::<String>())
        } else {
            s.description.clone()
        };
        println!("{:<name_w$}  {:<ver_w$}  {desc}", s.name, s.version);
    }

    Ok(())
}

pub async fn install(source: &str, name: &str, hub: &str, skills_dir: &Path) -> Result<()> {
    let is_short_name = !source.contains('/')
        && !source.starts_with("https://")
        && !source.starts_with("http://")
        && !source.starts_with("well-known:");

    if is_short_name {
        println!("Installing skill '{source}' from hub {hub}...");
        garudust_tools::hub::install_skill_from_hub(hub, source, skills_dir).await?;
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
