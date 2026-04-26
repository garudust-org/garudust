use std::collections::HashMap;
use std::sync::Arc;

use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::{ToolResult, ToolSchema},
};

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn register_arc(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
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
        tool.execute(params, ctx).await
    }

    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(String::as_str).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
