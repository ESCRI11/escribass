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

    /// Rebuilds a history from what was read off disk.
    ///
    /// Parents are resolved against the whole set at the end rather than entry by entry as it
    /// goes, because a directory listing has no order to trust. `append` needs each parent to
    /// be present already, which is true while one session mints ids in sequence and stops
    /// being true the moment a log is imported, merged, or written across a clock adjustment:
    /// id order is not ancestry, and treating it as such fails `parent_missing` on a DAG that
    /// is perfectly valid. A log whose parents really do not resolve is reported rather than
    /// partially loaded.
    pub fn from_parts(
        entries: impl IntoIterator<Item = PatchEntry>,
        refs: Refs,
    ) -> Result<Self, HistoryError> {
        let mut log = Self::new();
        for entry in entries {
            if log.entries.contains_key(&entry.id) {
                return Err(err(
                    &entry.id,
                    "entry_exists",
                    "the log is append-only; ids are never reused",
                ));
            }
            log.entries.insert(entry.id.clone(), entry);
        }
        for entry in log.entries.values() {
            for parent in &entry.parents {
                if !log.entries.contains_key(parent) {
                    return Err(err(
                        &entry.id,
                        "parent_missing",
                        format!("parent `{parent}` is not in the log"),
                    ));
                }
            }
        }
        for (name, at) in &refs.refs {
            if !log.entries.contains_key(at) {
                return Err(err(name, "entry_missing", format!("ref points at absent `{at}`")));
            }
        }
        if let Some(bad) = check_refs(&refs).into_iter().next() {
            return Err(err(&bad.path, bad.rule, bad.message));
        }
        log.refs = refs;
        Ok(log)
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
        self.check_create_ref(name, at)?;
        self.point(name, at)
    }

    /// Whether [`create_ref`](Self::create_ref) would succeed, without doing it.
    ///
    /// The mutators here refuse before they change anything, which is enough for a caller that
    /// intends to act. It is not enough for a dry run, which must be able to ask the question
    /// and get an answer rather than a state.
    pub fn check_create_ref(&self, name: &str, at: &str) -> Result<(), HistoryError> {
        if self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_exists", "that ref already exists"));
        }
        self.check_target(name, at)?;
        self.check_name(name, at)
    }

    /// Whether [`delete_ref`](Self::delete_ref) would succeed, without doing it.
    pub fn check_delete_ref(&self, name: &str) -> Result<(), HistoryError> {
        if name == self.refs.head {
            return Err(err(name, "delete_head", "HEAD always names a ref; switch away first"));
        }
        if !self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_missing", "that ref does not exist"));
        }
        Ok(())
    }

    fn check_target(&self, name: &str, at: &str) -> Result<(), HistoryError> {
        if self.entries.contains_key(at) {
            return Ok(());
        }
        Err(err(name, "entry_missing", format!("`{at}` is not in the log")))
    }

    fn check_name(&self, name: &str, at: &str) -> Result<(), HistoryError> {
        let mut proposed = self.refs.clone();
        proposed.refs.insert(name.to_string(), at.to_string());
        match check_refs(&proposed).into_iter().find(|v| v.rule.starts_with("ref_name")) {
            Some(bad) => Err(err(name, bad.rule, bad.message)),
            None => Ok(()),
        }
    }

    /// Moves an existing ref, which is what a commit does to the current branch.
    pub fn advance(&mut self, name: &str, to: &str) -> Result<(), HistoryError> {
        if !self.refs.refs.contains_key(name) {
            return Err(err(name, "ref_missing", "that ref does not exist"));
        }
        self.point(name, to)
    }

    fn point(&mut self, name: &str, at: &str) -> Result<(), HistoryError> {
        self.check_target(name, at)?;
        self.check_name(name, at)?;
        self.refs.refs.insert(name.to_string(), at.to_string());
        Ok(())
    }

    /// Discards a branch. Entries stay in the log, unreferenced and inert (ADR 0001 §2).
    pub fn delete_ref(&mut self, name: &str) -> Result<(), HistoryError> {
        self.check_delete_ref(name)?;
        self.refs.refs.remove(name);
        Ok(())
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

    /// Every entry `id` reaches, including itself.
    pub fn ancestors(&self, id: &str) -> Result<BTreeSet<String>, HistoryError> {
        Ok(self.ancestry(id)?.into_iter().map(|e| e.id.clone()).collect())
    }

    /// The entry two branches last had in common — a merge's base (ADR 0001 §4).
    ///
    /// A DAG can have several common ancestors; the one that matters is the latest, meaning the
    /// one no other common ancestor descends from. When two are equally latest there is no
    /// single base, and picking one would silently choose which of two histories to treat as
    /// the truth. That is reported instead: `merge_base_ambiguous` needs a criss-cross merge to
    /// reach, which M0.3 has no way to create, and a wrong answer here would be invisible.
    pub fn merge_base(&self, ours: &str, theirs: &str) -> Result<String, HistoryError> {
        let mine = self.ancestors(ours)?;
        let yours = self.ancestors(theirs)?;
        let common: BTreeSet<&String> = mine.intersection(&yours).collect();
        if common.is_empty() {
            return Err(err(ours, "merge_unrelated", "the two branches share no history"));
        }

        let mut latest: Vec<String> = Vec::new();
        for candidate in &common {
            let descended_from = common.iter().any(|other| {
                other != candidate
                    && self.ancestors(other).is_ok_and(|a| a.contains(candidate.as_str()))
            });
            if !descended_from {
                latest.push((*candidate).clone());
            }
        }

        match latest.len() {
            1 => Ok(latest.remove(0)),
            _ => Err(err(
                ours,
                "merge_base_ambiguous",
                format!("{} equally recent common ancestors; there is no single base", latest.len()),
            )),
        }
    }

    /// The entries to replay to reach `id`: the **first-parent** chain, root last to first.
    ///
    /// Not every ancestor. A merge entry's ops are the diff from `parents[0]`'s document to the
    /// merged one, so they already carry everything the other side contributed — replaying the
    /// other side's entries as well and then the merge on top applies the same change twice.
    /// For `add` and `replace` that is invisible, because both are idempotent. For `remove` it
    /// is fatal: the incoming entry deletes the path, the merge entry deletes it again, and the
    /// replay stops with `path_not_found` on a log that was written by a merge the tool
    /// accepted. The project then will not reopen.
    ///
    /// `parents[1..]` is lineage: it is what `merge_base` reads and what makes the log a DAG.
    /// It is not a replay path.
    pub fn first_parents(&self, id: &str) -> Result<Vec<&PatchEntry>, HistoryError> {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut at = id.to_string();
        loop {
            if !seen.insert(at.clone()) {
                return Err(err(id, "parent_cycle", "the log contains a cycle and cannot be replayed"));
            }
            let entry = self
                .entries
                .get(&at)
                .ok_or_else(|| err(id, "entry_missing", format!("`{at}` is not in the log")))?;
            chain.push(entry);
            match entry.parents.first() {
                Some(parent) => at = parent.clone(),
                None => break,
            }
        }
        chain.reverse();
        Ok(chain)
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
        for entry in self.first_parents(id)? {
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
        let ops = self.patch_to(name, current)?;
        self.refs.head = name.to_string();
        Ok(ops)
    }

    /// The patch that takes `current` to another branch's state, **without moving `HEAD`**.
    ///
    /// The pure half of [`switch`](Self::switch), and the reason it exists is a dry run. A
    /// preview implemented as "call `switch` and do not write" would still have moved
    /// `refs.head` in memory, and the next commit would land on a branch nobody chose — a
    /// failure with no error, visible only later as history on the wrong ref.
    pub fn patch_to(&self, name: &str, current: &Value) -> Result<Vec<Op>, HistoryError> {
        let target = self
            .refs
            .refs
            .get(name)
            .ok_or_else(|| err(name, "ref_missing", "that ref does not exist"))?;
        Ok(diff(current, &self.materialise(target)?))
    }
}
