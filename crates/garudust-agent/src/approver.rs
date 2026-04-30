use async_trait::async_trait;
use garudust_core::tool::{ApprovalDecision, CommandApprover};

/// Auto-approves every command — used in non-interactive / server mode.
pub struct AutoApprover;

#[async_trait]
impl CommandApprover for AutoApprover {
    async fn approve(&self, _tool: &str, _params: &str) -> ApprovalDecision {
        ApprovalDecision::Approved
    }
}

/// Always denies — useful for read-only agents.
pub struct DenyApprover;

#[async_trait]
impl CommandApprover for DenyApprover {
    async fn approve(&self, _tool: &str, _params: &str) -> ApprovalDecision {
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
    async fn approve(&self, _tool: &str, _params: &str) -> ApprovalDecision {
        ApprovalDecision::Approved
    }
}
