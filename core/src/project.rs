//! The `.escri` project directory (§10, ADR 0004).
//!
//! The five members §10 names, and nothing else:
//!
//! ```text
//! <name>.escri/
//!   song.json    the canonical model — a derived cache of patches/ (ADR 0004)
//!   patches/     one <entry-id>.json per patch entry, append-only
//!   refs.json    branch pointers and HEAD
//!   assets/      content-addressed: one file per asset, named by its SHA-256 (`add_asset`)
//!   lock.json    versions checked at load (§11)
//! ```
//!
//! **The log is authoritative and `song.json` is derived** (ADR 0004). Opening a project
//! replays the log and compares; a mismatch is reported rather than silently resolved in
//! either direction, because preferring the log would discard an edit somebody made and
//! preferring the document would let a hand-edited file diverge from its own history.
//!
//! Mutation lives in the next step. This module reads and writes what it is given.

use crate::canonical::to_canonical_json;
use crate::clock::Clock;
use crate::id::IdSource;
use crate::manifest::Manifest;
use crate::patch::{apply, Op};
use crate::validate::{validate, Violation};
use crate::version::{bump_versions, stamp_provenance};
use escribass_schema::song::{device_ref, Author, DeviceRef, Provenance};
use crate::history::{
    check_refs, entry_from_json, entry_to_json, refs_from_json, refs_to_json, History,
};
use crate::patch::diff;
use crate::history::entry as new_entry;
use escribass_schema::song::Song;
use escribass_schema::SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SONG: &str = "song.json";
const PATCHES: &str = "patches";
const REFS: &str = "refs.json";
const ASSETS: &str = "assets";
const LOCK: &str = "lock.json";
/// The directory lock, and not to be confused with `lock.json` beside it: that one pins the
/// build a project was authored against, this one says a process has the project open.
const HELD: &str = "lock";

/// A project that could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectError {
    /// The file or JSON Pointer the failure concerns.
    pub path: String,
    /// Stable machine-readable rule id, e.g. `"song_diverged"`.
    pub rule: &'static str,
    pub message: String,
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.path, self.rule, self.message)
    }
}

impl std::error::Error for ProjectError {}

fn err(path: impl AsRef<Path>, rule: &'static str, message: impl Into<String>) -> ProjectError {
    ProjectError {
        path: path.as_ref().display().to_string(),
        rule,
        message: message.into(),
    }
}

/// `lock.json` v2: the build this project was authored against (ADR 0010 §1).
///
/// Not a protobuf message, and it does not become one. It is not song state — it describes a
/// build, not a song — so CLAUDE.md #1 is not in play, and making it a proto would put a
/// message in `song.proto` that no renderer needs (§14.7).
///
/// `engine` and `plugins` default to empty because an absent pin is one that has not been
/// added yet. ADR 0010 §2 adds a pin on first *reference*, so a lock written before that
/// reference existed — including every lock this repository wrote before M1 — has nothing to
/// disagree with, and saying so costs one attribute rather than a migration.
///
/// M4 adds the compiled artefacts and model hashes ADR 0003 §3 named.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lock {
    schema_version: u32,
    #[serde(default)]
    engine: BTreeMap<String, String>,
    #[serde(default)]
    plugins: BTreeMap<String, Pin>,
    /// The model this project's prompts go to — **a record, not a pin** (ADR 0021 §4).
    ///
    /// Absent until the first prompt is sent, and absent from the file when absent here:
    /// every `lock.json` written before M3 PR 8 has no `ai` block and none gains one by
    /// being rewritten, which is what keeps the determinism goldens where they are.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ai: Option<Ai>,
}

/// What a project records about the model it is edited by (ADR 0021 §4).
///
/// Not a pin, and the difference is the whole row: a hosted model changes under its name, no
/// commit or hash verifies one, and nothing that counts "verified" versions may count this
/// (docs/plan.md, M3 trap 15). What it records is a **choice** — the provider and the model id
/// a person picked for this project — written on first use as ADR 0010 §2 writes a plugin pin,
/// never rewritten by a tool, and changed by editing the text (§2.6). What actually answered
/// is `provenance.model_id` on every entry, which is the fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ai {
    pub provider: String,
    pub model: String,
}

/// The provider and model a project records when it records none of its own (ADR 0021 §4).
///
/// `lock.baseline.json`'s `ai.provider` and `ai.model`. That file is the repository's baseline
/// document and not something a shipped binary can read, so the default lives here as well —
/// and `core/tests/project.rs` asserts the two agree, which is the only thing that keeps a
/// constant in code and a row in a JSON file from drifting apart.
pub const DEFAULT_PROVIDER: &str = "openrouter";
/// See [`DEFAULT_PROVIDER`]. Chosen by the user after the M3 spike and measured before it was
/// recorded (docs/plan.md, "DeepSeek V4.1 Flash, measured before it became the default").
pub const DEFAULT_MODEL: &str = "deepseek/deepseek-v4.1-flash";


/// What a project pins about one plugin.
///
/// No path and no parameter list. Where a binary lives is machine-specific and `lock.json` is
/// committed to the user's repository (§2.6) and byte-compared by the determinism suite;
/// parameters are large, derived from the binary, and belong to the manifest (ADR 0010 §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pin {
    commit: String,
    version: String,
}

/// Every plugin build a song depends on, in id order.
///
/// `SourceRef` and `ModelRef` are absent on purpose: they name content by hash, and the hash
/// *is* the pin (ADR 0010 §1).
///
/// **`SamplerRef` is not, corrected 2026-09-07 in PR 13.** Its hash pins the SFZ, and an SFZ is
/// not what renders it: the engine plays it through a bundled plugin whose build decides every
/// sample, and that build is exactly what §11 requires a project to pin. Before this, a
/// sampler-only project — `tests/renders/sfizz`, which names no `plugin_id` at all — had an
/// empty `plugins` block, so nothing in its `lock.json` moved when sfizz did and
/// `lock_mismatch` could not fire. Which plugin that is comes from the manifest, because it is
/// a fact about the engine build and `core` holding a second copy of it is how the two stop
/// agreeing (ADR 0010 §4, CLAUDE.md #6).
fn referenced_plugins<'a>(song: &'a Song, sampler: &'a str) -> BTreeSet<&'a str> {
    fn plugin_of<'a>(device: Option<&'a DeviceRef>, sampler: &'a str) -> Option<&'a str> {
        match device?.kind.as_ref()? {
            device_ref::Kind::Plugin(p) => Some(p.plugin_id.as_str()),
            device_ref::Kind::Sampler(_) => Some(sampler),
            _ => None,
        }
    }
    let mut found = BTreeSet::new();
    for track in song.tracks.values() {
        found.extend(plugin_of(track.instrument.as_ref().and_then(|i| i.r#ref.as_ref()), sampler));
        found.extend(track.fx_chain.values().filter_map(|e| plugin_of(e.r#ref.as_ref(), sampler)));
    }
    found
}

/// What an operator can do about a `lock_mismatch`. All three are operator actions, which is
/// why the refusal is a `ProjectError` and not a `Violation` (ADR 0006 §2, ADR 0010 §3).
const REPIN: &str = "install the build it names, rebuild, or edit lock.json deliberately \
                     (ADR 0010 §3; the tool for it is M2's)";

/// A change that has been applied, bumped and validated, but not recorded.
///
/// Held rather than returned as a tuple because `record` must take exactly what `prepare`
/// produced: re-deriving either half at the call site is how the two stop agreeing.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    song: Song,
    ops: Vec<Op>,
}

impl Prepared {
    /// The patch this change records — the re-derived diff, version bumps included.
    pub fn ops(&self) -> &[Op] {
        &self.ops
    }

    /// The document this change produces.
    pub fn song(&self) -> &Song {
        &self.song
    }
}

/// The `.escri` directory lock, held for as long as a process has the project open
/// (ADR 0012 §3).
///
/// **The lock is the kernel's, not the file's** (`std::fs::File::try_lock`, stable since Rust
/// 1.89 and therefore free of a dependency). A process opens `.escri/lock`, takes an advisory
/// lock on the open file, and holds the descriptor for as long as it has the project open; the
/// kernel releases the lock when that descriptor closes, which happens when the process ends —
/// cleanly, by `SIGKILL`, or by a crash. So there is nothing to adjudicate and nothing to
/// break: a second opener asks the kernel, and the answer is a fact about a running process
/// rather than an inference from a number in a file.
///
/// **Amended 2026-09-24 (ADR 0020 §5), replacing the pid protocol.** What stood here until
/// then read the holder's pid out of the file, asked `/proc` whether it was alive, and — if it
/// was not — unlinked the file and created a fresh one. That had two defects the kernel does
/// not have. A pid a dead holder wore and an unrelated program now wears read as *alive*, so
/// the lock stood for ever. And two openers of one crashed project could **both** end up
/// holding it: A read the stale pid, unlinked and created; B, which had read the same stale pid
/// before A's unlink, then unlinked **A's fresh lock** and created its own. The claim that a
/// same-instant replacement "finds the file there and refuses" was true only in the ordering
/// where B's unlink came first. Nothing unlinks now, so there is no window.
///
/// **The pid in the file is a label, never a decision.** It is written so a refusal can name
/// who to close, and it is what tells a person that the last holder *crashed*: a clean close
/// empties the file, so a non-empty one under a lock this process has just taken is a holder
/// that never got to `Drop` — which is ADR 0020 §5's report, kept.
///
/// Not a field of [`Project`], which is `Clone` and which `record` rebuilds and reassigns on
/// every commit — a guard living there would release the lock the replacement is still holding.
/// The lock belongs to whoever opened the project and outlives every `Project` value built from
/// it, which is the host, or one of the two server binaries.
///
/// `ponytail:` advisory, and it assumes a local filesystem — `flock` over NFS is the kernel's
/// emulation and a network filesystem is not something v1 is expected to run a project over. A
/// real lock protocol is worth writing when something is (ADR 0012 §3). The file is **never
/// removed**, which is how `cargo`'s own lock files behave and for this reason: unlinking a
/// file another process may already have open is the window this amendment closed.
#[derive(Debug)]
pub struct ProjectLock {
    /// The open descriptor the advisory lock lives on. Dropping it is releasing the lock, so
    /// this field is the whole of what the value guards; it is never read.
    held: std::fs::File,
    replaced: Option<String>,
}

impl ProjectLock {
    /// Takes the lock on an existing project directory.
    ///
    /// The directory has to be there already, which is why a `--create` takes this *after*
    /// [`Project::create`] rather than before: creating the directory in order to lock it
    /// would leave one behind for every mistyped path. Two simultaneous creates are still a
    /// race, and a much smaller one — `create` refuses outright if a project is already there.
    ///
    /// A lock whose last holder did not close cleanly is taken without ceremony — the kernel
    /// released it when that process ended — and [`replaced`] then carries the sentence that
    /// says so, naming the pid the file still held. That is the whole of ADR 0020 §5, and what
    /// it is worth is measured: 50 headless MCP sessions of 50 ended by a signal, and a signal
    /// does not run a Rust `Drop` — so every one of them left a project no next process would
    /// open.
    ///
    /// [`replaced`]: Self::replaced
    pub fn take(root: impl AsRef<Path>) -> Result<Self, ProjectError> {
        let path = root.as_ref().join(HELD);
        let held = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            // Never truncated on open: what the file holds is read *after* the lock is taken,
            // and it is the only evidence that the last holder crashed.
            .truncate(false)
            .open(&path)
            .map_err(|e| err(&path, "unwritable", e.to_string()))?;

        match held.try_lock() {
            Ok(()) => {}
            // Somebody has this open, right now. The pid is read only to name them, and a file
            // that names nobody refuses just the same — the refusal is the kernel's.
            Err(std::fs::TryLockError::WouldBlock) => {
                let owner = std::fs::read_to_string(&path).unwrap_or_default().trim().to_string();
                let named = if owner.is_empty() {
                    "another process".to_string()
                } else {
                    format!("process {owner}")
                };
                return Err(err(
                    &path,
                    "project_locked",
                    format!(
                        "{named} has this project open. Close it — the lock is the kernel's, \
                         held for as long as that process has the file open and released the \
                         moment it ends, however it ends (ADR 0012 §3, amended by ADR 0020 §5)"
                    ),
                ));
            }
            Err(std::fs::TryLockError::Error(e)) => {
                return Err(err(&path, "unwritable", e.to_string()))
            }
        }

        // Read under the lock, so what it says is settled: a non-empty file is a holder that
        // never reached its `Drop`, which is a crash or a signal (ADR 0020 §5).
        let crashed = std::fs::read_to_string(&path).unwrap_or_default().trim().to_string();
        let mut lock = Self { held, replaced: None };
        lock.sign();
        if !crashed.is_empty() {
            lock.replaced = Some(format!(
                "replaced a lock left by process {crashed}: it ended without releasing the \
                 lock, which a crash or a signal does (ADR 0020 §5)"
            ));
        }
        Ok(lock)
    }

    /// Writes this process's pid over whatever the file held. Best effort: the lock is the
    /// kernel's and this is only the label a refusal names, so a write that fails costs a
    /// sentence and never correctness.
    fn sign(&mut self) {
        use std::io::{Seek, Write};
        let _ = self.held.set_len(0);
        let _ = self.held.rewind();
        let _ = self.held.write_all(format!("{}\n", std::process::id()).as_bytes());
        let _ = self.held.flush();
    }

    /// What this take had to replace, if anything: one sentence naming the process whose lock
    /// was left behind, so a person learns a crash happened rather than nothing (ADR 0020 §5).
    pub fn replaced(&self) -> Option<&str> {
        self.replaced.as_deref()
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        // **Emptied, not removed.** Closing the descriptor is what releases the lock; this is
        // the *other* thing a clean close says — that the next opener has no crash to report.
        // Unlinking would reopen the window ADR 0020 §5's amendment closed: a file another
        // process may already have open is not ours to take away (see the type's note).
        let _ = self.held.set_len(0);
    }
}

/// An open project directory.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    root: PathBuf,
    song: Song,
    history: History,
    /// What this build can host (ADR 0010 §4). Held rather than threaded through every call
    /// because all three of the paths that need it — validating, opening and pinning — are
    /// reached from the outside separately. `Arc` because the manifest is one shared,
    /// read-only description of the build and `record` builds a fresh `Project` per commit;
    /// deep-copying 3000 parameter names per tool call would be the cost of a plain field.
    manifest: Arc<Manifest>,
    /// The plugin pins this project carries, as `lock.json` last held them. Monotone: `write`
    /// adds an entry for every referenced plugin that has none and removes nothing
    /// (ADR 0010 §2).
    pins: BTreeMap<String, Pin>,
    /// What `lock.json`'s `ai` block says, if it says anything (ADR 0021 §4). `None` until a
    /// prompt has been sent in this project.
    ai: Option<Ai>,
}

impl Project {
    /// Assembles a project in memory. Writes nothing.
    pub fn new(
        root: impl Into<PathBuf>,
        song: Song,
        history: History,
        manifest: Arc<Manifest>,
    ) -> Self {
        Self { root: root.into(), song, history, manifest, pins: BTreeMap::new(), ai: None }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn song(&self) -> &Song {
        &self.song
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    /// Creates a new project directory from an initial song.
    ///
    /// The root entry records the patch that builds the song from a default one, so the log
    /// is complete from the first commit and a replay never needs a special case for the
    /// beginning (ADR 0004).
    ///
    /// The id source and clock are parameters rather than fields: §11 forbids `core` from
    /// reading the wall clock or taking unseeded randomness on its own. M0.3's session will
    /// own them, which is where ADR 0001's "session constructor" lands.
    pub fn create(
        root: impl Into<PathBuf>,
        song: &Song,
        ids: &mut dyn IdSource,
        clock: &dyn Clock,
        author: Author,
        manifest: Arc<Manifest>,
    ) -> Result<Project, ProjectError> {
        let root = root.into();
        if root.join(SONG).exists() {
            return Err(err(root.join(SONG), "project_exists", "a project is already here"));
        }
        refuse_if_invalid(&root, song, &manifest)?;

        let empty = serde_json::to_value(Song::default()).expect("a default Song serialises");
        let full = serde_json::to_value(song).expect("a Song serialises");

        let id = ids.next_id();
        let mut history = History::new();
        history
            .append(new_entry(id.clone(), vec![], "create", &diff(&empty, &full),
                authorship(author, clock), SCHEMA_VERSION))
            .map_err(|e| err(&root, e.rule, e.message))?;
        history.create_ref("main", &id).map_err(|e| err(&root, e.rule, e.message))?;
        history.set_head("main").map_err(|e| err(&root, e.rule, e.message))?;

        let mut project = Project {
            root,
            song: song.clone(),
            history,
            manifest,
            pins: BTreeMap::new(),
            ai: None,
        };
        project.write()?;
        Ok(project)
    }

    /// Applies a patch, records it, and writes the project.
    ///
    /// Every check happens before anything is mutated, so a rejected commit leaves no orphan
    /// entry and no advanced ref — the project is exactly as it was.
    ///
    /// The re-deserialisation through `Song` is the step that matters. An operation can be
    /// legal JSON and illegal for the schema — `43.0` at an `int32` path is the defect ADR
    /// 0002 §11 records — and applying it to a `Value` would succeed while producing a
    /// document `core` cannot read back.
    ///
    /// What the entry records is the patch from the document before to the document after —
    /// the *effect*, re-derived — not the ops as they arrived. Proto3 JSON has more than one
    /// spelling for a value: `"1"` is a legal int32, `1` a legal enum. Both deserialise, and
    /// both come back out of `Song` in one canonical spelling. Recording the caller's spelling
    /// would leave the log replaying to a document that differs from the `song.json` written
    /// beside it, and the next `open` would refuse a project that was committed cleanly. The
    /// re-derived diff cannot say anything `song.json` does not (ADR 0004).
    ///
    /// Entity `version` is bumped inside [`Project::prepare`], which is the only position
    /// where the bump lands in the recorded ops (ADR 0005 §1).
    pub fn commit(
        &mut self,
        tool: &str,
        ops: &[Op],
        author: Author,
        ids: &mut dyn IdSource,
        clock: &dyn Clock,
    ) -> Result<String, ProjectError> {
        let made = authorship(author, clock);
        let prepared = self.prepare(ops, &made).map_err(|v| refusal(&self.root, &v))?;
        let Some(head) = self.history.head_id().map(str::to_string) else {
            return Err(err(self.root.join(REFS), "head_unset", "HEAD names no entry"));
        };
        self.record(prepared, tool, vec![head], &made, ids)
    }

    /// Everything a commit does *except* commit: apply, bump, validate, re-derive the patch.
    ///
    /// Touches nothing and mints nothing, so it is also what `dry_run` returns (ADR 0006 §3).
    /// A dry run is this function; there is no second implementation of the apply path to
    /// drift from the real one.
    /// Fails with **every** reason it failed, never the first: §5 promises errors a model can
    /// act on and §6 gives it three retries, which one-problem-per-round-trip spends on a
    /// document that had four.
    ///
    /// The error type is `Vec<Violation>` rather than `ProjectError` because `prepare` touches
    /// no file: every way it can fail is something the caller can fix by calling differently.
    /// That is the `Ok(valid = false)` half of ADR 0006 §2's split, and it falls out of the
    /// signature instead of needing a classifier.
    ///
    /// `made` is **the call's** provenance, and it is what a new entity is stamped with: core
    /// decides every entity's `provenance` here, on this path as on the typed ones, because
    /// this is the path a caller can write one through (ADR 0021 §1). The three optional ids
    /// are `None` for every caller that exists today; the proposal is the one place that will
    /// fill them (ADR 0021 §2, M3 PR 8).
    pub fn prepare(&self, ops: &[Op], made: &Provenance) -> Result<Prepared, Vec<Violation>> {
        self.prepare_inner(ops, Some(made), false)
    }

    /// [`prepare`](Self::prepare) for a **proposal's patch** (ADR 0019 §2): the version rule of
    /// an ordinary edit, and provenance left exactly as the ops carry it.
    ///
    /// The two exemptions `prepare_merge` grants together come apart here, which is why this is
    /// its own entry point rather than a third argument at the call site. Versions **are**
    /// guarded, because the patch is a caller's claim about a document that may have moved
    /// under it — that guard is the whole of how a pending proposal meets a person's
    /// concurrent edit (ADR 0019 §2; ADR 0012 §4). Provenance is **not** decided, because it
    /// already was: every entity in these ops was stamped by `prepare` on the fork, with the
    /// three ids of the call that minted it (ADR 0021 §2). Re-deciding it here would replace
    /// the call with the turn and lose the `tool_call_id` the entity is supposed to carry,
    /// which is the same "core's own values arriving from elsewhere" that exempts a merge.
    pub fn prepare_proposal(&self, ops: &[Op]) -> Result<Prepared, Vec<Violation>> {
        self.prepare_inner(ops, None, false)
    }

    /// [`prepare`](Self::prepare) for the callers whose operations legitimately carry an entity
    /// `version` and an entity `provenance`: a merge, and — since M2 PR 5 — `undo` and `redo`.
    ///
    /// A merge's patch is the diff between two states core itself produced, so the versions in
    /// it are core's own — the incoming branch's numbers, which ADR 0005 §2 needs in order to
    /// resolve to `max(ours, theirs) + 1`. Refusing them here would silently discard the other
    /// branch's count and let a client's version go backwards, which is what ADR 0005 §4
    /// rejected the rewind for.
    ///
    /// Undo is the same case by a different route: its ops are `diff(current, materialise(…))`,
    /// so the versions in them were read back out of a document core wrote. `prepare` would
    /// refuse this API's own history as though a caller had tried to write a field it does not
    /// own, and the resolution rule is exactly the one undo needs — `max` of the number the
    /// entity has now and the number it had then, plus one, which is one *past* where it is.
    ///
    /// **Provenance is exempt on exactly the same grounds** (ADR 0021 §1). A merge's new
    /// entities were minted by the other branch's author and an undo's re-added entity by
    /// whoever added it; both are core's own values arriving from the log, and stamping them
    /// with the author of the call that merged or redid would rewrite who made what.
    pub fn prepare_merge(&self, ops: &[Op]) -> Result<Prepared, Vec<Violation>> {
        self.prepare_inner(ops, None, true)
    }

    /// Two independent questions, asked separately because M3 found a caller that answers them
    /// differently.
    ///
    /// `made` decides **provenance**: `Some` for ops that are a caller's, so a new entity is
    /// stamped with the call; `None` for ops carrying provenance core itself wrote — a merge,
    /// an undo, a redo, and a proposal's patch.
    ///
    /// `merging` decides the **version** rule: `false` bumps by one and disputes a number the
    /// caller chose (ADR 0005 §3), `true` resolves `max(ours, theirs) + 1` and disputes
    /// nothing (ADR 0005 §2).
    ///
    /// The two coincided for the only two callers that existed until M3, which is why they
    /// were one argument; a proposal's patch is `None` and `false` (ADR 0019 §2).
    fn prepare_inner(
        &self,
        ops: &[Op],
        made: Option<&Provenance>,
        merging: bool,
    ) -> Result<Prepared, Vec<Violation>> {
        let before = serde_json::to_value(&self.song).expect("a Song serialises");

        let mut patched = apply(&before, ops).map_err(|e| {
            vec![Violation { path: e.path, rule: e.rule, message: e.message }]
        })?;

        // Provenance first, versions second. A caller that rewrote nothing but a provenance has
        // then changed nothing at all, so nothing bumps and nothing is recorded — where bumping
        // first would write an entry whose only operation raised a version for a field that was
        // put straight back (ADR 0021 §1).
        if let Some(made) = made {
            stamp_provenance(
                &before,
                &mut patched,
                &serde_json::to_value(made).expect("a Provenance serialises"),
            );
        }

        let disputed = bump_versions(&before, &mut patched, merging);
        if !disputed.is_empty() {
            return Err(disputed);
        }

        let song: Song = serde_json::from_value(patched).map_err(|e| {
            let touched: Vec<&str> = ops.iter().map(Op::path).take(8).collect();
            vec![Violation {
                // `ponytail:` names the ops rather than the exact field. Reach for
                // `serde_path_to_error` if that is ever not enough to find it.
                path: touched.first().map(|p| (*p).to_string()).unwrap_or_default(),
                rule: "op_illegal_for_schema",
                message: format!("the patched document is not a valid song: {e}. Ops touched: {}",
                    touched.join(", ")),
            }]
        })?;

        let violations = validate(&song, &self.manifest);
        if !violations.is_empty() {
            return Err(violations);
        }

        let after = serde_json::to_value(&song).expect("a Song serialises");
        Ok(Prepared { ops: diff(&before, &after), song })
    }

    /// Appends a prepared change to the log and writes the project.
    ///
    /// `parents` rather than one parent: a merge appends a single entry naming both sides
    /// (ADR 0001 §1), and that is the only difference between a merge and a commit at this
    /// level.
    /// `made` is the **entry's** provenance, stated rather than derived from an author and a
    /// clock: a proposal's entry names the model and the prompt as well (ADR 0021 §2), and the
    /// session is what knows which of those it has.
    pub fn record(
        &mut self,
        prepared: Prepared,
        tool: &str,
        parents: Vec<String>,
        made: &Provenance,
        ids: &mut dyn IdSource,
    ) -> Result<String, ProjectError> {
        let branch = self.history.refs().head.clone();
        let id = ids.next_id();
        let entry = new_entry(id.clone(), parents, tool, &prepared.ops,
            made.clone(), SCHEMA_VERSION);

        // The next state is built beside this one and only swapped in once it is on disk.
        // Mutating first and writing second left a failed write with the session one commit
        // ahead of the directory: the caller is told nothing happened, and every later call
        // builds on state the disk never saw (ADR 0004's ordering is about files; this is the
        // same argument about memory).
        let mut history = self.history.clone();
        history.append(entry).map_err(|e| err(&self.root, e.rule, e.message))?;
        history.advance(&branch, &id).map_err(|e| err(&self.root, e.rule, e.message))?;

        let mut next = Project {
            root: self.root.clone(),
            song: prepared.song,
            history,
            manifest: Arc::clone(&self.manifest),
            pins: self.pins.clone(),
            ai: self.ai.clone(),
        };
        next.write()?;
        *self = next;
        Ok(id)
    }

    /// Adopts a prepared document, recording nothing and writing nothing.
    ///
    /// The **proposal's** step and nothing else's (ADR 0019 §1): the model's calls run against
    /// a copy of the project, each one prepared by the same `prepare` and validated by the same
    /// rules, and what they leave behind is a document a person has not approved yet. It is
    /// `pub(crate)` because a document with no entry is exactly what ADR 0001 §2 and ADR 0004
    /// forbid on disk — the fork is in memory, is thrown away on Reject, and reaches the log
    /// only as the one patch decision 2 commits.
    pub(crate) fn keep(&mut self, prepared: Prepared) {
        self.song = prepared.song;
    }

    // ---- branches (ADR 0001 §2) ----

    /// Names a new position in the log and writes `refs.json`. Copies no data.
    pub fn create_branch(&mut self, name: &str, at: &str) -> Result<(), ProjectError> {
        let mut history = self.history.clone();
        history.create_ref(name, at).map_err(|e| err(&self.root, e.rule, e.message))?;
        self.swap(self.song.clone(), history)
    }

    /// Moves `HEAD` to another branch, returning the patch that got there.
    ///
    /// Appends nothing: switching is navigation, and a log that recorded navigation would grow
    /// every time somebody looked at a branch (ADR 0001 §2).
    pub fn switch_branch(&mut self, name: &str) -> Result<Vec<Op>, ProjectError> {
        let current = serde_json::to_value(&self.song).expect("a Song serialises");
        let ops = self
            .history
            .patch_to(name, &current)
            .map_err(|e| err(&self.root, e.rule, e.message))?;

        let target = apply(&current, &ops)
            .map_err(|e| err(self.root.join(SONG), e.rule, e.message))?;
        // A branch was validated when it was committed, so a failure here means the log no
        // longer replays into a song this build can read — an operator's problem, not a
        // caller's (ADR 0006 §2).
        let song: Song = serde_json::from_value(target).map_err(|e| {
            err(self.root.join(SONG), "replay_failed",
                format!("`{name}` does not replay into a valid song: {e}"))
        })?;

        let mut history = self.history.clone();
        history.set_head(name).map_err(|e| err(&self.root, e.rule, e.message))?;
        self.swap(song, history)?;
        Ok(ops)
    }

    /// Discards a branch name. The entries stay in the log, unreferenced and inert.
    pub fn delete_branch(&mut self, name: &str) -> Result<(), ProjectError> {
        let mut history = self.history.clone();
        history.delete_ref(name).map_err(|e| err(&self.root, e.rule, e.message))?;
        self.swap(self.song.clone(), history)
    }

    // ---- assets (§10) ----

    /// Puts `content` in `assets/` under its hash, and returns that hash.
    ///
    /// Content-addressed, so the name is a pure function of the bytes and a second call with
    /// the same content writes nothing: the file that is there *is* the file that would be
    /// written. That is what makes the call safe to repeat, and what keeps an asset out of
    /// the patch log — the log records changes to the song, and an asset is not in the song
    /// until a clip names it (CLAUDE.md #2 is about the document).
    ///
    /// `&self` rather than `&mut self`: nothing in memory changes. The index from hash to
    /// path that `compile` takes (ADR 0007 §4) is read from the directory when it is needed.
    ///
    /// `ponytail:` a file already present is trusted by name; nothing re-hashes it to prove
    /// the bytes match. Add a check here if a corrupted asset ever needs to be caught before
    /// the engine fails to open it.
    pub fn add_asset(&self, content: &[u8]) -> Result<String, ProjectError> {
        let hash = asset_hash(content);
        // Created here as well as in `write`: git stores no empty directory, so a project
        // cloned before its first asset arrives without one.
        let assets = self.root.join(ASSETS);
        std::fs::create_dir_all(&assets).map_err(|e| err(&assets, "unwritable", e.to_string()))?;
        let path = assets.join(&hash);
        if !path.exists() {
            write_atomically(&path, content)?;
        }
        Ok(hash)
    }

    /// The index `compile` resolves an `asset_hash` through (ADR 0007 §4): every file in
    /// `assets/`, by name, at an absolute path.
    ///
    /// Built from the directory listing rather than from the song, because the song names
    /// hashes and this is the only place that knows where they live. A project with no
    /// `assets/` yet indexes nothing rather than failing — the directory arrives with the
    /// first `add_asset`.
    ///
    /// `ponytail:` `std::path::absolute` is lexical, so it makes a path the engine can open
    /// from any working directory without resolving a symlink or touching the disk. Canonical
    /// paths would be the upgrade if a project is ever reached through one.
    pub fn assets(&self) -> Result<BTreeMap<String, PathBuf>, ProjectError> {
        let assets = self.root.join(ASSETS);
        let listing = match std::fs::read_dir(&assets) {
            Ok(listing) => listing,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
            Err(e) => return Err(err(&assets, "unreadable", e.to_string())),
        };
        let mut found = BTreeMap::new();
        for entry in listing {
            let path = entry.map_err(|e| err(&assets, "unreadable", e.to_string()))?.path();
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            let absolute =
                std::path::absolute(&path).map_err(|e| err(&path, "unreadable", e.to_string()))?;
            found.insert(name, absolute);
        }
        Ok(found)
    }

    /// Writes a proposed state and adopts it only once the write succeeded.
    fn swap(&mut self, song: Song, history: History) -> Result<(), ProjectError> {
        let mut next = Project {
            root: self.root.clone(),
            song,
            history,
            manifest: Arc::clone(&self.manifest),
            pins: self.pins.clone(),
            ai: self.ai.clone(),
        };
        next.write()?;
        *self = next;
        Ok(())
    }

    /// Reads a project directory, verifying that `song.json` matches a replay of the log and
    /// that every plugin it references is pinned at what this build has (§11, ADR 0010 §3).
    pub fn open(root: impl AsRef<Path>, manifest: Arc<Manifest>) -> Result<Project, ProjectError> {
        let root = root.as_ref().to_path_buf();

        let lock: Lock = serde_json::from_str(&read(&root.join(LOCK))?)
            .map_err(|e| err(root.join(LOCK), "lock_unreadable", e.to_string()))?;
        if lock.schema_version != SCHEMA_VERSION {
            return Err(err(
                root.join(LOCK),
                "schema_version_mismatch",
                format!(
                    "the project was written against schema {} and this build is {SCHEMA_VERSION}",
                    lock.schema_version
                ),
            ));
        }

        let song: Song = crate::from_canonical_json(&read(&root.join(SONG))?)
            .map_err(|e| err(root.join(SONG), "song_unreadable", e.to_string()))?;

        refuse_if_unpinned(&root, &song, &lock, &manifest)?;

        let refs = refs_from_json(&read(&root.join(REFS))?)
            .map_err(|e| err(root.join(REFS), e.rule, e.message))?;

        let mut entries = Vec::new();
        let patches = root.join(PATCHES);
        let listing = std::fs::read_dir(&patches)
            .map_err(|e| err(&patches, "unreadable", e.to_string()))?;
        for found in listing {
            let path = found.map_err(|e| err(&patches, "unreadable", e.to_string()))?.path();
            // Anything that is not a patch file is ignored: the directory belongs to us, but
            // an editor's swap file is not a reason to refuse to open a project.
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let entry = entry_from_json(&read(&path)?)
                .map_err(|e| err(&path, e.rule, e.message))?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
            if stem != entry.id {
                return Err(err(
                    &path,
                    "entry_filename_mismatch",
                    format!("the file is named `{stem}` and the entry inside it is `{}`", entry.id),
                ));
            }
            entries.push(entry);
        }

        let history = History::from_parts(entries, refs)
            .map_err(|e| err(&patches, e.rule, e.message))?;

        // The engine block is not compared, and it is not carried: `write` records the running
        // build's. ADR 0010 §3 makes this the one block that re-pins rather than refusing —
        // there is exactly one engine and a project cannot choose it, so refusing would refuse
        // every project on the machine at once, repaired by hand-editing each. Nothing is lost
        // by it, because what a render was made with travels with the render (ADR 0008 §5).
        let project = Project { root, song, history, manifest, pins: lock.plugins, ai: lock.ai };
        project.verify_against_replay()?;
        Ok(project)
    }

    /// The check ADR 0004 requires: the log is authoritative, so a `song.json` that does not
    /// match a replay is reported — naming the paths that differ — and never silently fixed.
    fn verify_against_replay(&self) -> Result<(), ProjectError> {
        let Some(head) = self.history.head_id() else {
            return Err(err(self.root.join(REFS), "head_unset", "HEAD names no entry"));
        };
        let replayed = self
            .history
            .materialise(head)
            .map_err(|e| err(self.root.join(PATCHES), e.rule, e.message))?;
        let stored = serde_json::to_value(&self.song).expect("a song serialises");
        if replayed == stored {
            return Ok(());
        }
        let differing: Vec<String> =
            diff(&stored, &replayed).iter().map(|op| op.path().to_string()).take(8).collect();
        Err(err(
            self.root.join(SONG),
            "song_diverged",
            format!(
                "song.json does not match a replay of the log, which is authoritative \
                 (ADR 0004). Differing paths: {}",
                differing.join(", ")
            ),
        ))
    }

    /// What `lock.json` holds right now, as text (ADR 0010 §1).
    ///
    /// Its own function because two callers write it: [`write`](Self::write), which writes the
    /// whole project, and [`record_ai`](Self::record_ai), which writes this file alone. A
    /// second spelling of the same block is how the two would come to disagree about what a
    /// lock contains.
    fn lock_text(&self) -> String {
        serde_json::to_string_pretty(&Lock {
            schema_version: SCHEMA_VERSION,
            engine: self.manifest.engine.clone(),
            plugins: self.pins.clone(),
            ai: self.ai.clone(),
        })
        .expect("the lock serialises")
            + "\n"
    }

    /// What this project records about the model it is edited by, if it records anything
    /// (ADR 0021 §4).
    pub fn ai(&self) -> Option<&Ai> {
        self.ai.as_ref()
    }

    /// Records the provider and model this project's prompts go to, **on first use**.
    ///
    /// ADR 0010 §2's shape, one block over: a plugin pin is added the first time a song
    /// references that plugin and never rewritten by a tool, and this is added the first time
    /// a prompt is sent and never rewritten either. A project that already records one keeps
    /// it — including one naming a different model, because changing it is editing the text
    /// (§2.6) and a tool that rewrote it would be writing `lock.json` outside the patch log,
    /// which is the objection the re-pin row already records (ADR 0010 §3).
    ///
    /// It writes `lock.json` and nothing else. A full [`write`](Self::write) here would rewrite
    /// every entry in `patches/` to record a fact about no entry at all — O(history) for one
    /// line (docs/plan.md, M3 trap 9).
    pub fn record_ai(&mut self, provider: &str, model: &str) -> Result<(), ProjectError> {
        if self.ai.is_some() {
            return Ok(());
        }
        self.ai = Some(Ai { provider: provider.to_string(), model: model.to_string() });
        write_atomically(&self.root.join(LOCK), self.lock_text())
    }

    /// Writes the whole project.
    ///
    /// Order is ADR 0004's: entries first, since an unreferenced entry is inert; `song.json`
    /// next; `refs.json` last, so advancing the ref is the moment the write becomes real. A
    /// crash before that leaves an orphan entry, which `open` reports rather than mistaking
    /// for history.
    pub fn write(&mut self) -> Result<(), ProjectError> {
        for directory in [&self.root, &self.root.join(PATCHES), &self.root.join(ASSETS)] {
            std::fs::create_dir_all(directory)
                .map_err(|e| err(directory, "unwritable", e.to_string()))?;
        }

        // ADR 0010 §2: the pins that already exist, plus one from the build manifest for
        // every referenced plugin that has none. Never rewritten and never dropped — a block
        // derived purely from the current song loses a pin on an ordinary delete, and ADR
        // 0005 §4's undo appends an inverse entry that re-adds the reference, which would
        // re-pin from whatever build is running now. That is a silent re-pin performed by
        // pressing undo. The cost is a stale pin, which `open` makes inert by reading only
        // the pins the song currently references.
        //
        // The mutation is the *reference*, made by a tool and in the log like every other
        // one; no entry can appear here that no logged op caused, and no tool writes this
        // file (CLAUDE.md #2).
        for id in referenced_plugins(&self.song, &self.manifest.sampler) {
            if self.pins.contains_key(id) {
                continue;
            }
            // A referenced plugin the manifest lacks cannot reach here: the validator refuses
            // it as `plugin_unknown` before anything is written. Skipping rather than
            // panicking keeps a write from being where a bug elsewhere becomes data loss.
            if let Some(built) = self.manifest.plugins.get(id) {
                let pin = Pin { commit: built.commit.clone(), version: built.version.clone() };
                self.pins.insert(id.to_string(), pin);
            }
        }

        write_atomically(&self.root.join(LOCK), self.lock_text())?;

        for (id, entry) in self.history.entries() {
            let text = entry_to_json(entry)
                .map_err(|e| err(self.root.join(PATCHES).join(id), e.rule, e.message))?;
            write_atomically(&self.root.join(PATCHES).join(format!("{id}.json")), &text)?;
        }

        let song = to_canonical_json(&self.song)
            .map_err(|e| err(self.root.join(SONG), "song_unwritable", e.to_string()))?;
        write_atomically(&self.root.join(SONG), &song)?;

        if let Some(bad) = check_refs(self.history.refs()).into_iter().next() {
            return Err(err(self.root.join(REFS), bad.rule, bad.message));
        }
        write_atomically(&self.root.join(REFS), &refs_to_json(self.history.refs()))
    }
}

/// Provenance for a call: the entry it records, and every entity it creates (ADR 0021 §1).
///
/// `created_at` comes from the injected clock, never `SystemTime::now` (§11). The three model
/// fields stay unset for every caller there is today, and the proposal is the one that will
/// set them — an MCP client's model stays anonymous even then, because nothing on that wire
/// says which model is on the other side (ADR 0021 §2).
pub fn authorship(author: Author, clock: &dyn Clock) -> Provenance {
    Provenance {
        author: author as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(clock.now()),
    }
}

/// §11's load check (ADR 0010 §3): a referenced plugin this build cannot match refuses to
/// open, and nothing is substituted, silenced or re-pinned.
///
/// **Only the pins the song currently references.** The block is monotone (§2), so a project
/// that once used Surge and no longer does still carries the entry; an inert pin is a record
/// of history, not a hostage.
///
/// A referenced plugin with *no* pin is not a disagreement. §2 adds a pin on first reference,
/// which happens at `write`, so an unpinned reference means the lock predates it — including
/// every lock written before M1 — or that an operator deleted the entry, which §3 makes the
/// way to re-pin while M1 ships no tool for it. The next write pins it.
///
/// This is `ProjectError`, not a `Violation`: every fix is an operator action, and inside
/// §6's retry loop a model would only spend retries on it (ADR 0006 §2, ADR 0010 §3).
fn refuse_if_unpinned(
    root: &Path,
    song: &Song,
    lock: &Lock,
    manifest: &Manifest,
) -> Result<(), ProjectError> {
    for id in referenced_plugins(song, &manifest.sampler) {
        let Some(pinned) = lock.plugins.get(id) else { continue };
        let at = || err(root.join(LOCK), "lock_mismatch", String::new());
        match manifest.plugins.get(id) {
            Some(built) if built.commit == pinned.commit && built.version == pinned.version => {}
            Some(built) => {
                return Err(ProjectError {
                    message: format!(
                        "`{id}` is pinned at {} ({}) and this build has {} ({}); {REPIN}",
                        pinned.version, pinned.commit, built.version, built.commit
                    ),
                    ..at()
                })
            }
            None => {
                return Err(ProjectError {
                    message: format!(
                        "`{id}` is pinned at {} ({}) and this build cannot host it; {REPIN}",
                        pinned.version, pinned.commit
                    ),
                    ..at()
                })
            }
        }
    }
    Ok(())
}

/// §5: every mutation is validated before it is applied, so an invalid song never reaches
/// the log. Reports every violation, not the first (§6 allows three retries).
fn refuse_if_invalid(root: &Path, song: &Song, manifest: &Manifest) -> Result<(), ProjectError> {
    let violations = validate(song, manifest);
    if violations.is_empty() {
        return Ok(());
    }
    Err(refusal(root, &violations))
}

/// Collapses a refusal into the single error `commit` reports.
///
/// The library API keeps one error type; the tool API keeps all of them (ADR 0006 §2). The
/// first violation's `rule` is carried through rather than replaced with a generic one, so a
/// caller matching on `version_not_writable` still sees it.
fn refusal(root: &Path, violations: &[Violation]) -> ProjectError {
    let Some(first) = violations.first() else {
        return err(root.join(SONG), "song_invalid", "refused with no reason given");
    };
    if violations.len() == 1 {
        return err(root.join(SONG), first.rule, first.message.clone());
    }
    let summary: Vec<String> =
        violations.iter().take(8).map(|v| format!("{} [{}]", v.path, v.rule)).collect();
    err(
        root.join(SONG),
        first.rule,
        format!("{} violation(s): {}", violations.len(), summary.join(", ")),
    )
}

/// The name an asset has in `assets/`: its SHA-256, lowercase hex (§10).
///
/// One hasher in the system. The engine does not compute this — `juce::SHA256` lives in a
/// module Tracktion does not link, and an engine that wrote into the project would be on the
/// wrong side of CLAUDE.md #6 — so what `AudioClip.asset_hash` means is decided here and
/// nowhere else (ADR 0011, Consequences).
pub fn asset_hash(content: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(content).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read(path: &Path) -> Result<String, ProjectError> {
    std::fs::read_to_string(path).map_err(|e| err(path, "unreadable", e.to_string()))
}

/// Writes through a temporary file in the same directory, then renames.
///
/// A rename within one filesystem is atomic, so a reader sees either the previous file or the
/// new one, never a half-written one — and a torn `song.json` is data loss.
///
/// `ponytail:` no `fsync`, so a power cut can still lose the tail of a write the OS had not
/// flushed. Add one here if that ever matters more than write latency.
fn write_atomically(path: &Path, contents: impl AsRef<[u8]>) -> Result<(), ProjectError> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tmp");
    let temporary = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&temporary, contents).map_err(|e| err(&temporary, "unwritable", e.to_string()))?;
    std::fs::rename(&temporary, path).map_err(|e| err(path, "unwritable", e.to_string()))
}
