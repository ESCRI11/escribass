//! The MCP surface (docs/specs.md §18.2, ADR 0006 §6).
//!
//! A JSON-RPC envelope and nothing else. Every decision about what a tool call means lives in
//! [`crate::session`]; the translation from a tool name and a JSON object to one of its
//! methods lives in [`crate::call`], which is where the two byte-level rules ADR 0006 §6
//! pinned now live too. What is left here is rmcp's shapes, the advertised tool list, and the
//! one place where an advertised *schema* has to say something the descriptor does not.
//!
//! It stopped owning the dispatch in M2 PR 2. `app` is handed a tool name and a JSON object
//! exactly as this is, and a second `match` on the name would have been ADR 0006 §1's sixteen
//! copies of the contract one milestone later, in a place no test drives (ADR 0012 §1).

use crate::call::{call, Answer, CallError, ErrorKind, IMPLEMENTED, JSON_TEXT_FIELDS};
use crate::descriptor::{tool_schemas, ToolSchema};
use crate::session::Session;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities, ServerInfo,
    Tool,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData as McpError, RoleServer};
use serde_json::{json, Map, Value};
use std::borrow::Cow;
use std::sync::{Arc, Mutex};

/// One open project, served over MCP.
///
/// The `Mutex` is the whole of the concurrency design: one project, one writer (ADR 0001 §2),
/// and tool calls that are short. `ponytail:` if a call ever becomes long enough to block
/// another, the fix is to make that call not hold the lock, not to shard the project.
pub struct SongTools {
    session: Arc<Mutex<Session>>,
    tools: Vec<Tool>,
}

impl SongTools {
    pub fn new(session: Session) -> Result<Self, String> {
        Ok(Self { session: Arc::new(Mutex::new(session)), tools: tools()? })
    }
}

/// The advertised tool list, built from the descriptor and filtered to what is implemented.
fn tools() -> Result<Vec<Tool>, String> {
    let derived: Vec<ToolSchema> = tool_schemas(escribass_proto::DESCRIPTOR)?;
    let mut found = Vec::with_capacity(IMPLEMENTED.len());
    for name in IMPLEMENTED {
        let schema = derived
            .iter()
            .find(|t| t.name == *name)
            .ok_or_else(|| format!("`{name}` is implemented but not in the service"))?;
        // rmcp's model structs are `#[non_exhaustive]`: built through their constructors, so
        // a new protocol field arrives as a default rather than a compile error here.
        found.push(Tool::new(
            Cow::Owned(schema.name.clone()),
            Cow::Owned(schema.description.clone()),
            Arc::new(json_text_input(&schema.name, schema.input_schema.clone())),
        ));
    }
    Ok(found)
}

/// Rewrites the advertised type of a canonical-JSON-text field from base64 to an array.
///
/// The rule itself is [`crate::call`]'s and applies to every carrier; what is MCP's alone is
/// that MCP publishes a JSON Schema for each tool, so the exception has to be *said* here as
/// well as honoured there.
fn json_text_input(tool: &str, mut schema: Map<String, Value>) -> Map<String, Value> {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
        return schema;
    };
    for (_, field) in JSON_TEXT_FIELDS.iter().filter(|(t, _)| *t == tool) {
        if let Some(property) = properties.get_mut(*field) {
            let description = property.get("description").cloned();
            let mut rewritten = Map::from_iter([
                ("type".to_string(), json!("array")),
                ("items".to_string(), json!({"type": "object"})),
            ]);
            if let Some(text) = description {
                rewritten.insert("description".to_string(), text);
            }
            *property = Value::Object(rewritten);
        }
    }
    schema
}

/// An [`Answer`] as MCP carries it: the readable text, the same document structured, and
/// whether the session refused.
///
/// A refused call is a *tool* error, which is what lets a model self-correct — §6's three
/// retries. It is not a protocol error: the call reached us and was answered.
fn answered(answer: Answer) -> CallToolResult {
    let mut result = CallToolResult::success(vec![ContentBlock::text(answer.text)]);
    result.structured_content = Some(answer.structured);
    result.is_error = Some(answer.refused);
    result
}

/// A call that was never answered, mapped onto JSON-RPC's two.
///
/// `invalid_params` is what a caller can fix by calling differently, so it joins §6's retry
/// loop; `internal_error` is an operator's and leaves it (ADR 0006 §2).
fn unanswered(e: CallError) -> McpError {
    match e.kind {
        ErrorKind::BadRequest => McpError::invalid_params(e.message, None),
        ErrorKind::Broken => McpError::internal_error(e.message, None),
    }
}

impl ServerHandler for SongTools {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.protocol_version = ProtocolVersion::default();
        info.capabilities = ServerCapabilities::builder().enable_tools().build();
        info.server_info = Implementation::from_build_env();
        // `from_build_env` reads rmcp's own package metadata, not ours; both fields are set
        // so a client is told which server it is talking to rather than which SDK.
        info.server_info.name = "escribass".to_string();
        info.server_info.version = env!("CARGO_PKG_VERSION").to_string();
        info.instructions = Some(
            "Edit a song through validated tools. Every mutating tool takes `dry_run`: call it \
             first, read the returned RFC 6902 patch, then call again to apply. A refused call \
             returns `valid: false` and every rule it broke, each with a stable `rule` id you \
             can act on. Never edit the project files directly."
                .to_string(),
        );
        info
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut result = ListToolsResult::default();
        result.tools = self.tools.clone();
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        // A panic in one call must not make every later call fail. Nothing in `call` can leave
        // a session half-mutated — tools and `prepare` are pure, and `record` swaps state in
        // only after the write succeeded — so the guard is recovered rather than propagated.
        let mut session = self.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let arguments = request.arguments.clone().unwrap_or_default();
        let answer =
            call(&mut session, request.name.as_ref(), &arguments).map_err(unanswered)?;
        Ok(CallToolResponse::Complete(answered(answer)))
    }
}
