//! Three-way merge of two branches (ADR 0001 §4).
//!
//! Merging is why the log is a DAG rather than a list, and the rule is deliberately small:
//! **disjoint paths auto-resolve; the same path is a conflict**. There is no content-aware
//! resolution, no "theirs wins", no line merge. A structured error that names the path is
//! something a person or a model can act on; a silent choice between two edits is a song
//! nobody wrote.
//!
//! The comparison is on RFC 6902 *paths*, and the paths compared are the ones `diff` produces
//! from two documents — never a tool's own ops, which the pipeline discards. `diff` recurses to
//! leaves, so granularity is the diff's rather than any tool's phrasing, and two branches
//! editing different notes in one clip do not collide.
//!
//! `version` is excluded from conflict detection and from nothing else. Both branches bump it
//! on every entity they touch, so including it would make every merge conflict by
//! construction — the failure ADR 0001 §4 anticipated. The values still cross, and ADR 0005
//! §2's `max(ours, theirs) + 1` resolves them.

use crate::patch::Op;
use crate::validate::Violation;
use serde_json::Value;

/// Whether `prefix` addresses `path` or something containing it.
///
/// Whole tokens: `/tracks/a` contains `/tracks/a/name` but not `/tracks/ab`, which is a
/// sibling. The same boundary rule RFC 6902 needs for `move`.
fn contains(prefix: &str, path: &str) -> bool {
    path == prefix
        || (path.len() > prefix.len()
            && path.starts_with(prefix)
            && path.as_bytes()[prefix.len()] == b'/')
}

/// An entity's own `version`, which core maintains and neither branch chose.
///
/// The parent has to be an entity. `PluginRef.version` is a *string* pinning a plugin release
/// (§4.4 requires it pinned), and two branches pinning different versions is exactly the
/// disagreement ADR 0001 §4 says is never resolved by a heuristic.
fn is_version(base: &Value, path: &str) -> bool {
    let Some((parent, last)) = path.rsplit_once('/') else { return false };
    last == "version"
        && base.pointer(parent).and_then(Value::as_object).is_some_and(crate::version::is_entity)
}

/// Where two patches touch the same part of the document.
///
/// Reported as `Violation`s so they arrive in `ToolResult.errors` beside every other refusal:
/// §6's retry loop and the orchestrator branch on `rule`, and a merge conflict is exactly the
/// kind of thing §9 puts in front of a person.
pub fn conflicts(base: &Value, ours: &[Op], theirs: &[Op]) -> Vec<Violation> {
    let mut found = Vec::new();
    for mine in ours.iter().filter(|op| !is_version(base, op.path())) {
        for yours in theirs.iter().filter(|op| !is_version(base, op.path())) {
            if contains(mine.path(), yours.path()) || contains(yours.path(), mine.path()) {
                found.push(Violation {
                    path: yours.path().to_string(),
                    rule: "merge_conflict",
                    message: format!(
                        "both branches changed `{}`: this branch {}, the other {}",
                        if mine.path().len() >= yours.path().len() { yours.path() } else { mine.path() },
                        describe(mine),
                        describe(yours),
                    ),
                });
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

fn describe(op: &Op) -> String {
    match op {
        Op::Add { value, .. } => format!("set `{}` to {value}", op.path()),
        Op::Replace { value, .. } => format!("set `{}` to {value}", op.path()),
        Op::Remove { .. } => format!("removed `{}`", op.path()),
        Op::Copy { from, .. } => format!("copied `{from}` to `{}`", op.path()),
        Op::Move { from, .. } => format!("moved `{from}` to `{}`", op.path()),
        Op::Test { .. } => format!("tested `{}`", op.path()),
    }
}
