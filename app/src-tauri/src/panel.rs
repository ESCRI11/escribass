//! The conversation, and the turn the panel is drawing (ADR 0021 §3, ADR 0019 §2, §3).
//!
//! Three things live here and nothing else does: what a turn *was*, where a conversation is
//! kept, and what the window has to be able to draw while one is running.
//!
//! **The conversation is the host's.** It is not song state — CLAUDE.md #1 is about the song —
//! so it is in neither `song.json` nor the log, and it is not inside the `.escri` either: a
//! prompt may carry text a person would not commit, and `patches/` is committed (ADR 0021 §3).
//! It lives beside the project and outside it, as one append-only **JSON Lines** file per
//! project under the application data directory Tauri names, keyed by the song's own id —
//! which survives a rename or a move of the directory. It is read back when the project is
//! opened, and it is sent **whole** with every prompt, so `ai` holds nothing between streams.
//!
//! **One line is one turn, and it is written when the turn's outcome is known.** A turn that
//! failed is settled at once; a turn that left a proposal is settled by Apply, Reject or Edit.
//! `ponytail:` a window closed on a pending proposal therefore records nothing — which is true
//! of the project as well, since nothing was applied, so the log names no prompt the file does
//! not have. The upgrade path, if a person ever wants to come back to a pending turn, is a
//! line written at the turn's end and a second recording the outcome; that is a file format
//! with two line kinds, and today there is one.
//!
//! **What a line carries is what the next prompt needs, plus what became of the turn.** The
//! wire `Turn` is rebuilt from the record by [`Recorded::wire`] — for a turn that has just
//! happened as much as for one read from the file — so what a restarted window sends the model
//! is what an open one sends, by construction rather than by two shapes being kept in step.
//! The per-call `patch` and `entry_id` are deliberately **not** kept: `ai` rebuilds a past
//! turn's tool message from `valid`, `summary` and the violations (`turn.py`, `answered`), and
//! a field nothing reads is a field that rots. Arguments are kept **verbatim**, as the text
//! the model wrote, because that is what goes back to the provider (assistant.proto).

use escribass_core::Session;

use escribass_core::Turn;
use escribass_proto::assistant::{CompletedCall, ToolCall, Turn as WireTurn};
use escribass_proto::tools::{ToolResult, Violation as WireViolation};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

/// What became of a turn. The panel's record, never the model's: a field for it on
/// `assistant.proto`'s `Turn` would put the window's state on the model's wire (ADR 0021 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Applied as one entry under `proposal` (ADR 0019 §2).
    Applied(String),
    /// Dropped. Nothing was written and nothing is undone (ADR 0019 §3).
    Rejected,
    /// The person's own patch, applied as theirs, as this entry (ADR 0019 §3).
    Edited(String),
    /// The turn never got as far as a proposal: `<rule>: <message>`, as the host saw it.
    Failed(String),
}

impl Outcome {
    fn to_json(&self) -> Value {
        match self {
            Outcome::Applied(entry) => json!({ "applied": entry }),
            Outcome::Rejected => json!({ "rejected": true }),
            Outcome::Edited(entry) => json!({ "edited": entry }),
            Outcome::Failed(why) => json!({ "failed": why }),
        }
    }

    fn from_json(value: &Value) -> Outcome {
        if let Some(entry) = value.get("applied").and_then(Value::as_str) {
            Outcome::Applied(entry.to_string())
        } else if let Some(entry) = value.get("edited").and_then(Value::as_str) {
            Outcome::Edited(entry.to_string())
        } else if let Some(why) = value.get("failed").and_then(Value::as_str) {
            Outcome::Failed(why.to_string())
        } else {
            Outcome::Rejected
        }
    }
}

/// One call the model made, and what it was answered — the half of a `ToolResult` that is
/// re-sent (see the module note).
#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub call_id: String,
    pub name: String,
    /// The arguments as the model wrote them, verbatim (assistant.proto, `ToolCall.args_json`).
    pub args: String,
    /// The model the response that made this call named (ADR 0021 §2).
    pub model_id: String,
    pub valid: bool,
    pub summary: String,
    pub errors: Vec<Refusal>,
}

/// One rule a call broke, as the file keeps it and as the window already reads one.
///
/// Its own three strings rather than `core::Violation`, whose `rule` is `&'static str`: a rule
/// read back out of a file is not static, and leaking one per line read to make it so would be
/// a memory leak bought to save a struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub path: String,
    pub rule: String,
    pub message: String,
}

/// One turn, as the conversation file records it (ADR 0021 §3).
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub prompt_id: String,
    pub prompt: String,
    /// When the prompt was sent, RFC 3339 — the rendering `provenance.created_at` gets in the
    /// log, from the same `Timestamp` type.
    pub at: Value,
    pub provider: String,
    /// The model that **answered**, never the one that was asked for (ADR 0021 §2). Empty
    /// when nothing did.
    pub model_id: String,
    pub calls: Vec<Call>,
    pub reply: String,
    pub outcome: Outcome,
}

impl Recorded {
    /// The turn as the wire carries it back to `ai` (ADR 0020 §3).
    ///
    /// The one function that builds a `Turn` for the conversation, whichever way the record
    /// arrived — so a conversation read from the file and one held since the window opened are
    /// the same bytes on the wire, rather than two shapes somebody has to keep in step.
    pub fn wire(&self) -> WireTurn {
        WireTurn {
            prompt: self.prompt.clone(),
            calls: self
                .calls
                .iter()
                .map(|call| CompletedCall {
                    call: Some(ToolCall {
                        call_id: call.call_id.clone(),
                        name: call.name.clone(),
                        args_json: call.args.clone(),
                        model_id: call.model_id.clone(),
                    }),
                    result: Some(ToolResult {
                        valid: call.valid,
                        errors: call
                            .errors
                            .iter()
                            .map(|violation| WireViolation {
                                path: violation.path.clone(),
                                rule: violation.rule.clone(),
                                message: violation.message.clone(),
                            })
                            .collect(),
                        patch: Vec::new(),
                        summary: call.summary.clone(),
                        entry_id: String::new(),
                    }),
                })
                .collect(),
            reply: self.reply.clone(),
        }
    }

    /// One line of the file, and what the panel draws a past turn from.
    pub fn to_json(&self) -> Value {
        json!({
            "prompt_id": self.prompt_id,
            "prompt": self.prompt,
            "at": self.at,
            "provider": self.provider,
            "model_id": self.model_id,
            "calls": self.calls.iter().map(Call::to_json).collect::<Vec<_>>(),
            "reply": self.reply,
            "outcome": self.outcome.to_json(),
        })
    }

    fn from_json(value: &Value) -> Option<Recorded> {
        let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        Some(Recorded {
            prompt_id: text("prompt_id"),
            prompt: text("prompt"),
            at: value.get("at").cloned().unwrap_or(Value::Null),
            provider: text("provider"),
            model_id: text("model_id"),
            calls: value.get("calls")?.as_array()?.iter().filter_map(Call::from_json).collect(),
            reply: text("reply"),
            outcome: Outcome::from_json(value.get("outcome").unwrap_or(&Value::Null)),
        })
    }
}

impl Call {
    fn of(done: &CompletedCall) -> Call {
        let call = done.call.clone().unwrap_or_default();
        let result = done.result.clone().unwrap_or_default();
        Call {
            call_id: call.call_id,
            name: call.name,
            args: call.args_json,
            model_id: call.model_id,
            valid: result.valid,
            summary: result.summary,
            errors: result
                .errors
                .into_iter()
                .map(|violation| Refusal {
                    path: violation.path,
                    rule: violation.rule,
                    message: violation.message,
                })
                .collect(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "call_id": self.call_id,
            "name": self.name,
            "args": self.args,
            "model_id": self.model_id,
            "valid": self.valid,
            "summary": self.summary,
            "errors": self.errors.iter().map(|violation| json!({
                "path": violation.path,
                "rule": violation.rule,
                "message": violation.message,
            })).collect::<Vec<_>>(),
        })
    }

    fn from_json(value: &Value) -> Option<Call> {
        let text = |key: &str| value.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        Some(Call {
            call_id: text("call_id"),
            name: text("name"),
            args: text("args"),
            model_id: text("model_id"),
            valid: value.get("valid").and_then(Value::as_bool).unwrap_or(false),
            summary: text("summary"),
            errors: value
                .get("errors")
                .and_then(Value::as_array)
                .map(|errors| {
                    let said = |violation: &Value, key: &str| {
                        violation.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
                    };
                    errors
                        .iter()
                        .map(|violation| Refusal {
                            path: said(violation, "path"),
                            rule: said(violation, "rule"),
                            message: said(violation, "message"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
    }
}

/// The conversation this window is holding, and the file it is kept in.
#[derive(Debug, Default)]
pub struct Conversation {
    /// `None` when the application data directory could not be named or made: the window still
    /// prompts, and what it loses is the record between sessions. A conversation that refused
    /// to open a project would be the panel deciding whether the song can be edited.
    at: Option<PathBuf>,
    turns: Vec<Recorded>,
}

impl Conversation {
    /// Reads the conversation for one song back, if there is one (ADR 0021 §3).
    ///
    /// `ponytail:` a line that will not parse is reported on stderr and skipped, rather than
    /// refusing to open the project or truncating the file. The file is append-only and this
    /// is the only reader; a half-written last line is what a machine that lost power leaves,
    /// and losing that turn is the whole cost.
    pub fn read(data_dir: Option<PathBuf>, song_id: &str) -> Conversation {
        let Some(directory) = data_dir.map(|dir| dir.join("conversations")) else {
            eprintln!(
                "escribass-app: no application data directory, so this conversation is this \
                 window's only (ADR 0021 §3)"
            );
            return Conversation::default();
        };
        if let Err(e) = std::fs::create_dir_all(&directory) {
            eprintln!("escribass-app: cannot keep conversations in {}: {e}", directory.display());
            return Conversation::default();
        }
        let at = directory.join(format!("{song_id}.jsonl"));
        let mut turns = Vec::new();
        match std::fs::read_to_string(&at) {
            Ok(text) => {
                for (number, line) in text.lines().enumerate() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<Value>(line).ok().as_ref().and_then(Recorded::from_json) {
                        Some(turn) => turns.push(turn),
                        None => eprintln!(
                            "escribass-app: {}:{} is not a turn and was skipped",
                            at.display(),
                            number + 1
                        ),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => eprintln!("escribass-app: cannot read {}: {e}", at.display()),
        }
        Conversation { at: Some(at), turns }
    }

    pub fn turns(&self) -> &[Recorded] {
        &self.turns
    }

    /// Every turn as the wire carries it, oldest first — what a prompt sends whole.
    pub fn wire(&self) -> Vec<WireTurn> {
        self.turns.iter().map(Recorded::wire).collect()
    }

    /// Appends one settled turn, to the file and to what this window holds.
    pub fn append(&mut self, turn: Recorded) {
        if let Some(at) = &self.at {
            use std::io::Write;
            let line = format!("{}\n", turn.to_json());
            let written = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(at)
                .and_then(|mut file| file.write_all(line.as_bytes()));
            if let Err(e) = written {
                eprintln!("escribass-app: cannot record the turn in {}: {e}", at.display());
            }
        }
        self.turns.push(turn);
    }

    /// Where the file is, for the window to say so. Empty when there is none.
    pub fn path(&self) -> &Path {
        self.at.as_deref().unwrap_or_else(|| Path::new(""))
    }
}

/// The turn the panel is drawing: the one in flight, or the one waiting for a decision.
///
/// Everything here is computed **on the turn's own thread**, by the watcher `core` calls after
/// every event (`Sidecar::turn`), and read by the `panel` command, which takes no session lock
/// at all. That is what lets the proposal be drawn as it grows: a turn holds the session for as
/// long as the model takes, so a window that could only read the proposal through the session
/// would have nothing to draw until it ended (ADR 0019 §2).
#[derive(Debug, Clone, Default)]
pub struct Live {
    pub prompt: String,
    pub prompt_id: String,
    pub at: Value,
    /// The model the last response named. Empty until one has answered.
    pub model_id: String,
    pub calls: Vec<Call>,
    pub reply: String,
    /// Still running, or waiting for Apply, Reject or Edit.
    pub running: bool,
    /// How the turn ended — `Answered`, or `Refused` when three of its calls were
    /// (ADR 0022 §3). Empty while it runs.
    pub ended: String,
    /// The proposal's patch — the RFC 6902 operations a person approves — or the violations
    /// that say why there is none yet.
    pub patch: Value,
    pub refused: Vec<Value>,
    /// The document as the proposal has it, so the arrangement and the roll can draw what the
    /// turn is proposing dashed over what the project still says (ADR 0019 §2).
    pub song: Value,
    /// What the window last answered Apply or Edit with, when it was refused: the same
    /// `ToolResult` shape every other refusal in this window arrives in.
    pub refusal: Value,
}

impl Live {
    pub fn started(prompt: &str, prompt_id: &str, at: Value) -> Live {
        Live {
            prompt: prompt.to_string(),
            prompt_id: prompt_id.to_string(),
            at,
            running: true,
            patch: Value::Null,
            song: Value::Null,
            refusal: Value::Null,
            ..Live::default()
        }
    }

    /// The turn is over: what it finally said, and the proposal it left behind.
    pub fn finished(&mut self, turn: &Turn, session: &Session) {
        self.watch(&turn.recorded, session);
        self.model_id = turn.model_id.clone();
        self.ended = format!("{:?}", turn.end);
        self.running = false;
    }

    /// This turn as the conversation records it, with what became of it (ADR 0021 §3).
    ///
    /// The one place a `Recorded` is built, so a turn that failed and a turn that was applied
    /// are the same shape in the file and neither is assembled twice.
    pub fn settled(&self, provider: &str, outcome: Outcome) -> Recorded {
        Recorded {
            prompt_id: self.prompt_id.clone(),
            prompt: self.prompt.clone(),
            at: self.at.clone(),
            provider: provider.to_string(),
            model_id: self.model_id.clone(),
            calls: self.calls.clone(),
            reply: self.reply.clone(),
            outcome,
        }
    }

    /// What the turn has produced so far, read off the session the proposal is on.
    ///
    /// Called from the watcher, which runs while `core` holds the session — so this is the one
    /// place the proposal is read, and everything the window draws comes from what it leaves
    /// behind here.
    pub fn watch(&mut self, turn: &WireTurn, session: &Session) {
        self.calls = turn.calls.iter().map(Call::of).collect();
        self.reply = turn.reply.clone();
        if let Some(last) = turn.calls.last().and_then(|done| done.call.as_ref()) {
            self.model_id = last.model_id.clone();
        }
        let Some(proposal) = session.proposal() else { return };
        match proposal.patch() {
            Ok(prepared) => {
                self.patch = serde_json::from_str(&escribass_core::ops_text(prepared.ops()))
                    .unwrap_or(Value::Null);
                self.refused = Vec::new();
            }
            Err(violations) => {
                self.patch = Value::Null;
                self.refused = violations.iter().map(wire_violation).collect();
            }
        }
        // Through `to_canonical_json` and parsed, which is exactly what `get_song` answers with
        // (`core/src/call.rs`, `song_answer`) — so the frontend decodes the proposal's document
        // with the same generated deserializer it decodes the project's with (ADR 0006 §6).
        self.song = escribass_core::to_canonical_json(proposal.song())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
    }

    pub fn to_json(&self) -> Value {
        json!({
            "prompt": self.prompt,
            "prompt_id": self.prompt_id,
            "at": self.at,
            "model_id": self.model_id,
            "calls": self.calls.iter().map(Call::to_json).collect::<Vec<_>>(),
            "reply": self.reply,
            "running": self.running,
            "ended": self.ended,
            "patch": self.patch,
            "refused": self.refused,
            "song": self.song,
            "refusal": self.refusal,
        })
    }
}

/// A `Violation` as the window already reads one (`App.tsx`, `ToolAnswer`).
fn wire_violation(violation: &escribass_core::Violation) -> Value {
    json!({ "path": violation.path, "rule": violation.rule, "message": violation.message })
}

/// Everything the panel is, in one place behind one lock.
#[derive(Debug, Default)]
pub struct Panel {
    /// The provider and model this project records, or the baseline default (ADR 0021 §4).
    /// Held here rather than read from the session, because the session is held for as long
    /// as a turn takes and this is asked for while one is running.
    pub provider: String,
    pub model: String,
    pub conversation: Conversation,
    pub live: Option<Live>,
}

impl Panel {
    /// What the `panel` command answers with.
    ///
    /// The model is named because the wireframes' panel names it, and it is named as what it
    /// is: `lock.json`'s record of a person's choice, never a pin — nothing verifies a hosted
    /// model id, and a readout that counted one would be a check that cannot fail dressed as
    /// one that passed (ADR 0021 §4; docs/plan.md, M3 trap 15).
    pub fn to_json(&self) -> Value {
        json!({
            "provider": self.provider,
            "model": self.model,
            "at": self.conversation.path().display().to_string(),
            "conversation": self.conversation.turns().iter().map(Recorded::to_json)
                .collect::<Vec<_>>(),
            "live": self.live.as_ref().map(Live::to_json).unwrap_or(Value::Null),
        })
    }
}

/// A `ToolResult` as the window reads one, for the answers the panel's own commands give.
pub fn result_json(result: &ToolResult) -> Value {
    let patch: Value = serde_json::from_slice(&result.patch).unwrap_or(Value::Null);
    let mut answer = Map::new();
    answer.insert("valid".into(), json!(result.valid));
    answer.insert(
        "errors".into(),
        result
            .errors
            .iter()
            .map(|violation| {
                json!({
                    "path": violation.path,
                    "rule": violation.rule,
                    "message": violation.message,
                })
            })
            .collect(),
    );
    answer.insert("patch".into(), patch);
    answer.insert("summary".into(), json!(result.summary));
    answer.insert("entry_id".into(), json!(result.entry_id));
    Value::Object(answer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a_turn() -> Recorded {
        Recorded {
            prompt_id: "b0a1".to_string(),
            prompt: "add a lead line over the bass".to_string(),
            at: json!("2026-09-24T12:00:00Z"),
            provider: "openrouter".to_string(),
            model_id: "deepseek/deepseek-v4.1-flash".to_string(),
            calls: vec![
                Call {
                    call_id: "call_01".to_string(),
                    name: "add_track".to_string(),
                    args: r#"{"name": "Lead", "kind": 1}"#.to_string(),
                    model_id: "deepseek/deepseek-v4.1-flash".to_string(),
                    valid: true,
                    summary: "2 ops: /tracks/01M…, /instruments/01M…".to_string(),
                    errors: vec![],
                },
                Call {
                    call_id: "call_02".to_string(),
                    name: "add_clip".to_string(),
                    args: r#"{"track_id": "nope"}"#.to_string(),
                    model_id: "deepseek/deepseek-v4.1-flash".to_string(),
                    valid: false,
                    summary: String::new(),
                    errors: vec![Refusal {
                        path: "/clips/x/track_id".to_string(),
                        rule: "track_unknown".to_string(),
                        message: "no track `nope`".to_string(),
                    }],
                },
            ],
            reply: "I proposed a lead line.".to_string(),
            outcome: Outcome::Applied("01M1FPMP00000000000000003B".to_string()),
        }
    }

    #[test]
    fn a_line_read_back_is_the_turn_that_was_written() {
        // The whole reason the file carries what it carries: a conversation read from disk is
        // sent to the model as the one held in memory is, so a restarted window does not
        // quietly send the model something else (ADR 0021 §3).
        let written = a_turn();
        let line = written.to_json().to_string();
        assert!(!line.contains('\n'), "a line is a line");
        let read = Recorded::from_json(&serde_json::from_str(&line).expect("a line is JSON"))
            .expect("a line is a turn");
        assert_eq!(read, written);
        assert_eq!(read.wire(), written.wire());
    }

    #[test]
    fn the_wire_turn_carries_what_ai_rebuilds_a_past_turn_from() {
        // `turn.py`'s `_history` and `answered`: the call's id, name and **verbatim**
        // arguments, and whether it was refused with every violation. Nothing else is re-sent,
        // which is why nothing else is kept.
        let wire = a_turn().wire();
        assert_eq!(wire.prompt, "add a lead line over the bass");
        assert_eq!(wire.reply, "I proposed a lead line.");
        let first = wire.calls[0].call.as_ref().expect("a call");
        assert_eq!(first.args_json, r#"{"name": "Lead", "kind": 1}"#);
        let refused = wire.calls[1].result.as_ref().expect("a result");
        assert!(!refused.valid);
        assert_eq!(refused.errors[0].rule, "track_unknown");
    }

    #[test]
    fn every_outcome_survives_the_file() {
        for outcome in [
            Outcome::Applied("01M".to_string()),
            Outcome::Rejected,
            Outcome::Edited("01N".to_string()),
            Outcome::Failed("provider_failed: 429".to_string()),
        ] {
            let mut turn = a_turn();
            turn.outcome = outcome.clone();
            let line: Value = serde_json::from_str(&turn.to_json().to_string()).expect("JSON");
            assert_eq!(Outcome::from_json(&line["outcome"]), outcome);
        }
    }

    #[test]
    fn a_conversation_is_read_back_from_its_file_and_a_bad_line_is_skipped() {
        let directory = std::env::temp_dir().join(format!("escribass-panel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let mut conversation = Conversation::read(Some(directory.clone()), "01M1FPMPSONG");
        assert!(conversation.turns().is_empty());
        conversation.append(a_turn());
        let mut second = a_turn();
        second.prompt = "and a pad".to_string();
        second.outcome = Outcome::Rejected;
        conversation.append(second);

        // A half-written line, which is what a machine that lost power leaves behind.
        use std::io::Write;
        let path = conversation.path().to_path_buf();
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("the file is there")
            .write_all(b"{\"prompt_id\": \"tru")
            .expect("it writes");

        let reopened = Conversation::read(Some(directory.clone()), "01M1FPMPSONG");
        assert_eq!(reopened.turns().len(), 2, "a bad line took the good ones with it");
        assert_eq!(reopened.turns()[1].prompt, "and a pad");
        assert_eq!(reopened.wire(), conversation.wire());
        // Keyed by the song's id, so a second project in the same directory is a second file.
        assert!(Conversation::read(Some(directory.clone()), "01M1FPMPOTHER").turns().is_empty());
        let _ = std::fs::remove_dir_all(&directory);
    }
}
