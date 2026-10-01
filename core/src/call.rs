//! One dispatch from a tool name to a session call, and no carrier's types in sight
//! (ADR 0012 §1).
//!
//! Until M2 there were two ways in and only one of them needed this file. gRPC is generated
//! from the descriptor and calls [`Session`] method by method; MCP is handed a name and a JSON
//! object, so it owned a hand-written `match` on the name. `app` is handed a name and a JSON
//! object too, and a third carrier can either write that `match` again — which is ADR 0006
//! §1's sixteen copies of the contract, one milestone later — or call the one that exists.
//! It calls this one. The MCP server is the JSON-RPC envelope around it and the Tauri command
//! is the second envelope; neither decides anything.
//!
//! Everything about what a tool call *means* still lives in [`crate::session`]. What lives
//! here is the translation from a name and a JSON object to one of its methods, and the two
//! things in that translation which are not translation at all, because both look correct and
//! are wrong at the byte level (ADR 0006 §6):
//!
//! 1. **`patch` is `bytes` in the proto**, and proto3 JSON encodes bytes as base64. Left
//!    alone, a caller would be asked to base64-encode an RFC 6902 array to call `apply_patch`,
//!    and would be handed base64 back. The patch crosses this boundary as a JSON array, in
//!    both directions, and [`JSON_TEXT_FIELDS`] is the one place that exception lives.
//! 2. **A `Song` is rendered with [`crate::to_canonical_json`]**, never `serde_json::to_value`,
//!    whose `Map` is a `BTreeMap` and would silently alphabetise the model's field order.
//!
//! Both rules were MCP's when only MCP needed them. They are the whole reason ADR 0012 §1 put
//! the UI on this path rather than a fresh one: a frontend that got either wrong would be
//! reading a different document from the one on disk, and both are already someone else's
//! test.

use crate::session::Session;
use crate::{to_canonical_json, ProjectError};
use escribass_proto::tools::{
    AddAssetRequest, AddAutomationRequest, AddClipRequest, AddEffectRequest, AddSectionRequest,
    AddTrackRequest, ApplyPatchRequest, CompileGeneratorRequest, CreateBranchRequest,
    DefineGeneratorRequest, DeleteBranchRequest, GetSongAtRequest, MergeBranchRequest,
    MoveSectionRequest, PreviewResponse, QuantizeRequest, RedoRequest, RenderExportRequest,
    RenderPreviewRequest, RenderResponse, SetNotesRequest, SetParamRequest, SetTempoRequest,
    SetTrackInstrumentRequest, SwitchBranchRequest, ToolResult, TransposeRequest, UndoRequest,
};
use serde_json::{json, Map, Value};

/// The tools this dispatch implements, in the order a carrier should advertise them.
///
/// Every RPC the service declares, in the order MCP advertises them. It is a list rather than
/// the descriptor's own order because the order a model sees should be ours to choose, and
/// because a tool that is declared but not wired up must never appear — a call that can only
/// fail spends a model's turn. A test keeps it equal to what [`call`] dispatches.
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
    "add_asset",
    "render_export",
    "render_preview",
    "set_tempo",
    "add_section",
    "move_section",
    "define_generator",
    "compile_generator",
    "undo",
    "redo",
    "create_branch",
    "switch_branch",
    "delete_branch",
    "merge_branch",
];

/// The tools the **model** is offered (ADR 0022 §1; ADR 0026 §3) — fourteen of
/// [`IMPLEMENTED`]'s twenty-seven, in its order, and data rather than a list written a second
/// time in Python (M3 trap 11).
///
/// Every one of them produces operations on the document and nothing else, and every one is
/// executed against the proposal (ADR 0019 §1). Thirteen are withheld, each for a reason
/// ADR 0022 §1 states: `get_song`, because a full document beside a summary is not trusted as
/// the summary (ADR 0018 §2); `get_song_at` and `get_history`, reads of the log a model acting
/// on the document has no use for; `set_param`, by the user's decision (question 7);
/// `add_asset`, which takes bytes the model does not have; `render_export`, which writes a file
/// at a path the model chose, and `render_preview`, which starts an engine it cannot hear
/// (CLAUDE.md #6); `undo` and `redo`, which reverse a person's approved change; and the four
/// branch tools, which move the session's `HEAD` under the window.
///
/// **`define_generator` and `compile_generator` are the thirteenth and fourteenth**, from
/// M4 PR 6 (ADR 0026 §3). A compile is the one call in this list whose refusal is a diagnostic
/// the model can act on — the child's own `line:column` and text, fed back whole and counted
/// against the three a turn allows — so withholding it would have kept it from the author most
/// likely to need it. A compile on a proposal spawns the sandbox and writes the **fork's**
/// clip; it writes no file and plays no sound, which is why the fork inherits the sandbox
/// where it does not inherit the engine (`Proposal::forked`).
///
/// **`apply_patch` is offered**, and the spike is why: no typed tool sets a track's mix,
/// deletes, renames, or resizes a clip, and three models reached for it unprompted within nine
/// instructions. Its two hazards are closed elsewhere — a caller-written provenance is
/// overwritten by core (ADR 0021 §1) and a version claim is disputed (ADR 0005 §3).
///
/// This is a statement about what is *offered* to one author. What is **accepted** is the same
/// for every caller, and no rule of the validator's reads the author (M3 trap 18).
pub const OFFERED: &[&str] = &[
    "apply_patch",
    "add_track",
    "set_track_instrument",
    "add_effect",
    "add_clip",
    "set_notes",
    "transpose",
    "quantize",
    "add_automation",
    "set_tempo",
    "add_section",
    "move_section",
    "define_generator",
    "compile_generator",
];

/// Fields that carry canonical JSON *text* in a `bytes` field, and so cross a JSON carrier as
/// JSON rather than as base64 (ADR 0006 §6).
///
/// Deliberately a short explicit list rather than a heuristic: `Instrument.state`,
/// `Effect.state` and `AddAssetRequest.content` are also `bytes` and *are* opaque binary, so
/// a rule like "every bytes field is really JSON" would corrupt them. An asset crosses as the
/// base64 proto3 JSON gives every `bytes` field, decoded by the generated deserializer like
/// any other argument. `core/tests/mcp.rs` checks each entry names a real `bytes` field, so
/// the list cannot rot silently.
pub const JSON_TEXT_FIELDS: &[(&str, &str)] = &[("apply_patch", "patch")];

/// One answered tool call, in the shape every carrier needs and none of their types.
///
/// Two renderings of one answer, because they are not the same bytes and the difference is
/// the point. `text` is what a reader reads: for a song it is [`crate::to_canonical_json`],
/// so the field order is the model's own (ADR 0002 §4). `structured` is the same *document*
/// as a `Value`, whose map sorts keys — which is all ADR 0002 §11 requires of a wire form,
/// and what a generated deserializer on the other side wants.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub text: String,
    pub structured: Value,
    /// A call the session refused. Not a transport failure: the call arrived and was answered,
    /// and what came back is something the caller can act on (ADR 0006 §2).
    pub refused: bool,
    /// The session's own answer, for the one carrier that needs the message rather than its
    /// JSON: `assistant.proto` carries a `ToolResult` **by value** on every `CallResult`, whole
    /// and with every violation, because "a model fixing one problem at a time wastes them"
    /// (ADR 0022 §3). Rebuilding one from `structured` would be parsing our own output, and
    /// `patch` would have to be re-serialised to the byte on the way.
    ///
    /// `None` for the answers that are not a `ToolResult`: the three reads, `add_asset`,
    /// `render_export` and `render_preview` — none of which the model is offered (`OFFERED`).
    pub result: Option<ToolResult>,
}

impl Answer {
    fn of(structured: Value, refused: bool) -> Self {
        let text = serde_json::to_string_pretty(&structured).expect("a Value serialises");
        Answer { text, structured, refused, result: None }
    }
}

/// A call that was not answered, and the distinction ADR 0006 §2 draws between the two ways.
#[derive(Debug)]
pub struct CallError {
    pub kind: ErrorKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The caller named a tool that does not exist, or arguments the tool cannot read.
    /// Calling differently would work, so this joins §6's retry loop.
    BadRequest,
    /// An operator error. Nothing the caller can say differently would help, so it leaves the
    /// retry loop rather than joining it.
    Broken,
}

impl std::fmt::Display for CallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CallError {}

fn bad(message: impl Into<String>) -> CallError {
    CallError { kind: ErrorKind::BadRequest, message: message.into() }
}

/// An operator error, formatted as every carrier has always shown it.
fn broken(e: ProjectError) -> CallError {
    CallError {
        kind: ErrorKind::Broken,
        message: format!("{} [{}]: {}", e.path, e.rule, e.message),
    }
}

fn decode<T: serde::de::DeserializeOwned>(
    tool: &str,
    arguments: &Map<String, Value>,
) -> Result<T, CallError> {
    serde_json::from_value(Value::Object(arguments.clone()))
        .map_err(|e| bad(format!("`{tool}`: {e}")))
}

/// Answers one tool call.
///
/// The session is `&mut` because half these tools write; a carrier that serves more than one
/// caller holds it behind whatever lock it already has.
pub fn call(
    session: &mut Session,
    name: &str,
    arguments: &Map<String, Value>,
) -> Result<Answer, CallError> {
    Ok(match name {
        "get_song" => {
            let song = session.get_song().song.expect("a session always has a song");
            song_answer(&song)?
        }
        "get_song_at" => {
            let request: GetSongAtRequest = decode("get_song_at", arguments)?;
            let song = session.get_song_at(&request).map_err(broken)?;
            song_answer(&song.song.expect("a replay yields a song"))?
        }
        "get_history" => {
            let history = session.get_history();
            Answer::of(history_json(&history), false)
        }
        "apply_patch" => {
            // Built by hand rather than through the proto deserializer: `patch` is `bytes`,
            // so that route wants base64, and the only way to reach it from the JSON array
            // a caller sends is to encode text we already hold.
            //
            // Hand-built means hand-validated. `as_bool().unwrap_or(false)` read
            // `"dry_run": "true"` as *false* and applied for real — a preview that writes,
            // which is the one failure §9's approve-then-apply flow cannot tolerate, and a
            // stringly-typed boolean is exactly what a model sends. Every other tool gets
            // this strictness from the generated deserializer; this one has to say it.
            for key in arguments.keys() {
                if key != "patch" && key != "dry_run" {
                    return Err(bad(format!("`apply_patch` has no argument `{key}`")));
                }
            }
            let text = match arguments.get("patch") {
                Some(Value::Array(_)) => {
                    serde_json::to_string_pretty(&arguments["patch"]).expect("a Value serialises")
                        + "\n"
                }
                Some(Value::String(text)) => text.clone(),
                _ => return Err(bad("`patch` is an RFC 6902 array".to_string())),
            };
            let dry_run = match arguments.get("dry_run") {
                None => false,
                Some(Value::Bool(chosen)) => *chosen,
                Some(other) => return Err(bad(format!("`dry_run` is a boolean, not {other}"))),
            };
            let request = ApplyPatchRequest { patch: text.into_bytes(), dry_run };
            tool_answer(&session.apply_patch(&request).map_err(broken)?)
        }
        "add_track" => {
            let request: AddTrackRequest = decode("add_track", arguments)?;
            tool_answer(&session.add_track(&request).map_err(broken)?)
        }
        "set_track_instrument" => {
            let request: SetTrackInstrumentRequest = decode("set_track_instrument", arguments)?;
            tool_answer(&session.set_track_instrument(&request).map_err(broken)?)
        }
        "add_effect" => {
            let request: AddEffectRequest = decode("add_effect", arguments)?;
            tool_answer(&session.add_effect(&request).map_err(broken)?)
        }
        "set_param" => {
            let request: SetParamRequest = decode("set_param", arguments)?;
            tool_answer(&session.set_param(&request).map_err(broken)?)
        }
        "add_clip" => {
            let request: AddClipRequest = decode("add_clip", arguments)?;
            tool_answer(&session.add_clip(&request).map_err(broken)?)
        }
        "set_notes" => {
            let request: SetNotesRequest = decode("set_notes", arguments)?;
            tool_answer(&session.set_notes(&request).map_err(broken)?)
        }
        "transpose" => {
            let request: TransposeRequest = decode("transpose", arguments)?;
            tool_answer(&session.transpose(&request).map_err(broken)?)
        }
        "quantize" => {
            let request: QuantizeRequest = decode("quantize", arguments)?;
            tool_answer(&session.quantize(&request).map_err(broken)?)
        }
        "add_automation" => {
            let request: AddAutomationRequest = decode("add_automation", arguments)?;
            tool_answer(&session.add_automation(&request).map_err(broken)?)
        }
        "add_asset" => {
            // Through the generated deserializer, which is what turns the base64 a caller
            // sends into bytes. Its answer is an address, not a `ToolResult`, so it is
            // shaped like a read's.
            let request: AddAssetRequest = decode("add_asset", arguments)?;
            let response = session.add_asset(&request).map_err(broken)?;
            Answer::of(json!({"asset_hash": response.asset_hash}), false)
        }
        "render_export" => {
            // Its own arm, like `add_asset`: what comes back is what the render produced,
            // not a `ToolResult` (song_tools.proto, `RenderResponse`). An engine that would
            // not run leaves through `broken`, outside §6's retry loop, because nothing a
            // caller says differently would start it (ADR 0006 §2).
            let request: RenderExportRequest = decode("render_export", arguments)?;
            let response = session.render_export(&request).map_err(broken)?;
            render_answer(&response)
        }
        "render_preview" => {
            // Its own arm, for `render_export`'s reason: what comes back is where the transport
            // is, not a `ToolResult`. An engine that will not play leaves through `broken`.
            let request: RenderPreviewRequest = decode("render_preview", arguments)?;
            let response = session.render_preview(&request).map_err(broken)?;
            preview_answer(&response)
        }
        "set_tempo" => {
            let request: SetTempoRequest = decode("set_tempo", arguments)?;
            tool_answer(&session.set_tempo(&request).map_err(broken)?)
        }
        "add_section" => {
            let request: AddSectionRequest = decode("add_section", arguments)?;
            tool_answer(&session.add_section(&request).map_err(broken)?)
        }
        "move_section" => {
            let request: MoveSectionRequest = decode("move_section", arguments)?;
            tool_answer(&session.move_section(&request).map_err(broken)?)
        }
        "define_generator" => {
            let request: DefineGeneratorRequest = decode("define_generator", arguments)?;
            tool_answer(&session.define_generator(&request).map_err(broken)?)
        }
        "compile_generator" => {
            // A `ToolResult` like any other arm, and that is the point: a compile that fails
            // for the author's reason is `valid = false` with the child's own text, so §6's
            // retry loop can see it; a child that would not start, or a toolchain that does
            // not match, leaves through `broken` and ends the turn (ADR 0026 §3).
            let request: CompileGeneratorRequest = decode("compile_generator", arguments)?;
            tool_answer(&session.compile_generator(&request).map_err(broken)?)
        }
        "undo" => {
            let request: UndoRequest = decode("undo", arguments)?;
            tool_answer(&session.undo(&request).map_err(broken)?)
        }
        "redo" => {
            let request: RedoRequest = decode("redo", arguments)?;
            tool_answer(&session.redo(&request).map_err(broken)?)
        }
        "create_branch" => {
            let request: CreateBranchRequest = decode("create_branch", arguments)?;
            tool_answer(&session.create_branch(&request).map_err(broken)?)
        }
        "switch_branch" => {
            let request: SwitchBranchRequest = decode("switch_branch", arguments)?;
            tool_answer(&session.switch_branch(&request).map_err(broken)?)
        }
        "delete_branch" => {
            let request: DeleteBranchRequest = decode("delete_branch", arguments)?;
            tool_answer(&session.delete_branch(&request).map_err(broken)?)
        }
        "merge_branch" => {
            let request: MergeBranchRequest = decode("merge_branch", arguments)?;
            tool_answer(&session.merge_branch(&request).map_err(broken)?)
        }
        unknown => return Err(bad(format!("no tool `{unknown}`; call tools/list"))),
    })
}

/// A [`ToolResult`] as a carrier carries it.
///
/// Built field by field rather than with `serde_json::to_value(&result)`, which would base64
/// the patch through the generated serde impl — a payload that parses, round-trips, and is
/// unreadable to the caller it was built for (ADR 0006 §6).
fn tool_answer(result: &ToolResult) -> Answer {
    let patch = if result.patch.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&result.patch).unwrap_or(Value::Null)
    };
    let mut answer = Answer::of(
        json!({
            "valid": result.valid,
            "errors": result.errors.iter().map(|e| json!({
                "path": e.path, "rule": e.rule, "message": e.message,
            })).collect::<Vec<_>>(),
            "patch": patch,
            "summary": result.summary,
            "entry_id": result.entry_id,
        }),
        !result.valid,
    );
    answer.result = Some(result.clone());
    answer
}

/// A [`RenderResponse`] as a carrier carries it.
///
/// The response's own three fields are built here for the same reason a `ToolResult`'s are:
/// the shape a caller reads is decided here rather than by a generated serializer. `result` is
/// **not** — it goes through the generated serializer, because `song_tools.proto` says it
/// crosses "by value rather than copied field by field — a field added to `RenderResult`
/// reaches a caller without touching this file", and copying its two fields by hand made that
/// true over gRPC and false here (M1 PR 13). That is M0.3's dropped `provenance` in a second
/// place, and three layers hid it: every scripted `render_export` is a dry run, so `result` was
/// always null; the parity harness had a hand copy of its own, so a dropped field was dropped
/// identically on both sides; and the one test reading a real render answer is behind a
/// feature whose CI job skips a `core/`-only change.
///
/// The generated serializer is the right one to reach for: it emits the same snake_case names
/// unconditionally (`preserve_proto_field_names`, `emit_fields`), and `RenderResult` has no
/// `bytes` field, so ADR 0006 §6's base64 rule has nothing to say about it. `result` is null
/// when no engine ran — a dry run or a refusal.
fn render_answer(response: &RenderResponse) -> Answer {
    Answer::of(
        json!({
            "valid": response.valid,
            "errors": response.errors.iter().map(|e| json!({
                "path": e.path, "rule": e.rule, "message": e.message,
            })).collect::<Vec<_>>(),
            "summary": response.summary,
            "result": response.result,
        }),
        !response.valid,
    )
}

/// A [`PreviewResponse`] as a carrier carries it, built as [`render_answer`] is and for the
/// same reason: the three fields of the answer are shaped here, and the engine's `event` goes
/// through the generated serializer, so a field added to `PreviewEvent` reaches a caller
/// without touching this file. `event` is null when nothing is playing and on a refusal.
fn preview_answer(response: &PreviewResponse) -> Answer {
    Answer::of(
        json!({
            "valid": response.valid,
            "errors": response.errors.iter().map(|e| json!({
                "path": e.path, "rule": e.rule, "message": e.message,
            })).collect::<Vec<_>>(),
            "summary": response.summary,
            "event": response.event,
        }),
        !response.valid,
    )
}

/// A song as an answer: the canonical text a reader reads, plus the same document structured.
fn song_answer(song: &escribass_schema::song::Song) -> Result<Answer, CallError> {
    let text = to_canonical_json(song).map_err(|e| CallError {
        kind: ErrorKind::Broken,
        message: e.to_string(),
    })?;
    let structured: Value = serde_json::from_str(&text).map_err(|e| CallError {
        kind: ErrorKind::Broken,
        message: e.to_string(),
    })?;
    Ok(Answer { text, structured, refused: false, result: None })
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
