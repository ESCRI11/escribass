//! The `.escri` project directory (§10, ADR 0004).
//!
//! The five members §10 names, and nothing else:
//!
//! ```text
//! <name>.escri/
//!   song.json    the canonical model — a derived cache of patches/ (ADR 0004)
//!   patches/     one <entry-id>.json per patch entry, append-only
//!   refs.json    branch pointers and HEAD
//!   assets/      content-addressed; nothing produces one before M1
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
use crate::patch::{apply, Op};
use crate::validate::{validate, Violation};
use crate::version::bump_versions;
use escribass_schema::song::{Author, Provenance};
use crate::history::{
    check_refs, entry_from_json, entry_to_json, refs_from_json, refs_to_json, History,
};
use crate::patch::diff;
use crate::history::entry as new_entry;
use escribass_schema::song::Song;
use escribass_schema::SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const SONG: &str = "song.json";
const PATCHES: &str = "patches";
const REFS: &str = "refs.json";
const ASSETS: &str = "assets";
const LOCK: &str = "lock.json";

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

/// `lock.json`. One field until there is something else to pin — plugins, models and
/// compiled artefacts arrive at M1 and M4 (ADR 0003 §3, §17).
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lock {
    schema_version: u32,
}

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

/// An open project directory.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    root: PathBuf,
    song: Song,
    history: History,
}

impl Project {
    /// Assembles a project in memory. Writes nothing.
    pub fn new(root: impl Into<PathBuf>, song: Song, history: History) -> Self {
        Self { root: root.into(), song, history }
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
    ) -> Result<Project, ProjectError> {
        let root = root.into();
        if root.join(SONG).exists() {
            return Err(err(root.join(SONG), "project_exists", "a project is already here"));
        }
        refuse_if_invalid(&root, song)?;

        let empty = serde_json::to_value(Song::default()).expect("a default Song serialises");
        let full = serde_json::to_value(song).expect("a Song serialises");

        let id = ids.next_id();
        let mut history = History::new();
        history
            .append(new_entry(id.clone(), vec![], "create", &diff(&empty, &full),
                authorship(Author::Human, clock), SCHEMA_VERSION))
            .map_err(|e| err(&root, e.rule, e.message))?;
        history.create_ref("main", &id).map_err(|e| err(&root, e.rule, e.message))?;
        history.set_head("main").map_err(|e| err(&root, e.rule, e.message))?;

        let project = Project { root, song: song.clone(), history };
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
        let prepared = self.prepare(ops).map_err(|v| refusal(&self.root, &v))?;
        let Some(head) = self.history.head_id().map(str::to_string) else {
            return Err(err(self.root.join(REFS), "head_unset", "HEAD names no entry"));
        };
        self.record(prepared, tool, vec![head], author, ids, clock)
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
    pub fn prepare(&self, ops: &[Op]) -> Result<Prepared, Vec<Violation>> {
        self.prepare_inner(ops, true)
    }

    /// [`prepare`](Self::prepare) for a merge, which is the one caller whose operations
    /// legitimately carry an entity `version`.
    ///
    /// A merge's patch is the diff between two states core itself produced, so the versions in
    /// it are core's own — the incoming branch's numbers, which ADR 0005 §2 needs in order to
    /// resolve to `max(ours, theirs) + 1`. Refusing them here would silently discard the other
    /// branch's count and let a client's version go backwards, which is what ADR 0005 §4
    /// rejected the rewind for.
    pub fn prepare_merge(&self, ops: &[Op]) -> Result<Prepared, Vec<Violation>> {
        self.prepare_inner(ops, false)
    }

    fn prepare_inner(&self, ops: &[Op], guard_version: bool) -> Result<Prepared, Vec<Violation>> {
        let before = serde_json::to_value(&self.song).expect("a Song serialises");

        let mut patched = apply(&before, ops).map_err(|e| {
            vec![Violation { path: e.path, rule: e.rule, message: e.message }]
        })?;
        let disputed = bump_versions(&before, &mut patched, !guard_version);
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

        let violations = validate(&song);
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
    pub fn record(
        &mut self,
        prepared: Prepared,
        tool: &str,
        parents: Vec<String>,
        author: Author,
        ids: &mut dyn IdSource,
        clock: &dyn Clock,
    ) -> Result<String, ProjectError> {
        let branch = self.history.refs().head.clone();
        let id = ids.next_id();
        let entry = new_entry(id.clone(), parents, tool, &prepared.ops,
            authorship(author, clock), SCHEMA_VERSION);

        // The next state is built beside this one and only swapped in once it is on disk.
        // Mutating first and writing second left a failed write with the session one commit
        // ahead of the directory: the caller is told nothing happened, and every later call
        // builds on state the disk never saw (ADR 0004's ordering is about files; this is the
        // same argument about memory).
        let mut history = self.history.clone();
        history.append(entry).map_err(|e| err(&self.root, e.rule, e.message))?;
        history.advance(&branch, &id).map_err(|e| err(&self.root, e.rule, e.message))?;

        let next = Project { root: self.root.clone(), song: prepared.song, history };
        next.write()?;
        *self = next;
        Ok(id)
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

    /// Writes a proposed state and adopts it only once the write succeeded.
    fn swap(&mut self, song: Song, history: History) -> Result<(), ProjectError> {
        let next = Project { root: self.root.clone(), song, history };
        next.write()?;
        *self = next;
        Ok(())
    }

    /// Reads a project directory, verifying that `song.json` matches a replay of the log.
    pub fn open(root: impl AsRef<Path>) -> Result<Project, ProjectError> {
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

        let project = Project { root, song, history };
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

    /// Writes the whole project.
    ///
    /// Order is ADR 0004's: entries first, since an unreferenced entry is inert; `song.json`
    /// next; `refs.json` last, so advancing the ref is the moment the write becomes real. A
    /// crash before that leaves an orphan entry, which `open` reports rather than mistaking
    /// for history.
    pub fn write(&self) -> Result<(), ProjectError> {
        for directory in [&self.root, &self.root.join(PATCHES), &self.root.join(ASSETS)] {
            std::fs::create_dir_all(directory)
                .map_err(|e| err(directory, "unwritable", e.to_string()))?;
        }

        let lock = serde_json::to_string_pretty(&Lock { schema_version: SCHEMA_VERSION })
            .expect("the lock serialises")
            + "\n";
        write_atomically(&self.root.join(LOCK), &lock)?;

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

/// Provenance for a commit. `created_at` comes from the injected clock, never
/// `SystemTime::now` (§11). The model fields stay unset until M3 produces one.
fn authorship(author: Author, clock: &dyn Clock) -> Provenance {
    Provenance {
        author: author as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(clock.now()),
    }
}

/// §5: every mutation is validated before it is applied, so an invalid song never reaches
/// the log. Reports every violation, not the first (§6 allows three retries).
fn refuse_if_invalid(root: &Path, song: &Song) -> Result<(), ProjectError> {
    let violations = validate(song);
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
fn write_atomically(path: &Path, contents: &str) -> Result<(), ProjectError> {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tmp");
    let temporary = path.with_file_name(format!(".{name}.tmp"));
    std::fs::write(&temporary, contents).map_err(|e| err(&temporary, "unwritable", e.to_string()))?;
    std::fs::rename(&temporary, path).map_err(|e| err(path, "unwritable", e.to_string()))
}
