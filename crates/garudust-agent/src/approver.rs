use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    config::{AgentConfig, RoleDefinition},
    tool::{ApprovalDecision, CommandApprover},
};

/// Auto-approves every command — used in non-interactive / server mode.
pub struct AutoApprover;

#[async_trait]
impl CommandApprover for AutoApprover {
    async fn approve(&self, tool: &str, params: &str, user_id: &str) -> ApprovalDecision {
        tracing::info!(tool, params, user_id, "auto-approved tool call");
        ApprovalDecision::Approved
    }
}

/// Always denies — useful for read-only agents.
pub struct DenyApprover;

#[async_trait]
impl CommandApprover for DenyApprover {
    async fn approve(&self, _tool: &str, _params: &str, _user_id: &str) -> ApprovalDecision {
        ApprovalDecision::Denied
    }
}

/// Hermes-style approver: approves all destructive tools unconditionally.
///
/// The primary safety gate is the constitutional constraints injected into the
/// system prompt — the model is instructed to self-regulate before proposing
/// any destructive action. This approver's role is:
///
/// 1. Provide the audit-log hook (logging is done in ToolRegistry::dispatch).
/// 2. Act as the enforcement point for future policy extensions (e.g. an LLM
///    self-check or user confirmation step) without changing call sites.
///
/// Pattern-matching blocklists are intentionally absent: any string-level check
/// can be bypassed by obfuscation (variable expansion, base64, pipe chains).
/// The model's semantic understanding of the constitutional constraints is a
/// stronger and more general defence.
pub struct ConstitutionalApprover;

#[async_trait]
impl CommandApprover for ConstitutionalApprover {
    async fn approve(&self, _tool: &str, _params: &str, _user_id: &str) -> ApprovalDecision {
        ApprovalDecision::Approved
    }
}

/// Role-based approver: enforces per-user tool access policy from `config.roles`.
///
/// Resolution order:
/// 1. `denied_tools` — hard block regardless of toolset/mode
/// 2. `allowed_toolsets` + `allowed_tools` — allowlist (None = unrestricted)
/// 3. Inner approver derived from the role's `approval_mode`
pub struct RolesApprover {
    inner: Arc<dyn CommandApprover>,
    /// None = all tools allowed; Some = only these tools pass the allowlist.
    allowed_tools: Option<HashSet<String>>,
    denied_tools: HashSet<String>,
}

impl RolesApprover {
    /// Build a per-request approver for the given (platform, user_id) pair.
    /// Falls back to the global `security.approval_mode` when no roles are configured
    /// or the user has no role assignment.
    pub fn for_user(
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        config: &AgentConfig,
        registry: &garudust_tools::ToolRegistry,
    ) -> Arc<dyn CommandApprover> {
        let roles = &config.roles;

        // No roles configured → use global approver unchanged.
        if roles.definitions.is_empty() && roles.users.is_empty() && roles.default_role.is_none() {
            return mode_to_approver(&config.security.approval_mode);
        }

        let role_name = roles
            .lookup_role(platform, user_id, username)
            .or_else(|| roles.default_role.clone());

        let Some(role_name) = role_name else {
            // Unknown user, no default_role → pending, deny everything.
            return Arc::new(DenyApprover);
        };

        let Some(def) = roles.definitions.get(&role_name) else {
            tracing::warn!(role = %role_name, user_id, "roles: definition not found, defaulting to deny");
            return Arc::new(DenyApprover);
        };

        let mode = def
            .approval_mode
            .as_deref()
            .unwrap_or(&config.security.approval_mode);
        let inner = mode_to_approver(mode);
        let allowed_tools = expand_allowed_tools(def, registry);
        let denied_tools = def.denied_tools.iter().cloned().collect();

        Arc::new(RolesApprover {
            inner,
            allowed_tools,
            denied_tools,
        })
    }
}

#[async_trait]
impl CommandApprover for RolesApprover {
    async fn approve(&self, tool_name: &str, params: &str, user_id: &str) -> ApprovalDecision {
        if self.denied_tools.contains(tool_name) {
            tracing::info!(tool = tool_name, user_id, "roles: tool in denied_tools");
            return ApprovalDecision::Denied;
        }
        if let Some(allowed) = &self.allowed_tools {
            if !allowed.contains(tool_name) {
                tracing::info!(tool = tool_name, user_id, "roles: tool not in allowed_tools");
                return ApprovalDecision::Denied;
            }
        }
        self.inner.approve(tool_name, params, user_id).await
    }
}

fn mode_to_approver(mode: &str) -> Arc<dyn CommandApprover> {
    match mode {
        "auto" => Arc::new(AutoApprover),
        "deny" => Arc::new(DenyApprover),
        _ => Arc::new(ConstitutionalApprover),
    }
}

fn expand_allowed_tools(
    def: &RoleDefinition,
    registry: &garudust_tools::ToolRegistry,
) -> Option<HashSet<String>> {
    if def.allowed_toolsets.is_empty() && def.allowed_tools.is_empty() {
        return None;
    }
    let by_toolset = registry.tool_names_by_toolset();
    let mut tools: HashSet<String> = def.allowed_tools.iter().cloned().collect();
    for ts in &def.allowed_toolsets {
        if let Some(names) = by_toolset.get(ts.as_str()) {
            tools.extend(names.iter().cloned());
        }
    }
    Some(tools)
}
