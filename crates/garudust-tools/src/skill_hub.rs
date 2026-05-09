use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

const RAW_BASE: &str = "https://raw.githubusercontent.com";

// ── Source resolution ─────────────────────────────────────────────────────────

/// Resolved download source for a skill.
enum SkillSource {
    /// Direct HTTPS URL to a SKILL.md file.
    Url(String),
    /// GitHub `owner/repo/path` — resolved to raw.githubusercontent.com.
    GitHub {
        owner: String,
        repo: String,
        path: String,
    },
    /// Well-known endpoint: fetch `{base}/.well-known/skills/{name}/SKILL.md`.
    WellKnown { base: String, name: String },
}

/// Parse a user-supplied source string into a `SkillSource`.
///
/// Accepted forms:
/// - `https://…/SKILL.md`               → direct URL
/// - `well-known:https://example.com`   → well-known endpoint
/// - `owner/repo/path/to/skill`         → GitHub (≥3 segments)
fn resolve_source(source: &str, skill_name: &str) -> Result<SkillSource> {
    if source.starts_with("well-known:") {
        let base = source
            .strip_prefix("well-known:")
            .unwrap()
            .trim_end_matches('/')
            .to_string();
        return Ok(SkillSource::WellKnown {
            base,
            name: skill_name.to_string(),
        });
    }

    if source.starts_with("https://") || source.starts_with("http://") {
        return Ok(SkillSource::Url(source.to_string()));
    }

    // GitHub: owner/repo or owner/repo/path/to/skill
    let parts: Vec<&str> = source.splitn(3, '/').collect();
    match parts.as_slice() {
        [owner, repo] => Ok(SkillSource::GitHub {
            owner: (*owner).to_string(),
            repo: (*repo).to_string(),
            path: skill_name.to_string(),
        }),
        [owner, repo, path] => Ok(SkillSource::GitHub {
            owner: (*owner).to_string(),
            repo: (*repo).to_string(),
            path: (*path).to_string(),
        }),
        _ => bail!(
            "cannot resolve source '{}'. Use: owner/repo/path, https://…/SKILL.md, or well-known:https://…",
            source
        ),
    }
}

async fn fetch_text(url: &str) -> Result<String> {
    reqwest::get(url)
        .await
        .with_context(|| format!("fetch {url}"))?
        .error_for_status()
        .with_context(|| format!("HTTP error fetching {url}"))?
        .text()
        .await
        .context("read response body")
}

// ── Install ───────────────────────────────────────────────────────────────────

/// Download and install a skill into `skills_dir/<name>/SKILL.md`.
///
/// `source` accepts:
/// - `owner/repo` or `owner/repo/path/to/skill` (GitHub)
/// - `https://example.com/SKILL.md` (direct URL)
/// - `well-known:https://example.com` (well-known endpoint)
///
/// Returns the skill name as written to disk.
pub async fn install_skill(source: &str, skill_name: &str, skills_dir: &Path) -> Result<String> {
    let resolved = resolve_source(source, skill_name)?;

    let (content, name) = match &resolved {
        SkillSource::Url(url) => {
            let text = fetch_text(url).await?;
            // Derive name from URL filename if not given
            let name = if skill_name.is_empty() {
                url.trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(skill_name)
                    .trim_end_matches(".md")
                    .to_string()
            } else {
                skill_name.to_string()
            };
            (text, name)
        }

        SkillSource::GitHub { owner, repo, path } => {
            let url = format!("{RAW_BASE}/{owner}/{repo}/main/{path}/SKILL.md");
            let text = fetch_text(&url)
                .await
                .with_context(|| format!("skill not found at {url}"))?;
            let name = path.rsplit('/').next().unwrap_or(path).to_string();
            (text, name)
        }

        SkillSource::WellKnown { base, name } => {
            let url = format!("{base}/.well-known/skills/{name}/SKILL.md");
            let text = fetch_text(&url)
                .await
                .with_context(|| format!("skill not found at {url}"))?;
            (text, name.clone())
        }
    };

    // Validate it looks like a SKILL.md
    if !content.trim_start().starts_with("---") {
        bail!("downloaded content does not look like a valid SKILL.md (missing frontmatter)");
    }

    let skill_dir = skills_dir.join(&name);
    tokio::fs::create_dir_all(&skill_dir)
        .await
        .with_context(|| format!("create {}", skill_dir.display()))?;

    let skill_path = skill_dir.join("SKILL.md");
    tokio::fs::write(&skill_path, &content)
        .await
        .with_context(|| format!("write {}", skill_path.display()))?;

    Ok(name)
}

// ── List ──────────────────────────────────────────────────────────────────────

pub struct InstalledSkill {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// List skills installed in `skills_dir` by scanning for SKILL.md files.
pub async fn list_installed(skills_dir: &Path) -> Vec<InstalledSkill> {
    let skills = crate::toolsets::skills::load_skills_from_dir(skills_dir).await;
    skills
        .into_iter()
        .map(|s| InstalledSkill {
            name: s.name,
            description: s.description,
            version: s.version.unwrap_or_else(|| "—".into()),
        })
        .collect()
}

// ── Uninstall ─────────────────────────────────────────────────────────────────

/// Remove a skill folder from `skills_dir`.
pub async fn uninstall_skill(skill_name: &str, skills_dir: &Path) -> Result<()> {
    let skill_dir = find_skill_dir(skills_dir, skill_name).await?;
    tokio::fs::remove_dir_all(&skill_dir)
        .await
        .with_context(|| format!("remove {}", skill_dir.display()))
}

/// Walk `skills_dir` recursively to find the folder whose SKILL.md has `name: <skill_name>`.
async fn find_skill_dir(skills_dir: &Path, skill_name: &str) -> Result<PathBuf> {
    let skills = crate::toolsets::skills::load_skills_from_dir(skills_dir).await;
    let skill = skills
        .into_iter()
        .find(|s| s.name == skill_name)
        .with_context(|| format!("skill '{skill_name}' is not installed"))?;

    Ok(skill.path.parent().unwrap_or(&skill.path).to_path_buf())
}
