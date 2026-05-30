use std::sync::Arc;

use async_trait::async_trait;
use garudust_core::{
    error::ToolError,
    tool::{Tool, ToolContext},
    types::ToolResult,
};
use rmcp::{
    model::{
        CallToolRequestParams, GetPromptRequestParams, PromptMessageContent, PromptMessageRole,
        ReadResourceRequestParams, ResourceContents,
    },
    service::{Peer, RoleClient},
};
use serde_json::{json, Map, Value};

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

/// The MCP primitives beyond tool calls — resource and prompt access — that we
/// surface to the model as synthetic tools.
#[derive(Clone, Copy)]
enum McpOp {
    ListResources,
    ReadResource,
    ListPrompts,
    GetPrompt,
}

/// Exposes an MCP server's resource/prompt primitives to the model as a tool.
/// One instance per (server, operation); registered only when the server
/// advertises the matching capability in its initialize response.
pub struct McpMetaTool {
    name: String,
    description: String,
    schema: Value,
    server_name: String,
    op: McpOp,
    peer: Peer<RoleClient>,
}

impl McpMetaTool {
    fn new(op: McpOp, server_name: &str, peer: Peer<RoleClient>) -> Self {
        let prefix = sanitize_ident(server_name);
        let (suffix, description, schema) = match op {
            McpOp::ListResources => (
                "list_resources",
                format!("List resources exposed by the '{server_name}' MCP server (uri, name, description). Use {prefix}_read_resource to fetch one."),
                json!({ "type": "object", "properties": {} }),
            ),
            McpOp::ReadResource => (
                "read_resource",
                format!("Read the contents of a resource from the '{server_name}' MCP server by URI."),
                json!({
                    "type": "object",
                    "properties": {
                        "uri": { "type": "string", "description": "Resource URI, as returned by list_resources" }
                    },
                    "required": ["uri"]
                }),
            ),
            McpOp::ListPrompts => (
                "list_prompts",
                format!("List prompt templates exposed by the '{server_name}' MCP server (name, description, arguments). Use {prefix}_get_prompt to render one."),
                json!({ "type": "object", "properties": {} }),
            ),
            McpOp::GetPrompt => (
                "get_prompt",
                format!("Fetch a rendered prompt template from the '{server_name}' MCP server by name."),
                json!({
                    "type": "object",
                    "properties": {
                        "name": { "type": "string", "description": "Prompt name, as returned by list_prompts" },
                        "arguments": { "type": "object", "description": "Optional template arguments (name → value)" }
                    },
                    "required": ["name"]
                }),
            ),
        };
        Self {
            name: format!("{prefix}_{suffix}"),
            description,
            schema,
            server_name: server_name.to_string(),
            op,
            peer,
        }
    }
}

#[async_trait]
impl Tool for McpMetaTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn toolset(&self) -> &str {
        &self.server_name
    }

    fn schema(&self) -> Value {
        self.schema.clone()
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<ToolResult, ToolError> {
        match self.op {
            McpOp::ListResources => {
                let resources = self
                    .peer
                    .list_all_resources()
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                if resources.is_empty() {
                    return Ok(ToolResult::ok(&self.name, "(no resources)".to_string()));
                }
                let text = resources
                    .iter()
                    .map(|r| {
                        let desc = r
                            .description
                            .as_deref()
                            .map(|d| format!(": {d}"))
                            .unwrap_or_default();
                        let mime = r
                            .mime_type
                            .as_deref()
                            .map(|m| format!(" [{m}]"))
                            .unwrap_or_default();
                        format!("{} — {}{desc}{mime}", r.uri, r.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::ok(&self.name, text))
            }

            McpOp::ReadResource => {
                let uri = params["uri"].as_str().ok_or_else(|| {
                    ToolError::InvalidArgs("uri required for read_resource".into())
                })?;
                let result = self
                    .peer
                    .read_resource(ReadResourceRequestParams::new(uri))
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                let text = result
                    .contents
                    .iter()
                    .map(|c| match c {
                        ResourceContents::TextResourceContents { text, .. } => text.clone(),
                        ResourceContents::BlobResourceContents {
                            blob, mime_type, ..
                        } => format!(
                            "[binary resource, {} base64 bytes{}]",
                            blob.len(),
                            mime_type
                                .as_deref()
                                .map(|m| format!(", {m}"))
                                .unwrap_or_default()
                        ),
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::ok(&self.name, text))
            }

            McpOp::ListPrompts => {
                let prompts = self
                    .peer
                    .list_all_prompts()
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                if prompts.is_empty() {
                    return Ok(ToolResult::ok(&self.name, "(no prompts)".to_string()));
                }
                let text = prompts
                    .iter()
                    .map(|p| {
                        let desc = p
                            .description
                            .as_deref()
                            .map(|d| format!(": {d}"))
                            .unwrap_or_default();
                        let args = p
                            .arguments
                            .as_ref()
                            .map(|a| {
                                let names = a
                                    .iter()
                                    .map(|arg| arg.name.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                format!(" (args: {names})")
                            })
                            .unwrap_or_default();
                        format!("{}{desc}{args}", p.name)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::ok(&self.name, text))
            }

            McpOp::GetPrompt => {
                let name = params["name"]
                    .as_str()
                    .ok_or_else(|| ToolError::InvalidArgs("name required for get_prompt".into()))?;
                let mut req = GetPromptRequestParams::new(name);
                if let Some(args) = params.get("arguments").and_then(Value::as_object) {
                    req = req.with_arguments(args.clone());
                }
                let result = self
                    .peer
                    .get_prompt(req)
                    .await
                    .map_err(|e| ToolError::Execution(e.to_string()))?;
                let text = result
                    .messages
                    .iter()
                    .map(|m| {
                        let role = match m.role {
                            PromptMessageRole::User => "user",
                            PromptMessageRole::Assistant => "assistant",
                        };
                        let body = match &m.content {
                            PromptMessageContent::Text { text } => text.clone(),
                            _ => "[non-text content]".to_string(),
                        };
                        format!("[{role}] {body}")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(ToolResult::ok(&self.name, text))
            }
        }
    }
}

/// Turn an arbitrary MCP server label into a tool-name-safe prefix
/// (alphanumeric / `_` / `-`). Used so resource/prompt meta-tools get stable,
/// API-valid names even when the server is launched via a path-like command.
fn sanitize_ident(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "mcp".to_string()
    } else {
        trimmed.to_string()
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
type McpConnection = (Vec<Arc<dyn Tool>>, Box<dyn std::any::Any + Send>);

/// Discover an already-initialized MCP service's primitives, wrapping each as a
/// `Tool` the agent can call. Tools become `McpProxyTool`s; resource and prompt
/// access is surfaced as `McpMetaTool`s, but only for the primitives the server
/// actually advertises in its initialize response. Shared by every transport.
async fn collect_tools(
    service: rmcp::service::RunningService<RoleClient, ()>,
    server_name: String,
) -> anyhow::Result<McpConnection> {
    let peer = service.peer().clone();

    let mut tools: Vec<Arc<dyn Tool>> = service
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
            )) as Arc<dyn Tool>)
        })
        .collect();

    // Surface resource/prompt primitives as meta-tools, gated on the server's
    // advertised capabilities so we never register a tool that always errors.
    let caps = service.peer_info().map(|info| &info.capabilities);
    let has_resources = caps.is_some_and(|c| c.resources.is_some());
    let has_prompts = caps.is_some_and(|c| c.prompts.is_some());

    if has_resources {
        tools.push(Arc::new(McpMetaTool::new(
            McpOp::ListResources,
            &server_name,
            peer.clone(),
        )));
        tools.push(Arc::new(McpMetaTool::new(
            McpOp::ReadResource,
            &server_name,
            peer.clone(),
        )));
    }
    if has_prompts {
        tools.push(Arc::new(McpMetaTool::new(
            McpOp::ListPrompts,
            &server_name,
            peer.clone(),
        )));
        tools.push(Arc::new(McpMetaTool::new(
            McpOp::GetPrompt,
            &server_name,
            peer.clone(),
        )));
    }

    Ok((tools, Box::new(service)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_ident_keeps_valid_chars() {
        assert_eq!(sanitize_ident("github"), "github");
        assert_eq!(sanitize_ident("remote-tools"), "remote-tools");
        assert_eq!(sanitize_ident("my_server_1"), "my_server_1");
    }

    #[test]
    fn sanitize_ident_replaces_path_and_special_chars() {
        // A path-like stdio command must still yield a tool-name-safe prefix.
        assert_eq!(sanitize_ident("/usr/bin/npx"), "usr_bin_npx");
        assert_eq!(sanitize_ident("npx @scope/pkg"), "npx__scope_pkg");
    }

    #[test]
    fn sanitize_ident_falls_back_when_empty() {
        assert_eq!(sanitize_ident(""), "mcp");
        assert_eq!(sanitize_ident("///"), "mcp");
        assert_eq!(sanitize_ident("___"), "mcp");
    }

    #[test]
    fn sanitize_ident_truncates_long_input() {
        let long = "a".repeat(100);
        assert_eq!(sanitize_ident(&long).len(), 48);
    }

    #[test]
    fn is_valid_mcp_tool_name_rules() {
        assert!(is_valid_mcp_tool_name("read_file"));
        assert!(is_valid_mcp_tool_name("tool-1"));
        assert!(!is_valid_mcp_tool_name(""));
        assert!(!is_valid_mcp_tool_name("has space"));
        assert!(!is_valid_mcp_tool_name("has/slash"));
        assert!(!is_valid_mcp_tool_name(&"x".repeat(129)));
    }
}
