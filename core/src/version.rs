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

use crate::validate::Violation;
use serde_json::{Map, Value};

/// An entity in the sense ADR 0002 §2 gives the word: it carries its own id and its own
/// version.
///
/// Both halves matter. `PluginRef` has a `version`, but it is a `string` naming a plugin
/// release and it has no `id` — keying on the field name alone would make
/// `set_track_instrument` unable to pin a plugin version, which is the tool's entire purpose.
/// `TempoEvent`, `TimeSignatureEvent` and `AutomationPoint` carry an id and no version
/// (ADR 0002 §2), so they fail the other half and are skipped by construction.
pub fn is_entity(map: &Map<String, Value>) -> bool {
    map.get("id").is_some_and(Value::is_string)
        && map.get("version").is_some_and(Value::is_number)
}

/// Rewrites every entity `version` in `patched` according to ADR 0005 §2, and reports every
/// place a caller asked for a different number than core computed (ADR 0005 §3).
///
/// One mechanism rather than two, because "core owns `version`" and "a caller may not write
/// it" are the same statement. A guard that inspected operation *paths* could not see a
/// version arriving inside a whole-entity value, could not tell `"1"` from `1` — the proto3
/// JSON leniency that made the log disagree with `song.json` in M0.2 — and refused the API's
/// own output, since a patch `ToolResult` returns carries the bumps it caused. Comparing what
/// the caller ended up asking for against what the rule computed catches all four cases and
/// lets a previewed patch be applied unchanged, which is what §9 needs.
///
/// A new entity is exempt: it has no number a client could be holding, and tools build one at
/// 0 for the rule to take to 1.
pub fn bump_versions(before: &Value, patched: &mut Value) -> Vec<Violation> {
    let mut disputed = Vec::new();
    walk(Some(before), patched, "", &mut disputed);
    disputed
}

/// Returns whether this subtree changed, and fixes up versions on the way back up.
///
/// The returned flag deliberately ignores entity `version` members, so a child's own bump is
/// invisible to its parent. Ancestors still bump — a child only bumps because its *content*
/// changed, and that change is what the parent sees.
fn walk(
    before: Option<&Value>,
    patched: &mut Value,
    at: &str,
    disputed: &mut Vec<Violation>,
) -> bool {
    let Value::Object(map) = patched else {
        return before != Some(&*patched);
    };
    let before_map = before.and_then(Value::as_object);
    // An object that *was* an entity still is one, whatever arrived in its place. Keying only
    // on the patched value would let a caller drop `version`, or send it as the string `"1"`
    // that proto3 JSON accepts for a `uint32`, and so choose the number core owns.
    let entity = is_entity(map) || before_map.is_some_and(is_entity);

    // A node that was absent, or was not an object, counts as changed in full.
    let mut changed = before_map.is_none();

    if let Some(was) = before_map {
        // Removals are changes too: a deleted track is a change to `Song`.
        changed |= was.keys().any(|k| !map.contains_key(k) && !(entity && k == "version"));
    }

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        if entity && key == "version" {
            continue;
        }
        let deeper = format!("{at}/{}", crate::patch::escape_token(&key));
        let was = before_map.and_then(|m| m.get(&key)).cloned();
        let value = map.get_mut(&key).expect("the key came from this map");
        changed |= walk(was.as_ref(), value, &deeper, disputed);
    }

    if entity {
        // A different `id` in the same place is a *different entity*, not an edit to this one:
        // `set_track_instrument` replaces an instrument wholesale. The number the old one had
        // belongs to something that no longer exists, so the new entity starts fresh and the
        // caller is disputing nothing.
        let same_entity = before_map.and_then(|m| m.get("id")) == map.get("id");
        let held = before_map.and_then(|m| m.get("version")).filter(|_| same_entity);
        let asked = map.get("version").cloned();
        let was = held.and_then(Value::as_u64);
        // Anything that is not a number counts as 0, so a version spelled as a string or left
        // out entirely is replaced rather than taken at face value.
        let now = map.get("version").and_then(Value::as_u64).unwrap_or(0);
        // Two answers, because two callers arrive here. An ordinary edit bumps by one; a merge
        // brings the other branch's number in the value and resolves to `max + 1` (ADR 0001
        // §4). Preferring the ordinary answer when the caller already stated it is what lets a
        // patch this API returned be applied unchanged — otherwise saying the right number
        // pushes it one higher, and the round trip §9 needs is impossible by construction.
        let ordinary = was.unwrap_or(0) + 1;
        let resolved = was.unwrap_or(0).max(now) + 1;
        let next = if !changed {
            was.unwrap_or(now)
        } else if asked == Some(Value::from(ordinary)) {
            ordinary
        } else {
            resolved
        };

        // The caller asked for something only if the number changed under their hand, and only
        // an entity that already existed has a number anyone could be holding.
        if let Some(held) = held {
            if asked.as_ref() != Some(held) && asked != Some(Value::from(next)) {
                disputed.push(Violation {
                    path: format!("{at}/version"),
                    rule: "version_not_writable",
                    message: format!(
                        "`version` is maintained by core and is never written by a tool op \
                         (ADR 0005 §3); this asked for {} where core computes {next}",
                        asked.unwrap_or(Value::Null)
                    ),
                });
            }
        }
        map.insert("version".to_string(), Value::from(next));
    }

    changed
}
