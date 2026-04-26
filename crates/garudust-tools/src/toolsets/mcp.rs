use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use rmcp::{
    model::CallToolRequestParams,
    service::{Peer, RoleClient},
};
use serde_json::{Map, Value};

/// Wraps a single tool exposed by an external MCP server.
pub struct McpProxyTool {
    tool_name: String,
    tool_description: String,
    input_schema: Value,
    server_name: String,
    peer: Peer<RoleClient>,
}

impl McpProxyTool {
    pub fn new(
        tool_name: String,
        tool_description: String,
        input_schema: Value,
        server_name: String,
        peer: Peer<RoleClient>,
    ) -> Self {
        Self {
            tool_name,
            tool_description,
            input_schema,
            server_name,
            peer,
        }
    }
}

#[async_trait]
impl Tool for McpProxyTool {
    fn name(&self) -> &str {
        &self.tool_name
    }

    fn description(&self) -> &str {
        &self.tool_description
    }

    fn toolset(&self) -> &str {
        &self.server_name
    }

    fn schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        let arguments: Option<Map<String, Value>> = if params.is_null()
            || params.is_object() && params.as_object().is_some_and(Map::is_empty)
        {
            None
        } else {
            params.as_object().cloned()
        };

        let mut req = CallToolRequestParams::new(self.tool_name.clone());
        req.arguments = arguments;

        let result = self
            .peer
            .call_tool(req)
            .await
            .map_err(|e| ToolError::Execution(e.to_string()))?;

        let text = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        let is_error = result.is_error.unwrap_or(false);
        Ok(if is_error {
            ToolResult::err(&self.tool_name, text)
        } else {
            ToolResult::ok(&self.tool_name, text)
        })
    }
}

/// Connect to an MCP server via stdio and return discovered tools plus an opaque
/// keep-alive handle. The caller must hold the handle for the lifetime of the tools.
pub async fn connect_mcp_server(
    command: &str,
    args: &[String],
) -> anyhow::Result<(Vec<Arc<McpProxyTool>>, Box<dyn std::any::Any + Send>)> {
    use rmcp::{transport::TokioChildProcess, ServiceExt};

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    let transport = TokioChildProcess::new(cmd)?;
    let service: rmcp::service::RunningService<RoleClient, ()> = ().serve(transport).await?;
    let peer = service.peer().clone();
    let server_name = command.to_string();

    let mcp_tools: Vec<Arc<McpProxyTool>> = service
        .list_all_tools()
        .await?
        .into_iter()
        .map(|t| {
            let input_schema = Value::Object((*t.input_schema).clone());
            Arc::new(McpProxyTool::new(
                t.name.to_string(),
                t.description.as_deref().unwrap_or("").to_string(),
                input_schema,
                server_name.clone(),
                peer.clone(),
            )) as Arc<McpProxyTool>
        })
        .collect();

    Ok((mcp_tools, Box::new(service)))
}
