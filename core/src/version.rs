//! Entity `version` maintenance (ADR 0005 §1–§3).
//!
//! `version` is core's, never a tool's (ADR 0001 §4). The interesting part is *where* the bump
//! happens: `Project::commit` records `diff(before, after)`, so anything changed after that
//! diff is taken lives in `song.json` and in nothing else, and every replay of the log comes
//! out one version behind the file beside it. The bump therefore runs on the patched document
//! *before* it is re-deserialised, which puts the resulting `replace /…/version` operations
//! inside the entry that gets written.
//!
//! It works on `serde_json::Value` rather than on `Song` because the rule is one statement
//! about a shape — an object with an `id` and a `version` — and expressing it over the typed
//! tree means ten near-identical implementations that fall out of step as entities are added.
//! This is not the pattern `core/AGENTS.md` prohibits: that rule is about *serialising output*
//! through `Value`, whose map alphabetises struct fields. Nothing here serialises.

use crate::patch::Op;
use serde_json::{Map, Value};

/// An entity in the sense ADR 0002 §2 gives the word: it carries its own id and its own
/// version.
///
/// Both halves matter. `PluginRef` has a `version`, but it is a `string` naming a plugin
/// release and it has no `id` — keying on the field name alone would make
/// `set_track_instrument` unable to pin a plugin version, which is the tool's entire purpose.
/// `TempoEvent`, `TimeSignatureEvent` and `AutomationPoint` carry an id and no version
/// (ADR 0002 §2), so they fail the other half and are skipped by construction.
fn is_entity(map: &Map<String, Value>) -> bool {
    map.get("id").is_some_and(Value::is_string)
        && map.get("version").is_some_and(Value::is_number)
}

/// Rewrites every entity `version` in `patched` according to ADR 0005 §2.
///
/// A changed entity goes to `max(before, patched) + 1`; an unchanged one keeps the number it
/// had. `max` rather than `before + 1` is what makes a merge work without a separate rule:
/// the incoming side's ops carry the other branch's number, `before` holds this branch's, and
/// the result is ADR 0001 §4's stated resolution.
pub fn bump_versions(before: &Value, patched: &mut Value) {
    walk(Some(before), patched);
}

/// Returns whether this subtree changed, and fixes up versions on the way back up.
///
/// The returned flag deliberately ignores entity `version` members, so a child's own bump is
/// invisible to its parent. Ancestors still bump — a child only bumps because its *content*
/// changed, and that change is what the parent sees.
fn walk(before: Option<&Value>, patched: &mut Value) -> bool {
    let Value::Object(map) = patched else {
        return before != Some(&*patched);
    };
    let entity = is_entity(map);
    let before_map = before.and_then(Value::as_object);

    // A node that was absent, or was not an object, counts as changed in full.
    let mut changed = before_map.is_none();

    if let Some(was) = before_map {
        // Removals are changes too: a deleted track is a change to `Song`.
        changed |= was.keys().any(|k| !map.contains_key(k) && !(entity && k == "version"));
    }

    for (key, value) in map.iter_mut() {
        if entity && key == "version" {
            continue;
        }
        changed |= walk(before_map.and_then(|m| m.get(key)), value);
    }

    if entity {
        let was = before_map.and_then(|m| m.get("version")).and_then(Value::as_u64);
        let now = map.get("version").and_then(Value::as_u64).unwrap_or(0);
        let next = if changed {
            // A new entity has no previous number; tools construct one with `version: 0`, so
            // this lands it at 1.
            Value::from(was.unwrap_or(0).max(now) + 1)
        } else {
            Value::from(was.unwrap_or(now))
        };
        map.insert("version".to_string(), next);
    }

    changed
}

/// The paths of any operations that try to write an entity's `version` (ADR 0005 §3).
///
/// `bump_versions` would overwrite such a value anyway, which is exactly why this exists:
/// silently discarding what a caller asked for is how a contract stops being one. The typed
/// tools never emit one; `apply_patch` could.
///
/// `ponytail:` an op whose parent does not resolve in `before` — writing `/tracks/x/version`
/// in the same patch that adds `/tracks/x` — is allowed through, because there is nothing yet
/// to classify it against. `bump_versions` still has the last word on the value.
pub fn version_writes<'a>(before: &Value, ops: &'a [Op]) -> Vec<&'a str> {
    ops.iter()
        .map(Op::path)
        .filter(|path| {
            let Some((parent, last)) = path.rsplit_once('/') else { return false };
            last == "version"
                && before
                    .pointer(parent)
                    .and_then(Value::as_object)
                    .is_some_and(is_entity)
        })
        .collect()
}
