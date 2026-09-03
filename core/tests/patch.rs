//! RFC 6902 apply tests.
//!
//! Cases are the RFC's own Appendix A examples where they apply, minus the array ones — the
//! song document has no arrays (ADR 0001 §3) and this module rejects them explicitly.

use escribass_core::{apply, Op, PatchError};
use serde_json::{json, Value};

fn ops(v: Value) -> Vec<Op> {
    serde_json::from_value(v).expect("ops must deserialise")
}

fn err(doc: &Value, patch: Value) -> PatchError {
    apply(doc, &ops(patch)).expect_err("expected the patch to fail")
}

// ---- the six operations ----

#[test]
fn add_inserts_and_overwrites() {
    let doc = json!({"a": {"b": 1}});
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "add", "path": "/a/c", "value": 2}]))).unwrap(),
        json!({"a": {"b": 1, "c": 2}})
    );
    // RFC 6902: add to an existing member replaces it.
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "add", "path": "/a/b", "value": 9}]))).unwrap(),
        json!({"a": {"b": 9}})
    );
}

#[test]
fn remove_deletes_and_requires_the_target() {
    let doc = json!({"a": {"b": 1, "c": 2}});
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "remove", "path": "/a/b"}]))).unwrap(),
        json!({"a": {"c": 2}})
    );
    assert_eq!(err(&doc, json!([{"op": "remove", "path": "/a/z"}])).rule, "path_not_found");
}

#[test]
fn replace_requires_the_target_to_exist() {
    let doc = json!({"a": 1});
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "replace", "path": "/a", "value": 2}]))).unwrap(),
        json!({"a": 2})
    );
    // The one thing separating replace from add.
    assert_eq!(err(&doc, json!([{"op": "replace", "path": "/z", "value": 2}])).rule, "path_not_found");
}

#[test]
fn test_compares_and_reports_both_values() {
    let doc = json!({"a": {"b": 43}});
    apply(&doc, &ops(json!([{"op": "test", "path": "/a/b", "value": 43}]))).unwrap();

    let e = err(&doc, json!([{"op": "test", "path": "/a/b", "value": 44}]));
    assert_eq!(e.rule, "test_failed");
    assert!(e.message.contains("44") && e.message.contains("43"), "{}", e.message);
}

#[test]
fn move_and_copy() {
    let doc = json!({"a": {"b": 1}, "c": {}});
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "move", "from": "/a/b", "path": "/c/b"}]))).unwrap(),
        json!({"a": {}, "c": {"b": 1}})
    );
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "copy", "from": "/a/b", "path": "/c/b"}]))).unwrap(),
        json!({"a": {"b": 1}, "c": {"b": 1}})
    );
    // RFC 6902: the source may not be a proper prefix of the destination.
    assert_eq!(
        err(&doc, json!([{"op": "move", "from": "/a", "path": "/a/inside"}])).rule,
        "move_into_self"
    );
}

// ---- structure, from the deserializer ----

#[test]
fn an_operation_missing_its_required_member_does_not_deserialise() {
    // `remove` has no `value`; `add` must have one. RFC 6902's structural rules come from the
    // tagged enum rather than a hand-written match on the `op` string.
    assert!(serde_json::from_value::<Vec<Op>>(json!([{"op": "add", "path": "/a"}])).is_err());
    assert!(serde_json::from_value::<Vec<Op>>(json!([{"op": "move", "path": "/a"}])).is_err());
    assert!(serde_json::from_value::<Vec<Op>>(json!([{"op": "sing", "path": "/a"}])).is_err());
    assert!(
        serde_json::from_value::<Vec<Op>>(json!([{"op": "remove", "path": "/a", "value": 1}]))
            .is_err(),
        "an unexpected member is rejected, not ignored"
    );
}

#[test]
fn ops_round_trip_through_json_unchanged() {
    let text = r#"[{"op":"replace","path":"/a","value":43},{"op":"remove","path":"/b"}]"#;
    let parsed: Vec<Op> = serde_json::from_str(text).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), text);
}

// ---- RFC 6901 pointers ----

#[test]
fn pointer_escapes_unescape_in_the_right_order() {
    // `~1` before `~0`: the reverse turns `~01` into `/` instead of `~1`.
    let doc = json!({"~1": 1, "a/b": 2, "~": 3});
    assert_eq!(apply(&doc, &ops(json!([{"op": "test", "path": "/~01", "value": 1}]))), Ok(doc.clone()));
    assert_eq!(apply(&doc, &ops(json!([{"op": "test", "path": "/a~1b", "value": 2}]))), Ok(doc.clone()));
    assert_eq!(apply(&doc, &ops(json!([{"op": "test", "path": "/~0", "value": 3}]))), Ok(doc));
}

#[test]
fn the_empty_pointer_addresses_the_whole_document() {
    let doc = json!({"a": 1});
    assert_eq!(
        apply(&doc, &ops(json!([{"op": "replace", "path": "", "value": {"b": 2}}]))).unwrap(),
        json!({"b": 2})
    );
    assert_eq!(err(&doc, json!([{"op": "remove", "path": ""}])).rule, "remove_root");
}

#[test]
fn a_pointer_that_does_not_start_with_a_slash_is_rejected() {
    assert_eq!(
        err(&json!({"a": 1}), json!([{"op": "replace", "path": "a", "value": 2}])).rule,
        "pointer_invalid"
    );
}

// ---- the properties that matter ----

#[test]
fn a_patch_is_all_or_nothing() {
    // RFC 6902 requires it, and returning a new document makes it true by construction: the
    // caller still holds the document it started with.
    let doc = json!({"a": 1, "b": 2});
    let e = err(&doc, json!([
        {"op": "replace", "path": "/a", "value": 9},
        {"op": "replace", "path": "/missing", "value": 9}
    ]));
    assert_eq!(e.index, 1, "the error names the failing operation");
    assert_eq!(doc, json!({"a": 1, "b": 2}), "the input is untouched");
}

#[test]
fn an_array_in_the_document_is_an_explicit_error() {
    // The song document has none, because every collection is a map keyed by entity id.
    // Failing loudly beats implementing index handling that cannot be reached.
    let e = err(&json!({"a": [1, 2]}), json!([{"op": "replace", "path": "/a/0", "value": 9}]));
    assert_eq!(e.rule, "array_in_document");
    assert!(e.message.contains("ADR 0001 §3"), "{}", e.message);
}

#[test]
fn object_keys_stay_sorted() {
    // A canary. `serde_json::Map` is a `BTreeMap` only while nothing enables
    // `serde_json/preserve_order`; feature unification would make it insertion-ordered and
    // silently change the key order of every canonical document and patch file.
    let doc: Value = serde_json::from_str(r#"{"b":1,"a":2}"#).unwrap();
    assert_eq!(serde_json::to_string(&doc).unwrap(), r#"{"a":2,"b":1}"#);

    let out = apply(&doc, &ops(json!([{"op": "add", "path": "/c", "value": 3}]))).unwrap();
    assert_eq!(serde_json::to_string(&out).unwrap(), r#"{"a":2,"b":1,"c":3}"#);
}
