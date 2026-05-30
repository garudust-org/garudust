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

fn is_valid_mcp_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// An opaque keep-alive handle. The caller must hold it for the lifetime of the
/// returned tools — dropping it tears down the MCP connection (and, for stdio,
/// kills the child process).
type McpConnection = (Vec<Arc<McpProxyTool>>, Box<dyn std::any::Any + Send>);

/// Discover an already-initialized MCP service's tools, wrapping each as an
/// `McpProxyTool`. Shared by every transport.
async fn collect_tools(
    service: rmcp::service::RunningService<RoleClient, ()>,
    server_name: String,
) -> anyhow::Result<McpConnection> {
    let peer = service.peer().clone();

    let mcp_tools: Vec<Arc<McpProxyTool>> = service
        .list_all_tools()
        .await?
        .into_iter()
        .filter_map(|t| {
            if !is_valid_mcp_tool_name(&t.name) {
                tracing::warn!(
                    name = %t.name,
                    server = %server_name,
                    "MCP: skipping tool with invalid name (must be 1-128 alphanumeric/underscore/hyphen chars)"
                );
                return None;
            }
            let input_schema = Value::Object((*t.input_schema).clone());
            Some(Arc::new(McpProxyTool::new(
                t.name.to_string(),
                t.description.as_deref().unwrap_or("").to_string(),
                input_schema,
                server_name.clone(),
                peer.clone(),
            )) as Arc<McpProxyTool>)
        })
        .collect();

    Ok((mcp_tools, Box::new(service)))
}

/// Connect to an MCP server over **stdio** by spawning `command` as a child
/// process, and return its discovered tools plus a keep-alive handle.
pub async fn connect_mcp_server(command: &str, args: &[String]) -> anyhow::Result<McpConnection> {
    use rmcp::{transport::TokioChildProcess, ServiceExt};

    let mut cmd = tokio::process::Command::new(command);
    cmd.args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit());

    let transport = TokioChildProcess::new(cmd)?;
    let service: rmcp::service::RunningService<RoleClient, ()> = ().serve(transport).await?;
    collect_tools(service, command.to_string()).await
}

/// Connect to a remote MCP server over **streamable HTTP** at `url`, and return
/// its discovered tools plus a keep-alive handle. `server_name` labels the
/// resulting tools' toolset (typically the configured server name).
pub async fn connect_mcp_http_server(
    url: &str,
    server_name: &str,
) -> anyhow::Result<McpConnection> {
    use rmcp::{transport::StreamableHttpClientTransport, ServiceExt};

    let transport = StreamableHttpClientTransport::from_uri(url);
    let service: rmcp::service::RunningService<RoleClient, ()> = ().serve(transport).await?;
    collect_tools(service, server_name.to_string()).await
}
