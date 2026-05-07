use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_HUB: &str = "garudust-org/garudust-hub";

const RAW_BASE: &str = "https://raw.githubusercontent.com";

fn raw_url(repo: &str, branch: &str, path: &str) -> String {
    format!("{RAW_BASE}/{repo}/{branch}/{path}")
}

// ── Hub index ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HubIndex {
    #[serde(default)]
    pub tools: Vec<HubToolEntry>,
}

#[derive(Deserialize, Clone)]
pub struct HubToolEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    /// Files to download, relative to the tool's folder in the hub repo.
    /// Must include `tool.yaml`.
    pub files: Vec<String>,
}

impl HubToolEntry {
    /// Infer runtime requirement from the file list.
    pub fn requires(&self) -> &'static str {
        let has = |ext: &str| self.files.iter().any(|f| f.ends_with(ext));
        if has(".py") {
            "python3"
        } else if has(".js") || has("package.json") {
            "node"
        } else if has("Cargo.toml") || self.files.iter().any(|f| f.ends_with("main.rs")) {
            "rust"
        } else if has(".sh") {
            "bash"
        } else {
            "—"
        }
    }
}

pub async fn fetch_index(repo: &str) -> Result<HubIndex> {
    let url = raw_url(repo, "main", "index.yaml");
    let text = reqwest::get(&url)
        .await
        .with_context(|| format!("fetch {url}"))?
        .error_for_status()
        .with_context(|| format!("hub index not found at {url}"))?
        .text()
        .await?;

    serde_yaml::from_str(&text).context("parse hub index.yaml")
}

// ── Install ───────────────────────────────────────────────────────────────────

/// Download a tool from the hub into `tools_dir/<tool_name>/`.
/// Updates `registry.json` on success.
pub async fn install_tool(repo: &str, tool_name: &str, tools_dir: &Path) -> Result<()> {
    let index = fetch_index(repo).await?;
    let entry = index
        .tools
        .iter()
        .find(|t| t.name == tool_name)
        .with_context(|| format!("tool '{tool_name}' not found in hub {repo}"))?
        .clone();

    let install_dir = tools_dir.join(&entry.name);
    tokio::fs::create_dir_all(&install_dir)
        .await
        .with_context(|| format!("create {}", install_dir.display()))?;

    let client = reqwest::Client::new();
    for file in &entry.files {
        let hub_path = format!("tools/{}/{file}", entry.name);
        let url = raw_url(repo, "main", &hub_path);
        let bytes = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("fetch {url}"))?
            .error_for_status()
            .with_context(|| format!("file not found: {url}"))?
            .bytes()
            .await?;

        let dest = install_dir.join(file);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&dest, &bytes)
            .await
            .with_context(|| format!("write {}", dest.display()))?;

        // Make shell/python scripts executable
        #[cfg(unix)]
        {
            let ext = std::path::Path::new(file.as_str())
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            if ext.eq_ignore_ascii_case("sh") || ext.eq_ignore_ascii_case("py") {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(0o755);
                tokio::fs::set_permissions(&dest, perms).await?;
            }
        }
    }

    // Update registry
    let mut registry = read_registry(tools_dir).await;
    registry.tools.retain(|t| t.name != entry.name);
    registry.tools.push(InstalledEntry {
        name: entry.name.clone(),
        version: entry.version.clone(),
        source: format!("hub:{repo}"),
    });
    write_registry(tools_dir, &registry).await?;

    Ok(())
}

// ── Uninstall ─────────────────────────────────────────────────────────────────

pub async fn uninstall_tool(tool_name: &str, tools_dir: &Path) -> Result<()> {
    let mut registry = read_registry(tools_dir).await;
    let before = registry.tools.len();
    registry.tools.retain(|t| t.name != tool_name);

    if registry.tools.len() == before {
        bail!("tool '{tool_name}' is not installed");
    }

    let removal_dir = tools_dir.join(tool_name);
    if removal_dir.exists() {
        tokio::fs::remove_dir_all(&removal_dir)
            .await
            .with_context(|| format!("remove {}", removal_dir.display()))?;
    }

    write_registry(tools_dir, &registry).await?;
    Ok(())
}

// ── Update ────────────────────────────────────────────────────────────────────

/// Re-download a tool (or all tools) from the hub.
pub async fn update_tool(tool_name: Option<&str>, tools_dir: &Path) -> Result<Vec<String>> {
    let registry = read_registry(tools_dir).await;
    let mut updated = Vec::new();

    for entry in &registry.tools {
        if let Some(name) = tool_name {
            if entry.name != name {
                continue;
            }
        }
        let Some(repo) = entry.source.strip_prefix("hub:") else {
            continue; // local tools skip
        };
        install_tool(repo, &entry.name, tools_dir).await?;
        updated.push(entry.name.clone());
    }

    if let Some(name) = tool_name {
        if updated.is_empty() {
            bail!("tool '{name}' is not installed from a hub");
        }
    }

    Ok(updated)
}

// ── Registry ──────────────────────────────────────────────────────────────────

fn registry_path(tools_dir: &Path) -> PathBuf {
    tools_dir.join("registry.json")
}

#[derive(Serialize, Deserialize, Default)]
pub struct InstalledRegistry {
    #[serde(default)]
    pub tools: Vec<InstalledEntry>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct InstalledEntry {
    pub name: String,
    pub version: String,
    /// `"hub:<owner>/<repo>"` for hub tools, `"local"` for hand-crafted tools.
    pub source: String,
}

pub async fn read_registry(tools_dir: &Path) -> InstalledRegistry {
    let path = registry_path(tools_dir);
    let Ok(text) = tokio::fs::read_to_string(&path).await else {
        return InstalledRegistry::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

pub async fn write_registry(tools_dir: &Path, registry: &InstalledRegistry) -> Result<()> {
    let path = registry_path(tools_dir);
    let text = serde_json::to_string_pretty(registry)?;
    tokio::fs::write(&path, text)
        .await
        .with_context(|| format!("write {}", path.display()))
}

// ── List ──────────────────────────────────────────────────────────────────────

pub struct ToolStatus {
    pub name: String,
    pub installed_version: Option<String>,
    pub hub_version: Option<String>,
    pub source: Option<String>,
    pub requires: String,
    pub description: String,
}

/// List tools from both the registry and (optionally) the hub index.
pub async fn list_tools(tools_dir: &Path, fetch_hub: bool) -> Vec<ToolStatus> {
    let registry = read_registry(tools_dir).await;

    let hub_index = if fetch_hub {
        fetch_index(DEFAULT_HUB).await.ok()
    } else {
        None
    };

    let mut statuses: Vec<ToolStatus> = registry
        .tools
        .iter()
        .map(|e| {
            let hub_entry = hub_index
                .as_ref()
                .and_then(|idx| idx.tools.iter().find(|t| t.name == e.name));
            ToolStatus {
                name: e.name.clone(),
                installed_version: Some(e.version.clone()),
                hub_version: hub_entry.map(|t| t.version.clone()),
                source: Some(e.source.clone()),
                requires: hub_entry.map_or("—".into(), |t| t.requires().to_string()),
                description: hub_entry.map_or(String::new(), |t| t.description.clone()),
            }
        })
        .collect();

    // Also include hub tools not yet installed
    if let Some(idx) = &hub_index {
        for hub_tool in &idx.tools {
            if !statuses.iter().any(|s| s.name == hub_tool.name) {
                statuses.push(ToolStatus {
                    name: hub_tool.name.clone(),
                    installed_version: None,
                    hub_version: Some(hub_tool.version.clone()),
                    source: None,
                    requires: hub_tool.requires().to_string(),
                    description: hub_tool.description.clone(),
                });
            }
        }
    }

    statuses.sort_by(|a, b| a.name.cmp(&b.name));
    statuses
}
