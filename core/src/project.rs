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
use crate::history::{
    check_refs, entry_from_json, entry_to_json, refs_from_json, refs_to_json, History,
};
use crate::patch::diff;
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
