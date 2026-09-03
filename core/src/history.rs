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

    // Two refs differing only by case are indistinguishable to a reader, and would collide if
    // the store ever moved to one file per ref (ADR 0001 §2).
    let mut folded: std::collections::BTreeMap<String, Vec<&String>> = Default::default();
    for name in refs.refs.keys() {
        folded.entry(name.to_ascii_lowercase()).or_default().push(name);
    }
    for (_, names) in folded.iter().filter(|(_, n)| n.len() > 1) {
        for name in names {
            add(
                format!("/refs/{name}"),
                "ref_name_case_collision",
                "two refs may not differ only by case",
            );
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
