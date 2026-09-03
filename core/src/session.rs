//! The tool API, implemented once (§5, ADR 0006).
//!
//! Both transports — gRPC and MCP — dispatch here. They translate; they decide nothing. Every
//! rule about what a tool call means, what it returns, and what counts as an error lives in
//! this file, so the two surfaces cannot drift into disagreeing about the same call.
//!
//! **The split ADR 0006 §2 makes, made once, by return type.** `Ok(ToolResult)` with
//! `valid = false` is something the caller can fix by calling differently; `Err(ProjectError)`
//! is something only an operator can fix. It is not a classifier over rule names: `prepare`
//! is pure and cannot fail for an operator reason, and `record` only fails for one, so the
//! signatures already carry the distinction and this module just keeps it.
//!
//! **A dry run is the real path's first half**, not a copy of it (ADR 0006 §3). The patch a
//! dry run returns is produced by the same `ops_text` that fills the entry a commit writes.

use crate::clock::Clock;
use crate::history::{ops_text, HistoryError};
use crate::id::IdSource;
use crate::patch::Op;
use crate::project::{Prepared, Project, ProjectError};
use crate::validate::Violation;
use escribass_proto::tools::{
    ApplyPatchRequest, GetSongAtRequest, HistoryResponse, SongResponse, ToolResult,
};
use escribass_schema::song::{Author, Song};
use serde_json::Value;

/// One open project, and the sources of the two things §11 forbids `core` from reaching for
/// on its own.
///
/// This is ADR 0001's "session constructor". The project is named when the process starts and
/// never changes: an MCP stdio process is not a session, and two writers on one `.escri` would
/// race the write ordering ADR 0004 depends on (ADR 0006 §5).
pub struct Session {
    project: Project,
    ids: Box<dyn IdSource>,
    clock: Box<dyn Clock>,
    author: Author,
}

impl Session {
    pub fn new(
        project: Project,
        ids: Box<dyn IdSource>,
        clock: Box<dyn Clock>,
        author: Author,
    ) -> Self {
        Self { project, ids, clock, author }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    // ---- reads (ADR 0006 §1) ----

    pub fn get_song(&self) -> SongResponse {
        SongResponse { song: Some(self.project.song().clone()) }
    }

    /// The document as it stood at one entry, by replaying the log to it (ADR 0001 §2).
    pub fn get_song_at(&self, request: &GetSongAtRequest) -> Result<SongResponse, ProjectError> {
        let replayed = self
            .project
            .history()
            .materialise(&request.entry_id)
            .map_err(|e| history_error(&e))?;
        let song: Song = serde_json::from_value(replayed).map_err(|e| ProjectError {
            path: request.entry_id.clone(),
            rule: "replay_failed",
            message: format!("the replayed document is not a valid song: {e}"),
        })?;
        Ok(SongResponse { song: Some(song) })
    }

    pub fn get_history(&self) -> HistoryResponse {
        HistoryResponse {
            entries: self.project.history().entries().clone().into_iter().collect(),
            refs: Some(self.project.history().refs().clone()),
        }
    }

    // ---- the raw pipeline ----

    /// Apply an RFC 6902 patch (ADR 0006, `apply_patch`).
    ///
    /// The typed tools are the supported surface; this one exists because M0.4 has to drive
    /// `core` through the tool API rather than the file (CLAUDE.md #2), and because §9 lets a
    /// user edit a proposed diff before applying it.
    pub fn apply_patch(&mut self, request: &ApplyPatchRequest) -> Result<ToolResult, ProjectError> {
        let mut ops: Vec<Op> = match serde_json::from_slice(&request.patch) {
            Ok(ops) => ops,
            Err(e) => {
                return Ok(refused(vec![Violation {
                    path: String::new(),
                    rule: "patch_unreadable",
                    message: format!("`patch` is not an RFC 6902 array: {e}"),
                }]))
            }
        };
        normalise_input(&mut ops);
        self.run("apply_patch", &ops, request.dry_run)
    }

    /// Every mutating tool ends here: prepare, and either describe it or record it.
    ///
    /// Private because the typed tools of the next steps are the surface; what they share is
    /// this function, not a trait.
    fn run(&mut self, tool: &str, ops: &[Op], dry_run: bool) -> Result<ToolResult, ProjectError> {
        let prepared = match self.project.prepare(ops) {
            Ok(prepared) => prepared,
            Err(violations) => return Ok(refused(violations)),
        };

        // Built from `prepared` before anything is written, so a dry run and the commit that
        // follows it cannot describe different patches.
        let patch = ops_text(prepared.ops()).into_bytes();
        let summary = summarise(prepared.ops());

        if dry_run {
            return Ok(ToolResult {
                valid: true,
                errors: vec![],
                patch,
                summary,
                entry_id: String::new(),
            });
        }

        let entry_id = self.commit(prepared, tool)?;
        Ok(ToolResult { valid: true, errors: vec![], patch, summary, entry_id })
    }

    fn commit(&mut self, prepared: Prepared, tool: &str) -> Result<String, ProjectError> {
        let Some(head) = self.project.history().head_id().map(str::to_string) else {
            return Err(ProjectError {
                path: self.project.root().display().to_string(),
                rule: "head_unset",
                message: "HEAD names no entry".to_string(),
            });
        };
        self.project.record(prepared, tool, vec![head], self.author, &mut *self.ids, &*self.clock)
    }
}

/// A refused call: no patch, no entry, and every reason it was refused (§5, §6).
fn refused(violations: Vec<Violation>) -> ToolResult {
    ToolResult {
        valid: false,
        errors: violations.into_iter().map(wire).collect(),
        patch: Vec::new(),
        summary: String::new(),
        entry_id: String::new(),
    }
}

/// `Violation` as it crosses the wire. Not a fifth error type — the transport of the one
/// `core` already returns everywhere (ADR 0006 §2).
fn wire(violation: Violation) -> escribass_proto::tools::Violation {
    escribass_proto::tools::Violation {
        path: violation.path,
        rule: violation.rule.to_string(),
        message: violation.message,
    }
}

/// A `HistoryError` reaching a caller. Same three fields, so no translation is needed beyond
/// the type.
fn history_error(e: &HistoryError) -> ProjectError {
    ProjectError { path: e.path.clone(), rule: e.rule, message: e.message.clone() }
}

/// One deterministic line describing a patch (§5's `summary`).
///
/// Generated from the diff rather than written per tool: two identical calls must produce
/// identical bytes (§11), and a sentence composed by a tool is a second description of what
/// the ops already say. M3 can enrich this; it cannot make it non-deterministic.
fn summarise(ops: &[Op]) -> String {
    const SHOWN: usize = 3;
    if ops.is_empty() {
        return "no change".to_string();
    }
    let paths: Vec<&str> = ops.iter().map(Op::path).take(SHOWN).collect();
    let more = ops.len().saturating_sub(SHOWN);
    format!(
        "{} op{}: {}{}",
        ops.len(),
        if ops.len() == 1 { "" } else { "s" },
        paths.join(", "),
        if more > 0 { format!(", and {more} more") } else { String::new() },
    )
}

/// Normalises values entering through the tool API (ADR 0002 §4).
///
/// `-0.0` is the one that matters: it is a distinct double that serialises as `-0.0`, so a
/// song carrying one is not byte-identical to the same song carrying `0.0`, and §11's
/// determinism claim quietly weakens. ADR 0002 §4 assigns the fix to this boundary — the
/// validator rejects what gets past it (`negative_zero`), and this is what keeps that rule
/// from ever firing on ordinary input.
fn normalise_input(ops: &mut [Op]) {
    for op in ops {
        match op {
            Op::Add { value, .. } | Op::Replace { value, .. } | Op::Test { value, .. } => {
                normalise(value)
            }
            Op::Remove { .. } | Op::Copy { .. } | Op::Move { .. } => {}
        }
    }
}

fn normalise(value: &mut Value) {
    match value {
        Value::Number(n) => {
            if n.as_f64() == Some(0.0) && n.as_f64().is_some_and(|f| f.is_sign_negative()) {
                *value = Value::from(0.0);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalise),
        Value::Object(map) => map.values_mut().for_each(normalise),
        _ => {}
    }
}

/// A new song: the smallest document that passes the validator (§4.4).
///
/// A master track because §4.4 requires exactly one; a tempo event at tick 0 and a time
/// signature because the maps may not be empty and need an origin; a render target because its
/// enum may not be left unspecified. Nothing else — a new project is empty, not opinionated.
///
/// Ids and the timestamp come from the caller's sources, never from `SystemTime::now` or an
/// unseeded generator (§11, CLAUDE.md #3), which is what lets two scripted sessions create
/// byte-identical projects.
pub fn new_song(ids: &mut dyn IdSource, clock: &dyn Clock) -> Song {
    use escribass_schema::song::*;
    use escribass_schema::SCHEMA_VERSION;

    let made = || Provenance {
        author: Author::Human as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(clock.now()),
    };

    let song_id = ids.next_id();
    let master_id = ids.next_id();
    let tempo_id = ids.next_id();
    let signature_id = ids.next_id();

    Song {
        id: song_id,
        provenance: Some(made()),
        version: 1,
        schema_version: SCHEMA_VERSION,
        tempo_map: Some(TempoMap {
            events: [(
                tempo_id.clone(),
                TempoEvent { id: tempo_id, tick: 0, bpm: 120.0 },
            )]
            .into_iter()
            .collect(),
        }),
        time_signature_map: Some(TimeSignatureMap {
            events: [(
                signature_id.clone(),
                TimeSignatureEvent { id: signature_id, tick: 0, numerator: 4, denominator: 4 },
            )]
            .into_iter()
            .collect(),
        }),
        sections: Default::default(),
        markers: Default::default(),
        tracks: [(
            master_id.clone(),
            Track {
                id: master_id,
                provenance: Some(made()),
                version: 1,
                name: "Master".to_string(),
                kind: TrackKind::Master as i32,
                index: 0,
                instrument: None,
                fx_chain: Default::default(),
                routing: Some(Routing::default()),
                mix: Some(Mix { gain_db: 0.0, pan: 0.0, mute: false, solo: false }),
                allow_overlap: false,
            },
        )]
        .into_iter()
        .collect(),
        clips: Default::default(),
        automation: Default::default(),
        generators: Default::default(),
        render_target: Some(RenderTarget {
            kind: RenderKind::Master as i32,
            sample_rate: 48_000,
            bit_depth: 24,
            dither: false,
        }),
    }
}
