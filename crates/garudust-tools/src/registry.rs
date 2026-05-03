use std::collections::HashMap;
use std::sync::Arc;

use garudust_core::{
    error::ToolError,
    tool::{ApprovalDecision, Tool, ToolContext},
    types::{ToolResult, ToolSchema},
};
use jsonschema::Validator;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    validators: HashMap<String, Arc<Validator>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            validators: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        let name = tool.name().to_string();
        let schema = tool.schema();
        self.insert_validated(name, Arc::new(tool), &schema);
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        let schema = tool.schema();
        self.insert_validated(name, tool, &schema);
    }

    fn insert_validated(&mut self, name: String, tool: Arc<dyn Tool>, schema: &serde_json::Value) {
        if let Ok(v) = jsonschema::validator_for(schema) {
            self.validators.insert(name.clone(), Arc::new(v));
        } else {
            tracing::warn!(tool = %name, "tool schema failed to compile — param validation skipped");
        }
        self.tools.insert(name, tool);
    }

    pub fn schemas(&self, toolsets: &[&str]) -> Vec<ToolSchema> {
        self.tools
            .values()
            .filter(|t| toolsets.is_empty() || toolsets.contains(&t.toolset()))
            .map(|t| t.to_schema())
            .collect()
    }

    pub fn all_schemas(&self) -> Vec<ToolSchema> {
        self.tools.values().map(|t| t.to_schema()).collect()
    }

    pub async fn dispatch(
        &self,
        name: &str,
        params: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| ToolError::NotFound(name.into()))?;

        // Validate params against the tool's declared JSON Schema.
        if let Some(validator) = self.validators.get(name) {
            let errors: Vec<String> = validator
                .iter_errors(&params)
                .map(|e| format!("{e}"))
                .collect();
            if !errors.is_empty() {
                return Err(ToolError::InvalidArgs(errors.join("; ")));
            }
        }

        // Property-based approval gate for destructive tools.
        if tool.is_destructive_for(&params) {
            let decision = ctx.approver.approve(name, &params.to_string()).await;
            tracing::info!(
                session_id = %ctx.session_id,
                tool       = %name,
                args       = %truncate(&params.to_string(), 500),
                approved   = %(decision != ApprovalDecision::Denied),
                "tool approval"
            );
            if decision == ApprovalDecision::Denied {
                return Err(ToolError::ApprovalDenied);
            }
        }

        tracing::info!(
            session_id = %ctx.session_id,
            tool       = %name,
            args       = %truncate(&params.to_string(), 500),
            "tool call started"
        );

        let started = tokio::time::Instant::now();
        let result = tool.execute(params, ctx).await;
        let duration_ms = started.elapsed().as_millis();

        match &result {
            Ok(r) => tracing::info!(
                session_id  = %ctx.session_id,
                tool        = %name,
                duration_ms = duration_ms,
                success     = true,
                tool_error  = r.is_error,
                "tool call completed"
            ),
            Err(e) => tracing::info!(
                session_id  = %ctx.session_id,
                tool        = %name,
                duration_ms = duration_ms,
                success     = false,
                error       = %e,
                "tool call failed"
            ),
        }

        result
    }

    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
}

/// Truncate a string to at most `max` bytes at a UTF-8 char boundary for safe logging.
fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
fn make_schema(required: &[&str], props: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": props,
        "required": required
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use garudust_core::{
        budget::IterationBudget,
        config::AgentConfig,
        error::ToolError,
        memory::MemoryStore,
        tool::{ApprovalDecision, CommandApprover, Tool, ToolContext},
        types::ToolResult,
    };

    use super::ToolRegistry;

    struct Echo;

    #[async_trait]
    impl Tool for Echo {
        #[allow(clippy::unnecessary_literal_bound)]
        fn name(&self) -> &str {
            "echo"
        }
        #[allow(clippy::unnecessary_literal_bound)]
        fn description(&self) -> &str {
            "echoes input"
        }
        #[allow(clippy::unnecessary_literal_bound)]
        fn toolset(&self) -> &str {
            "test"
        }
        fn schema(&self) -> serde_json::Value {
            serde_json::json!({ "type": "object", "properties": { "text": { "type": "string" } } })
        }
        async fn execute(
            &self,
            params: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            let text = params["text"].as_str().unwrap_or("").to_string();
            Ok(ToolResult::ok("echo_id", text))
        }
    }

    struct DenyAll;
    #[async_trait]
    impl CommandApprover for DenyAll {
        async fn approve(&self, _tool_name: &str, _params: &str) -> ApprovalDecision {
            ApprovalDecision::Denied
        }
    }

    struct NopMemory;
    #[async_trait]
    impl MemoryStore for NopMemory {
        async fn read_memory(
            &self,
        ) -> Result<garudust_core::memory::MemoryContent, garudust_core::AgentError> {
            Ok(garudust_core::memory::MemoryContent::default())
        }
        async fn write_memory(
            &self,
            _: &garudust_core::memory::MemoryContent,
        ) -> Result<(), garudust_core::AgentError> {
            Ok(())
        }
        async fn read_user_profile(&self) -> Result<String, garudust_core::AgentError> {
            Ok(String::new())
        }
        async fn write_user_profile(&self, _: &str) -> Result<(), garudust_core::AgentError> {
            Ok(())
        }
    }

    fn make_ctx() -> ToolContext {
        ToolContext {
            session_id: "s".into(),
            agent_id: "a".into(),
            iteration: 0,
            budget: Arc::new(IterationBudget::new(10)),
            memory: Arc::new(NopMemory),
            config: Arc::new(AgentConfig::default()),
            approver: Arc::new(DenyAll),
            sub_agent: None,
        }
    }

    #[test]
    fn register_and_names() {
        let mut r = ToolRegistry::new();
        r.register(Echo);
        let names = r.names();
        assert!(names.contains(&"echo"));
    }

    #[test]
    fn all_schemas_returns_schema() {
        let mut r = ToolRegistry::new();
        r.register(Echo);
        let schemas = r.all_schemas();
        assert_eq!(schemas.len(), 1);
        assert_eq!(schemas[0].name, "echo");
    }

    #[tokio::test]
    async fn dispatch_known_tool() {
        let mut r = ToolRegistry::new();
        r.register(Echo);
        let ctx = make_ctx();
        let result = r
            .dispatch("echo", serde_json::json!({ "text": "hello" }), &ctx)
            .await
            .unwrap();
        assert_eq!(result.content, "hello");
    }

    #[tokio::test]
    async fn dispatch_unknown_returns_not_found() {
        let r = ToolRegistry::new();
        let ctx = make_ctx();
        let err = r
            .dispatch("nope", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    // ── Schema validation tests ───────────────────────────────────────────────

    struct Typed;

    #[async_trait]
    impl Tool for Typed {
        fn name(&self) -> &'static str {
            "typed"
        }
        fn description(&self) -> &'static str {
            "typed tool"
        }
        fn toolset(&self) -> &'static str {
            "test"
        }
        fn schema(&self) -> serde_json::Value {
            super::make_schema(
                &["path"],
                &serde_json::json!({
                    "path":  { "type": "string" },
                    "limit": { "type": "integer" }
                }),
            )
        }
        async fn execute(
            &self,
            _p: serde_json::Value,
            _ctx: &ToolContext,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::ok("id", "ok"))
        }
    }

    #[tokio::test]
    async fn dispatch_valid_params_passes() {
        let mut r = ToolRegistry::new();
        r.register(Typed);
        let ctx = make_ctx();
        r.dispatch("typed", serde_json::json!({ "path": "/tmp/x" }), &ctx)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn dispatch_missing_required_field_fails() {
        let mut r = ToolRegistry::new();
        r.register(Typed);
        let ctx = make_ctx();
        let err = r
            .dispatch("typed", serde_json::json!({}), &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn dispatch_wrong_type_fails() {
        let mut r = ToolRegistry::new();
        r.register(Typed);
        let ctx = make_ctx();
        let err = r
            .dispatch("typed", serde_json::json!({ "path": 42 }), &ctx)
            .await
            .unwrap_err();
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }
}
