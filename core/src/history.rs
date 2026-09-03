//! The on-disk shape of the patch log: `patches/*.json` and `refs.json`.
//!
//! Serialisation only — the DAG itself, and any filesystem access, live elsewhere. This is a
//! separate module because a shape defect is silent where a layout defect is loud: a file
//! that round-trips cleanly and is still wrong passes every obvious test.
//!
//! **Why the generated serde impl cannot write these files.** `PatchEntry.ops` is `bytes` in
//! `history.proto`, and the generated implementation base64-encodes it:
//!
//! ```text
//! serialize_field("ops", pbjson::private::base64::encode(&self.ops).as_str())
//! ```
//!
//! Writing entries that way produces `"ops": "W3sib3AiOi4uLg=="` — a valid file that
//! round-trips and passes a naive test, while violating ADR 0001 §1, which specifies a
//! literal RFC 6902 array, and §2.6, which requires the project be readable and diffable.
//! So the disk form comes from [`EntryOnDisk`] below, whose `ops` is a real `Vec<Op>`.
//!
//! `Refs` has no such problem — it is strings and a map — so it is written with the
//! generated impl, which already produces proto field names and sorted keys (ADR 0002 §4).
//!
//! [`EntryOnDisk`] is a serialisation adapter, not a second model. History is not song state,
//! so CLAUDE.md #1 and §14.2 are not in play; `PatchEntry` remains the only definition.

use crate::patch::Op;
use crate::validate::Violation;
use escribass_schema::history::{PatchEntry, Refs};
use escribass_schema::song::Provenance;
use serde::{Deserialize, Serialize};

/// A patch entry or refs file that could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryError {
    pub path: String,
    pub rule: &'static str,
    pub message: String,
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} [{}]: {}", self.path, self.rule, self.message)
    }
}

impl std::error::Error for HistoryError {}

fn err(path: &str, rule: &'static str, message: impl Into<String>) -> HistoryError {
    HistoryError { path: path.to_string(), rule, message: message.into() }
}

/// The literal shape of `patches/<id>.json`, in the field order ADR 0001 §1 specifies.
///
/// `provenance` is the generated `Provenance`, so it serialises through the same impl that
/// writes it inside `song.json` — proto field names, defaults emitted, declaration order.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryOnDisk {
    id: String,
    parents: Vec<String>,
    tool: String,
    ops: Vec<Op>,
    provenance: Provenance,
    schema_version: u32,
}

/// Builds an entry whose `ops` hold canonical text, which is the invariant every other
/// function here relies on.
pub fn entry(
    id: impl Into<String>,
    parents: Vec<String>,
    tool: impl Into<String>,
    ops: &[Op],
    provenance: Provenance,
    schema_version: u32,
) -> PatchEntry {
    PatchEntry {
        id: id.into(),
        parents,
        tool: tool.into(),
        ops: ops_text(ops).into_bytes(),
        provenance: Some(provenance),
        schema_version,
    }
}

/// The canonical text of an operations array, standalone.
///
/// This is what `PatchEntry.ops` carries and what the wire carries, so both hold the same
/// canonical JSON document as the file — ADR 0002 §11, as amended. It is not byte-identical
/// to the file's `ops` member, which is indented one level deeper by the surrounding object.
pub fn ops_text(ops: &[Op]) -> String {
    let mut text = serde_json::to_string_pretty(ops).expect("ops always serialise");
    text.push('\n');
    text
}

/// Reads the operations an entry carries.
pub fn ops_of(entry: &PatchEntry) -> Result<Vec<Op>, HistoryError> {
    serde_json::from_slice(&entry.ops).map_err(|e| {
        err(&entry.id, "ops_unreadable", format!("`ops` is not an RFC 6902 array: {e}"))
    })
}

/// Serialises an entry to its `patches/<id>.json` form.
pub fn entry_to_json(entry: &PatchEntry) -> Result<String, HistoryError> {
    let Some(provenance) = entry.provenance.clone() else {
        return Err(err(&entry.id, "provenance_missing", "every entry carries provenance (§4.3)"));
    };
    let on_disk = EntryOnDisk {
        id: entry.id.clone(),
        parents: entry.parents.clone(),
        tool: entry.tool.clone(),
        ops: ops_of(entry)?,
        provenance,
        schema_version: entry.schema_version,
    };
    let mut text = serde_json::to_string_pretty(&on_disk)
        .map_err(|e| err(&entry.id, "entry_unwritable", e.to_string()))?;
    text.push('\n');
    Ok(text)
}

/// Parses a `patches/<id>.json` file.
pub fn entry_from_json(text: &str) -> Result<PatchEntry, HistoryError> {
    let on_disk: EntryOnDisk = serde_json::from_str(text)
        .map_err(|e| err("", "entry_unreadable", format!("not a patch entry: {e}")))?;
    Ok(entry(
        on_disk.id,
        on_disk.parents,
        on_disk.tool,
        &on_disk.ops,
        on_disk.provenance,
        on_disk.schema_version,
    ))
}

/// Serialises `refs.json`. The generated impl already produces the canonical form.
pub fn refs_to_json(refs: &Refs) -> String {
    let mut text = serde_json::to_string_pretty(refs).expect("refs always serialise");
    text.push('\n');
    text
}

/// Parses `refs.json`.
pub fn refs_from_json(text: &str) -> Result<Refs, HistoryError> {
    serde_json::from_str(text).map_err(|e| err("", "refs_unreadable", format!("not refs: {e}")))
}

/// Checks the ref-name rules of ADR 0001 §2.
///
/// Returns `Violation`s rather than its own error type: a caller already handles those from
/// `validate`, and refs are part of what makes a project well-formed.
pub fn check_refs(refs: &Refs) -> Vec<Violation> {
    let mut found = Vec::new();
    let mut add = |path: String, rule: &'static str, message: &str| {
        found.push(Violation { path, rule, message: message.to_string() });
    };

    for name in refs.refs.keys() {
        let at = format!("/refs/{name}");
        if name.is_empty() {
            add(at.clone(), "ref_name_empty", "a ref name is not empty");
            continue;
        }
        // This also carries ADR 0001 §2's "no two refs differing only by case": the charset
        // has no uppercase in it, so two names that fold together are the same name, and a
        // map cannot hold it twice. A separate pass would only ever fire alongside this one.
        if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._/-".contains(c))
        {
            add(at.clone(), "ref_name_charset", "ref names are ASCII [a-z0-9._/-] (ADR 0001 §2)");
        }
        if name.starts_with('/') || name.ends_with('/') {
            add(at.clone(), "ref_name_slash", "a ref name has no leading or trailing slash");
        }
        if name.split('/').any(|part| part == "..") {
            add(at.clone(), "ref_name_dotdot", "a ref name contains no `..` segment");
        }
    }

    if refs.head.is_empty() {
        add("/head".to_string(), "head_unset", "HEAD always names a ref (ADR 0001 §2)");
    } else if !refs.refs.contains_key(&refs.head) {
        add(
            "/head".to_string(),
            "head_dangling",
            "HEAD names a ref that does not exist; there is no detached state",
        );
    }

    found.sort();
    found
}

// ---------------------------------------------------------------------------
// The DAG (ADR 0001 §1, §2)
// ---------------------------------------------------------------------------

use crate::patch::{apply, diff};
use escribass_schema::song::Song;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// The patch DAG and the refs that name positions in it.
///
/// Entries are held outright rather than behind a store trait: there is one writer (§3, §5),
/// entries are a few hundred bytes, and an interface with one implementation would be an
/// abstraction nothing asked for. The project store serialises this same struct.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct History {
    entries: BTreeMap<String, PatchEntry>,
    refs: Refs,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn refs(&self) -> &Refs {
        &self.refs
    }

    pub fn entries(&self) -> &BTreeMap<String, PatchEntry> {
        &self.entries
    }

    pub fn get(&self, id: &str) -> Option<&PatchEntry> {
        self.entries.get(id)
    }

    /// The entry the current ref points at. `None` only before the root exists.
    pub fn head_id(&self) -> Option<&str> {
        self.refs.refs.get(&self.refs.head).map(String::as_str)
    }

    /// Adds an entry. Append-only: an id is never reused and an entry is never rewritten.
    pub fn append(&mut self, entry: PatchEntry) -> Result<(), HistoryError> {
        if self.entries.contains_key(&entry.id) {
            return Err(err(&entry.id, "entry_exists", "the log is append-only; ids are never reused"));
        }
        for parent in &entry.parents {
            if !self.entries.contains_key(parent) {
                return Err(err(
                    &entry.id,
                    "parent_missing",
                    format!("parent `{parent}` is not in the log"),
                ));
            }
        }
        self.entries.insert(entry.id.clone(), entry);
        Ok(())
    }

    /// Names a new position in the log. Branching copies no data (ADR 0001 §2).
    pub fn create_ref(&mut self, name: &str, at: &str) -> Result<(), HistoryError> {
        if self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_exists", "that ref already exists"));
        }
        self.point(name, at)
    }

    /// Moves an existing ref, which is what a commit does to the current branch.
    pub fn advance(&mut self, name: &str, to: &str) -> Result<(), HistoryError> {
        if !self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_missing", "that ref does not exist"));
        }
        self.point(name, to)
    }

    fn point(&mut self, name: &str, at: &str) -> Result<(), HistoryError> {
        if !self.entries.contains_key(at) {
            return Err(err(name, "entry_missing", format!("`{at}` is not in the log")));
        }
        let mut proposed = self.refs.clone();
        proposed.refs.insert(name.to_string(), at.to_string());
        if let Some(bad) = check_refs(&proposed).into_iter().find(|v| v.rule.starts_with("ref_name"))
        {
            return Err(err(name, bad.rule, bad.message));
        }
        self.refs = proposed;
        Ok(())
    }

    /// Discards a branch. Entries stay in the log, unreferenced and inert (ADR 0001 §2).
    pub fn delete_ref(&mut self, name: &str) -> Result<(), HistoryError> {
        if name == self.refs.head {
            return Err(err(name, "delete_head", "HEAD always names a ref; switch away first"));
        }
        self.refs
            .refs
            .remove(name)
            .map(|_| ())
            .ok_or_else(|| err(name, "ref_missing", "that ref does not exist"))
    }

    /// Points `HEAD` at an existing ref. There is no detached state (ADR 0001 §2).
    pub fn set_head(&mut self, name: &str) -> Result<(), HistoryError> {
        if !self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_missing", "HEAD can only name a ref that exists"));
        }
        self.refs.head = name.to_string();
        Ok(())
    }

    /// The entries leading to `id`, in the order they must be replayed.
    ///
    /// A topological sort, because a merge entry has two parents and its ancestors form a DAG
    /// rather than a chain. Ties break on id, and ids are ULIDs, so the order is both
    /// deterministic and chronological — which is what lets a replay be compared byte for
    /// byte (§11).
    pub fn ancestry(&self, id: &str) -> Result<Vec<&PatchEntry>, HistoryError> {
        let mut reachable: BTreeSet<String> = BTreeSet::new();
        let mut pending = vec![id.to_string()];
        while let Some(current) = pending.pop() {
            if !reachable.insert(current.clone()) {
                continue;
            }
            let entry = self
                .entries
                .get(&current)
                .ok_or_else(|| err(id, "entry_missing", format!("`{current}` is not in the log")))?;
            pending.extend(entry.parents.iter().cloned());
        }

        let mut remaining: BTreeMap<&str, usize> = BTreeMap::new();
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for key in &reachable {
            let entry = &self.entries[key];
            remaining.insert(key, entry.parents.iter().filter(|p| reachable.contains(*p)).count());
            for parent in &entry.parents {
                children.entry(parent.as_str()).or_default().push(key);
            }
        }

        let mut ready: BTreeSet<&str> =
            remaining.iter().filter(|(_, n)| **n == 0).map(|(k, _)| *k).collect();
        let mut order = Vec::with_capacity(reachable.len());
        while let Some(next) = ready.iter().next().copied() {
            ready.remove(next);
            order.push(&self.entries[next]);
            for child in children.get(next).into_iter().flatten() {
                let left = remaining.get_mut(child).expect("child is reachable");
                *left -= 1;
                if *left == 0 {
                    ready.insert(child);
                }
            }
        }

        if order.len() != reachable.len() {
            // Only reachable if an entry is its own ancestor, which append refuses to create.
            return Err(err(id, "parent_cycle", "the log contains a cycle and cannot be replayed"));
        }
        Ok(order)
    }

    /// Rebuilds the document at a node by replaying the log (ADR 0004).
    ///
    /// Replay starts from a default `Song`, never from `{}`. The canonical form emits every
    /// no-presence field (ADR 0002 §4), so `replace` is legal against a fresh document from
    /// the first op; starting empty would make the root patch a pile of `add`s that diverge
    /// from what the tool API produces.
    ///
    /// `ponytail:` O(history) per call, as ADR 0001 already accepts. Cache the materialised
    /// document per ref if a large project drags.
    pub fn materialise(&self, id: &str) -> Result<Value, HistoryError> {
        let mut doc = serde_json::to_value(Song::default()).expect("a default Song serialises");
        for entry in self.ancestry(id)? {
            let ops = ops_of(entry)?;
            doc = apply(&doc, &ops).map_err(|e| {
                err(&entry.id, "replay_failed", format!("entry `{}` no longer applies: {e}", entry.id))
            })?;
        }
        Ok(doc)
    }

    /// Moves `HEAD` to another branch, returning the patch that takes the caller's document
    /// to that branch's state.
    ///
    /// **Appends nothing.** Switching is navigation, and history that recorded navigation
    /// would grow every time somebody looked at a branch.
    pub fn switch(&mut self, name: &str, current: &Value) -> Result<Vec<Op>, HistoryError> {
        let target = self
            .refs
            .refs
            .get(name)
            .ok_or_else(|| err(name, "ref_missing", "that ref does not exist"))?
            .clone();
        let ops = diff(current, &self.materialise(&target)?);
        self.refs.head = name.to_string();
        Ok(ops)
    }
}
