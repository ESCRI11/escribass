//! The MCP surface (docs/specs.md §18.2, ADR 0006 §6).
//!
//! A translation layer and nothing else. Every decision about what a tool call means lives in
//! [`crate::session`]; this module turns MCP's shapes into that one's and back, so a caller
//! cannot get a different answer here than over gRPC.
//!
//! Two things here are not translation, and both are named in ADR 0006 §6 because both look
//! correct and are wrong at the byte level:
//!
//! 1. **`patch` is `bytes` in the proto**, and proto3 JSON encodes bytes as base64. Left
//!    alone, a model would be asked to base64-encode an RFC 6902 array to call `apply_patch`,
//!    and would be handed base64 back. The patch crosses this boundary as a JSON array, in
//!    both directions, and [`JSON_TEXT_FIELDS`] is the one place that exception lives.
//! 2. **A `Song` is rendered with [`crate::to_canonical_json`]**, never `serde_json::to_value`,
//!    whose `Map` is a `BTreeMap` and would silently alphabetise the model's field order.

use crate::descriptor::{tool_schemas, ToolSchema};
use crate::session::Session;
use crate::{to_canonical_json, ProjectError};
use escribass_proto::tools::{
    AddAutomationRequest, AddClipRequest, AddEffectRequest, AddSectionRequest, AddTrackRequest,
    ApplyPatchRequest, CreateBranchRequest, DeleteBranchRequest, GetSongAtRequest,
    MergeBranchRequest, MoveSectionRequest, QuantizeRequest, SetNotesRequest,
    SetParamRequest, SetTempoRequest,
    SetTrackInstrumentRequest, SwitchBranchRequest, ToolResult, TransposeRequest,
};
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

/// The tools this server implements, in the order it advertises them.
///
/// `song_tools.proto` declares twenty RPCs; the rest arrive in later steps. Advertising a tool
/// that is not wired up would spend a model's turn on a call that can only fail, so the list
/// is what works, and a test keeps it equal to what `call_tool` dispatches.
pub const IMPLEMENTED: &[&str] = &[
    "get_song",
    "get_song_at",
    "get_history",
    "apply_patch",
    "add_track",
    "set_track_instrument",
    "add_effect",
    "set_param",
    "add_clip",
    "set_notes",
    "transpose",
    "quantize",
    "add_automation",
    "set_tempo",
    "add_section",
    "move_section",
    "create_branch",
    "switch_branch",
    "delete_branch",
    "merge_branch",
];

/// Fields that carry canonical JSON *text* in a `bytes` field, and so must cross MCP as JSON
/// rather than as base64 (ADR 0006 §6).
///
/// Deliberately a short explicit list rather than a heuristic: `Instrument.state` and
/// `Effect.state` are also `bytes` and *are* opaque binary, so a rule like "every bytes field
/// is really JSON" would corrupt them. `tests/mcp.rs` checks each entry names a real `bytes`
/// field, so the list cannot rot silently.
pub const JSON_TEXT_FIELDS: &[(&str, &str)] = &[("apply_patch", "patch")];

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

/// A [`ToolResult`] as MCP carries it.
///
/// Built field by field rather than with `serde_json::to_value(&result)`, which would base64
/// the patch through the generated serde impl — a payload that parses, round-trips, and is
/// unreadable to the model it was built for (ADR 0006 §6).
fn tool_result(result: &ToolResult) -> CallToolResult {
    let patch = if result.patch.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&result.patch).unwrap_or(Value::Null)
    };
    let structured = json!({
        "valid": result.valid,
        "errors": result.errors.iter().map(|e| json!({
            "path": e.path, "rule": e.rule, "message": e.message,
        })).collect::<Vec<_>>(),
        "patch": patch,
        "summary": result.summary,
        "entry_id": result.entry_id,
    });
    complete(structured, !result.valid)
}

fn complete(structured: Value, is_error: bool) -> CallToolResult {
    let text = serde_json::to_string_pretty(&structured).expect("a Value serialises");
    let mut result = CallToolResult::success(vec![ContentBlock::text(text)]);
    result.structured_content = Some(structured);
    // A refused call is a *tool* error, which is what lets a model self-correct — §6's three
    // retries. It is not a protocol error: the call reached us and was answered.
    result.is_error = Some(is_error);
    result
}

/// A song as a result: the canonical text a model reads, plus the same document structured.
///
/// The text comes from `to_canonical_json` so the field order is the model's own (ADR 0002
/// §4). `structured_content` is a `Value`, whose map sorts keys — the same *document*, which
/// is what ADR 0002 §11 requires of a wire form, but not the same bytes. The text block is the
/// one to read.
fn song_result(song: &escribass_schema::song::Song) -> Result<CallToolResult, McpError> {
    let text = to_canonical_json(song).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let structured: Value =
        serde_json::from_str(&text).map_err(|e| McpError::internal_error(e.to_string(), None))?;
    let mut result = CallToolResult::success(vec![ContentBlock::text(text)]);
    result.structured_content = Some(structured);
    result.is_error = Some(false);
    Ok(result)
}

/// An operator error. Not a refusal: nothing the caller can say differently would help, so it
/// leaves the retry loop rather than joining it (ADR 0006 §2).
fn broken(e: ProjectError) -> McpError {
    McpError::internal_error(format!("{} [{}]: {}", e.path, e.rule, e.message), None)
}

fn arguments(request: &CallToolRequestParams) -> Map<String, Value> {
    request.arguments.clone().unwrap_or_default()
}

fn decode<T: serde::de::DeserializeOwned>(
    tool: &str,
    request: &CallToolRequestParams,
) -> Result<T, McpError> {
    serde_json::from_value(Value::Object(arguments(request)))
        .map_err(|e| McpError::invalid_params(format!("`{tool}`: {e}"), None))
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
        // A panic in one call must not make every later call fail. Nothing here can leave a
        // session half-mutated — tools and `prepare` are pure, and `record` swaps state in only
        // after the write succeeded — so the guard is recovered rather than propagated.
        let mut session = self.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = match request.name.as_ref() {
            "get_song" => {
                let song = session.get_song().song.expect("a session always has a song");
                song_result(&song)?
            }
            "get_song_at" => {
                let arguments: GetSongAtRequest = decode("get_song_at", &request)?;
                let song = session.get_song_at(&arguments).map_err(broken)?;
                song_result(&song.song.expect("a replay yields a song"))?
            }
            "get_history" => {
                let history = session.get_history();
                complete(history_json(&history), false)
            }
            "apply_patch" => {
                // Built directly rather than through the proto deserializer: `patch` is
                // `bytes`, so that route wants base64, and the only way to reach it from the
                // JSON array a model sends is to encode text we already hold. One field, one
                // conversion — a codec in between would be a codec to keep correct.
                let arguments = arguments(&request);
                let text = match arguments.get("patch") {
                    Some(Value::Array(_)) | Some(Value::Object(_)) => {
                        serde_json::to_string_pretty(&arguments["patch"]).expect("a Value serialises")
                            + "\n"
                    }
                    Some(Value::String(text)) => text.clone(),
                    _ => {
                        return Err(McpError::invalid_params(
                            "`patch` is an RFC 6902 array".to_string(),
                            None,
                        ))
                    }
                };
                let call = ApplyPatchRequest {
                    patch: text.into_bytes(),
                    dry_run: arguments.get("dry_run").and_then(Value::as_bool).unwrap_or(false),
                };
                tool_result(&session.apply_patch(&call).map_err(broken)?)
            }
            "add_track" => {
                let arguments: AddTrackRequest = decode("add_track", &request)?;
                tool_result(&session.add_track(&arguments).map_err(broken)?)
            }
            "set_track_instrument" => {
                let arguments: SetTrackInstrumentRequest =
                    decode("set_track_instrument", &request)?;
                tool_result(&session.set_track_instrument(&arguments).map_err(broken)?)
            }
            "add_effect" => {
                let arguments: AddEffectRequest = decode("add_effect", &request)?;
                tool_result(&session.add_effect(&arguments).map_err(broken)?)
            }
            "set_param" => {
                let arguments: SetParamRequest = decode("set_param", &request)?;
                tool_result(&session.set_param(&arguments).map_err(broken)?)
            }
            "add_clip" => {
                let arguments: AddClipRequest = decode("add_clip", &request)?;
                tool_result(&session.add_clip(&arguments).map_err(broken)?)
            }
            "set_notes" => {
                let arguments: SetNotesRequest = decode("set_notes", &request)?;
                tool_result(&session.set_notes(&arguments).map_err(broken)?)
            }
            "transpose" => {
                let arguments: TransposeRequest = decode("transpose", &request)?;
                tool_result(&session.transpose(&arguments).map_err(broken)?)
            }
            "quantize" => {
                let arguments: QuantizeRequest = decode("quantize", &request)?;
                tool_result(&session.quantize(&arguments).map_err(broken)?)
            }
            "add_automation" => {
                let arguments: AddAutomationRequest = decode("add_automation", &request)?;
                tool_result(&session.add_automation(&arguments).map_err(broken)?)
            }
            "set_tempo" => {
                let arguments: SetTempoRequest = decode("set_tempo", &request)?;
                tool_result(&session.set_tempo(&arguments).map_err(broken)?)
            }
            "add_section" => {
                let arguments: AddSectionRequest = decode("add_section", &request)?;
                tool_result(&session.add_section(&arguments).map_err(broken)?)
            }
            "move_section" => {
                let arguments: MoveSectionRequest = decode("move_section", &request)?;
                tool_result(&session.move_section(&arguments).map_err(broken)?)
            }
            "create_branch" => {
                let arguments: CreateBranchRequest = decode("create_branch", &request)?;
                tool_result(&session.create_branch(&arguments).map_err(broken)?)
            }
            "switch_branch" => {
                let arguments: SwitchBranchRequest = decode("switch_branch", &request)?;
                tool_result(&session.switch_branch(&arguments).map_err(broken)?)
            }
            "delete_branch" => {
                let arguments: DeleteBranchRequest = decode("delete_branch", &request)?;
                tool_result(&session.delete_branch(&arguments).map_err(broken)?)
            }
            "merge_branch" => {
                let arguments: MergeBranchRequest = decode("merge_branch", &request)?;
                tool_result(&session.merge_branch(&arguments).map_err(broken)?)
            }
            unknown => {
                return Err(McpError::invalid_params(
                    format!("no tool `{unknown}`; call tools/list"),
                    None,
                ))
            }
        };
        Ok(CallToolResponse::Complete(result))
    }
}

/// The log, in the shape it has on disk.
///
/// `crate::entry_to_json` already solves this: `ops` as the RFC 6902 array it is rather than
/// the base64 the generated impl would produce, and every field including `provenance`. Hand
/// building the object here dropped provenance, which §5 calls the audit trail — and a
/// transport that drops a field is deciding rather than translating (ADR 0006).
fn history_json(history: &escribass_proto::tools::HistoryResponse) -> Value {
    let entries: Map<String, Value> = history
        .entries
        .iter()
        .map(|(id, entry)| {
            let shape = crate::entry_to_json(entry)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null);
            (id.clone(), shape)
        })
        .collect();
    let refs = history.refs.as_ref();
    json!({
        "entries": entries,
        "head": refs.map(|r| r.head.clone()).unwrap_or_default(),
        "refs": refs.map(|r| r.refs.clone()).unwrap_or_default(),
    })
}
