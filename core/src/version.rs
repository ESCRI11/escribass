//! The two §4.3 fields core owns: entity `version` (ADR 0005 §1–§3) and entity `provenance`
//! (ADR 0021 §1).
//!
//! `version` is core's, never a tool's (ADR 0001 §4). The interesting part is *where* the bump
//! happens: `Project::commit` records `diff(before, after)`, so anything changed after that
//! diff is taken lives in `song.json` and in nothing else, and every replay of the log comes
//! out one version behind the file beside it. The bump therefore runs on the patched document
//! *before* it is re-deserialised, which puts the resulting `replace /…/version` operations
//! inside the entry that gets written. [`stamp_provenance`] is a second walk over the same
//! shape, for the same reason and in the same position, and [`restore_versions`] is a third —
//! the one that *undoes* a bump, so a proposal's per-call scaffolding never reaches the log
//! (ADR 0019 §2). All three are here together because "core owns this field and a caller may
//! not choose it" is one statement made three times.
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
pub fn bump_versions(before: &Value, patched: &mut Value, merging: bool) -> Vec<Violation> {
    let mut disputed = Vec::new();
    walk(Some(before), patched, "", merging, &mut disputed);
    disputed
}

/// Decides every entity's `provenance` in `patched`: an entity that did not exist before gets
/// `made`, an entity that existed keeps the one it had (ADR 0021 §1).
///
/// An **overwrite and not a refusal**, which is the half that took a decision. `created_at`
/// comes from the clock the call runs under, so a guard that compared what a caller sent
/// against what core would write would refuse every previewed patch re-applied — §9's
/// approve-then-apply sends the dry run's patch back verbatim — and, ignoring `created_at`,
/// would still refuse Edit, where a person applies a patch whose new entities carry the
/// *model's* provenance and should get hers. There is nothing a caller can legitimately be
/// disputing, because provenance was never the caller's to state; this is ADR 0006 §4 on the
/// one path that had not enforced it.
///
/// **Ids are not touched.** A dry run mints real ids from a fork and those ids are the keys a
/// pending edit is applied by (ADR 0012 §4), so a caller's ids are its own; what guards them is
/// the validator's `id_not_ulid` and `key_id_mismatch`.
///
/// `made` arrives already serialised because it is one value for the whole call and there are
/// hundreds of entities in a song.
pub fn stamp_provenance(before: &Value, patched: &mut Value, made: &Value) {
    stamp(Some(before), patched, made);
}

/// Puts every entity `version` in `proposed` back to the number `before` holds for it — 0 for
/// an entity `before` does not have, which is what a tool builds a new one at (ADR 0019 §2).
///
/// The proposal's scaffolding, removed. A track the model edited three times stands at
/// `before + 3` on the fork, because each call ran the same `prepare` and each `prepare`
/// bumped; what a person approves is **one** change, so the log must record `before + 1`.
/// Rather than computing that, the numbers are put back where they started and the single
/// `prepare` that follows computes exactly what one call producing this document would have —
/// which is the whole of decision 2's "with `version`s as one `prepare` would compute them",
/// and is why the proposal does not commit through `prepare_merge`: `max(ours, theirs) + 1`
/// would take that track to `before + 4` and dispute nothing (ADR 0005 §2, §3).
///
/// The third walk over [`is_entity`]'s shape, beside [`bump_versions`] and
/// [`stamp_provenance`], and here for their reason: "core owns `version`" is one statement,
/// and the place that undoes a bump belongs beside the place that makes one.
pub fn restore_versions(before: &Value, proposed: &mut Value) {
    restore(Some(before), proposed);
}

fn restore(before: Option<&Value>, proposed: &mut Value) {
    let Value::Object(map) = proposed else {
        return;
    };
    let before_map = before.and_then(Value::as_object);
    // The version rule's reading of "entity", for its reason: an object that *was* one still
    // is one, whatever arrived in its place.
    let entity = is_entity(map) || before_map.is_some_and(is_entity);

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        let was = before_map.and_then(|m| m.get(&key));
        restore(was, map.get_mut(&key).expect("the key came from this map"));
    }

    if !entity {
        return;
    }
    // A different `id` in the same place is a different entity, as it is for the other two
    // walks: the number the old one had belongs to something that no longer exists.
    let held = before_map
        .filter(|was| was.get("id") == map.get("id"))
        .and_then(|was| was.get("version"))
        .cloned();
    map.insert("version".to_string(), held.unwrap_or_else(|| Value::from(0)));
}

fn stamp(before: Option<&Value>, patched: &mut Value, made: &Value) {
    let Value::Object(map) = patched else {
        return;
    };
    let before_map = before.and_then(Value::as_object);
    // The version rule's reading of "entity", and for its reason: an object that *was* one
    // still is one, whatever arrived in its place.
    let entity = is_entity(map) || before_map.is_some_and(is_entity);

    let keys: Vec<String> = map.keys().cloned().collect();
    for key in keys {
        // No clone of the subtree, unlike `walk` below: `before` and `patched` are two values,
        // so the shared borrow of one and the mutable borrow of the other do not meet.
        let was = before_map.and_then(|m| m.get(&key));
        stamp(was, map.get_mut(&key).expect("the key came from this map"), made);
    }

    if !entity {
        return;
    }
    // A different `id` in the same place is a *different entity*, not an edit to this one —
    // `set_track_instrument` replaces an instrument wholesale — so it is new here.
    match before_map.filter(|was| was.get("id") == map.get("id")).map(|was| was.get("provenance")) {
        // It existed: an edit does not change who created a thing. What records this call's
        // author is the entry (ADR 0001 §2).
        Some(Some(had)) => {
            map.insert("provenance".to_string(), had.clone());
        }
        // It existed and recorded nothing — only reachable on a document the validator would
        // already refuse (`message_missing`). Keeping its own means keeping none: filling one
        // in here would invent a creation nothing witnessed, which is the forgery this walk
        // exists to stop, arriving from the other side.
        Some(None) => {
            map.remove("provenance");
        }
        None => {
            map.insert("provenance".to_string(), made.clone());
        }
    }
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
    merging: bool,
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
        changed |= walk(was.as_ref(), value, &deeper, merging, disputed);
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

        // Two callers, two rules, stated separately rather than as one heuristic that has to
        // serve both. An ordinary edit bumps by one and ignores whatever number arrived in the
        // value; a merge resolves to `max(ours, theirs) + 1`, which is ADR 0001 §4's rule and
        // is what keeps a client from ever being handed a number it has already seen.
        //
        // These were one branch, distinguished by whether the caller had already stated the
        // ordinary answer. That collapsed silently when the other branch happened to be exactly
        // one ahead: `max(L, L+1) + 1` should be `L+2`, and the shortcut returned `L+1` — a
        // number the other branch had already handed out for different content.
        let next = match (changed, merging) {
            (false, _) => was.unwrap_or(now),
            (true, false) => was.unwrap_or(0).saturating_add(1),
            (true, true) => was.unwrap_or(0).max(now).saturating_add(1),
        };

        // Only an ordinary edit can dispute: a merge's numbers are core's own, arriving from
        // the other branch (ADR 0005 §2).
        if !merging {
            if let Some(held) = held {
                if asked.as_ref() != Some(held) && asked != Some(Value::from(next)) {
                    disputed.push(Violation {
                        path: format!("{at}/version"),
                        rule: "version_not_writable",
                        message: format!(
                            "`version` is maintained by core and is never written by a tool op \
                             (ADR 0005 §3); this asked for {} where core computes {next}",
                            asked.clone().unwrap_or(Value::Null)
                        ),
                    });
                }
            }
        }
        map.insert("version".to_string(), Value::from(next));
    }

    changed
}
