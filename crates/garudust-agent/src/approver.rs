use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    config::{RoleDefinition, RolesConfig},
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
    /// Wrap an existing inner approver with the tool-filtering rules from a role definition.
    /// Used when the inner approver is built externally (e.g. `InteractiveApprover`).
    pub fn with_inner(
        inner: Arc<dyn CommandApprover>,
        def: &RoleDefinition,
        registry: &garudust_tools::ToolRegistry,
    ) -> Arc<dyn CommandApprover> {
        Arc::new(RolesApprover {
            inner,
            allowed_tools: expand_allowed_tools(def, registry),
            denied_tools: def.denied_tools.iter().cloned().collect(),
        })
    }

    /// Build a per-request approver for the given (platform, user_id) pair.
    ///
    /// `roles` is the live in-memory `RolesConfig` (read from the RwLock before calling).
    /// `approval_mode` is the global fallback from `security.approval_mode`.
    ///
    /// # Fail-closed behavior (important)
    ///
    /// This function has **asymmetric behavior** depending on whether roles are configured:
    ///
    /// - **No roles at all** (`definitions`, `users`, and `default_role` all empty):
    ///   the global `approval_mode` is used unchanged — effectively open to all users.
    /// - **Any roles configured** (even just one definition with no users assigned):
    ///   users who are not explicitly assigned a role AND have no `default_role` fall through
    ///   to `DenyApprover` — they are blocked from all tools.
    ///
    /// This means adding the first role definition immediately changes the security
    /// posture: previously-open access becomes deny-by-default for unassigned users.
    /// Operators should set `default_role` to control what unassigned users can do.
    pub fn for_user(
        platform: &str,
        user_id: &str,
        username: Option<&str>,
        roles: &RolesConfig,
        approval_mode: &str,
        registry: &garudust_tools::ToolRegistry,
    ) -> Arc<dyn CommandApprover> {
        // No roles configured → use global approver unchanged.
        if roles.definitions.is_empty() && roles.users.is_empty() && roles.default_role.is_none() {
            return mode_to_approver(approval_mode);
        }

        // Roles are configured but no default_role: unknown users will be denied.
        // Warn once so operators know unassigned users are blocked.
        if roles.default_role.is_none() {
            tracing::warn!(
                "roles are configured but `default_role` is not set — \
                 users without an explicit role assignment will be denied all tools. \
                 Set `roles.default_role` to control access for unassigned users."
            );
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

        let mode = def.approval_mode.as_deref().unwrap_or(approval_mode);
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
                tracing::info!(
                    tool = tool_name,
                    user_id,
                    "roles: tool not in allowed_tools"
                );
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        config::{RoleDefinition, RolesConfig},
        error::ToolError,
        tool::{ApprovalDecision, CommandApprover, Tool, ToolContext},
        types::ToolResult,
    };
    use garudust_tools::ToolRegistry;

    use super::RolesApprover;

    // ── Minimal mock tool ────────────────────────────────────────────────────

    struct MockTool {
        name: &'static str,
        toolset: &'static str,
    }

    #[async_trait]
    #[allow(clippy::unnecessary_literal_bound)]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            self.name
        }
        fn description(&self) -> &str {
            "mock"
        }
        fn toolset(&self) -> &str {
            self.toolset
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object", "properties": {} })
        }
        async fn execute(
            &self,
            _p: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::ok("id", "ok"))
        }
    }

    fn registry_with(tools: &[(&'static str, &'static str)]) -> ToolRegistry {
        let mut r = ToolRegistry::new();
        for (name, toolset) in tools {
            r.register(MockTool { name, toolset });
        }
        r
    }

    const AUTO: &str = "auto";

    async fn decide(approver: &Arc<dyn CommandApprover>, tool: &str) -> ApprovalDecision {
        approver.approve(tool, "{}", "test_user").await
    }

    fn make(
        roles: &RolesConfig,
        reg: &ToolRegistry,
        platform: &str,
        user_id: &str,
    ) -> Arc<dyn CommandApprover> {
        RolesApprover::for_user(platform, user_id, None, roles, AUTO, reg)
    }

    // ── Tests ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn no_roles_configured_uses_global_auto() {
        let reg = registry_with(&[]);
        let approver = make(&RolesConfig::default(), &reg, "telegram", "111");
        assert_eq!(
            decide(&approver, "any_tool").await,
            ApprovalDecision::Approved
        );
    }

    #[tokio::test]
    async fn unknown_user_no_default_role_is_denied() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "admin".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "999", "admin");

        let reg = registry_with(&[]);
        // user "555" has no role, no default_role
        let approver = make(&roles, &reg, "telegram", "555");
        assert_eq!(
            decide(&approver, "any_tool").await,
            ApprovalDecision::Denied
        );
    }

    #[tokio::test]
    async fn default_role_applied_to_unknown_user() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "readonly".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                ..Default::default()
            },
        );
        roles.default_role = Some("readonly".into());

        let reg = registry_with(&[]);
        let approver = make(&roles, &reg, "telegram", "stranger");
        assert_eq!(
            decide(&approver, "read_file").await,
            ApprovalDecision::Approved
        );
    }

    #[tokio::test]
    async fn role_with_approval_mode_deny_blocks_all() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "readonly".into(),
            RoleDefinition {
                approval_mode: Some("deny".into()),
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "readonly");

        let reg = registry_with(&[]);
        let approver = make(&roles, &reg, "telegram", "111");
        assert_eq!(decide(&approver, "bash").await, ApprovalDecision::Denied);
    }

    #[tokio::test]
    async fn role_with_approval_mode_auto_approves_all() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "admin".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "admin");

        let reg = registry_with(&[]);
        let approver = make(&roles, &reg, "telegram", "111");
        assert_eq!(decide(&approver, "bash").await, ApprovalDecision::Approved);
    }

    #[tokio::test]
    async fn allowed_toolset_permits_tools_in_set() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "member".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                allowed_toolsets: vec!["web".into()],
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "member");

        let reg = registry_with(&[("search_web", "web"), ("bash", "terminal")]);
        let approver = make(&roles, &reg, "telegram", "111");

        assert_eq!(
            decide(&approver, "search_web").await,
            ApprovalDecision::Approved
        );
        assert_eq!(decide(&approver, "bash").await, ApprovalDecision::Denied);
    }

    #[tokio::test]
    async fn individual_allowed_tool_passes_without_toolset() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "member".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                allowed_tools: vec!["read_file".into()],
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "member");

        let reg = registry_with(&[("read_file", "files"), ("bash", "terminal")]);
        let approver = make(&roles, &reg, "telegram", "111");

        assert_eq!(
            decide(&approver, "read_file").await,
            ApprovalDecision::Approved
        );
        assert_eq!(decide(&approver, "bash").await, ApprovalDecision::Denied);
    }

    #[tokio::test]
    async fn denied_tools_wins_over_allowlist() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "member".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                allowed_toolsets: vec!["web".into()],
                denied_tools: vec!["search_web".into()],
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "member");

        let reg = registry_with(&[("search_web", "web")]);
        let approver = make(&roles, &reg, "telegram", "111");
        assert_eq!(
            decide(&approver, "search_web").await,
            ApprovalDecision::Denied
        );
    }

    #[tokio::test]
    async fn undefined_role_name_is_denied() {
        let mut roles = RolesConfig::default();
        // user has role "ghost" but "ghost" has no definition
        roles.set_user_role("telegram", "111", "ghost");

        let reg = registry_with(&[]);
        let approver = make(&roles, &reg, "telegram", "111");
        assert_eq!(
            decide(&approver, "any_tool").await,
            ApprovalDecision::Denied
        );
    }

    #[tokio::test]
    async fn empty_allowed_toolsets_means_unrestricted() {
        let mut roles = RolesConfig::default();
        roles.definitions.insert(
            "admin".into(),
            RoleDefinition {
                approval_mode: Some("auto".into()),
                allowed_toolsets: vec![],
                allowed_tools: vec![],
                ..Default::default()
            },
        );
        roles.set_user_role("telegram", "111", "admin");

        let reg = registry_with(&[("bash", "terminal"), ("search_web", "web")]);
        let approver = make(&roles, &reg, "telegram", "111");

        assert_eq!(decide(&approver, "bash").await, ApprovalDecision::Approved);
        assert_eq!(
            decide(&approver, "search_web").await,
            ApprovalDecision::Approved
        );
    }
}
