//! RFC 6902 JSON Patch, and the RFC 6901 pointers it addresses with.
//!
//! §5 makes every mutation a JSON Patch against the canonical JSON, so this is the only way
//! song state ever changes. It operates on `serde_json::Value` and knows nothing about
//! `Song`: applying a patch and deciding whether the result is a valid song are separate
//! jobs, and `validate` does the second.
//!
//! **The document has no arrays.** `song.proto` has no `repeated` fields — every collection
//! is a map keyed by entity id (ADR 0001 §3) — so array index shifting, the `-` append token
//! and array-move detection are all absent by construction. Rather than implement handling
//! that cannot be reached, an array encountered while walking is an explicit error naming
//! the rule that made it impossible.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One RFC 6902 operation.
///
/// Tagged by `op`, so the structural rules — which member each operation requires — come
/// from the deserializer rather than a hand-written match on a string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase", deny_unknown_fields)]
pub enum Op {
    Add { path: String, value: Value },
    Remove { path: String },
    Replace { path: String, value: Value },
    Move { from: String, path: String },
    Copy { from: String, path: String },
    Test { path: String, value: Value },
}

impl Op {
    /// The location this operation acts on.
    pub fn path(&self) -> &str {
        match self {
            Op::Add { path, .. }
            | Op::Remove { path }
            | Op::Replace { path, .. }
            | Op::Move { path, .. }
            | Op::Copy { path, .. }
            | Op::Test { path, .. } => path,
        }
    }
}

/// A patch that could not be applied.
///
/// Carries `path`, `rule` and `message` like [`crate::Violation`], so a caller — including
/// the AI orchestrator, which matches on stable rule ids — handles both the same way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchError {
    /// Index of the failing operation in the patch.
    pub index: usize,
    /// RFC 6901 pointer to the location that failed.
    pub path: String,
    /// Stable machine-readable rule id, e.g. `"path_not_found"`.
    pub rule: &'static str,
    pub message: String,
}

impl std::fmt::Display for PatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = if self.path.is_empty() { "<root>" } else { &self.path };
        write!(f, "op {}: {} [{}]: {}", self.index, path, self.rule, self.message)
    }
}

impl std::error::Error for PatchError {}

/// Applies a patch, returning a new document.
///
/// RFC 6902 requires a patch be all-or-nothing. That holds by construction here: the input is
/// never touched, and an error means the caller keeps the document it already had.
pub fn apply(doc: &Value, ops: &[Op]) -> Result<Value, PatchError> {
    let mut out = doc.clone();
    for (index, op) in ops.iter().enumerate() {
        apply_one(&mut out, op).map_err(|(path, rule, message)| PatchError {
            index,
            path,
            rule,
            message,
        })?;
    }
    Ok(out)
}

type OpFailure = (String, &'static str, String);

fn fail(path: &str, rule: &'static str, message: impl Into<String>) -> OpFailure {
    (path.to_string(), rule, message.into())
}

fn apply_one(doc: &mut Value, op: &Op) -> Result<(), OpFailure> {
    match op {
        Op::Add { path, value } => set(doc, path, value.clone()),
        Op::Replace { path, value } => {
            // `replace` differs from `add` only in requiring the target to exist.
            resolve(doc, path)?;
            set(doc, path, value.clone())
        }
        Op::Remove { path } => remove(doc, path).map(|_| ()),
        Op::Test { path, value } => {
            let found = resolve(doc, path)?;
            if found == value {
                Ok(())
            } else {
                Err(fail(path, "test_failed", format!("expected {value}, found {found}")))
            }
        }
        // `ponytail:` move and copy are implemented but nothing here emits them — `diff`
        // produces add/remove/replace only. They stay because §5 specifies RFC 6902 [MUST],
        // and a partial implementation of a named standard is the failure that looks correct
        // until somebody hands us a conforming patch. Delete them only with the [MUST].
        Op::Copy { from, path } => {
            let taken = resolve(doc, from)?.clone();
            set(doc, path, taken)
        }
        Op::Move { from, path } => {
            // RFC 6902: the source may not be a proper prefix of the destination, or the
            // operation would move a value inside itself.
            if path.starts_with(from) && path.len() > from.len() && !from.is_empty() {
                return Err(fail(path, "move_into_self", format!("`{path}` is inside `{from}`")));
            }
            let taken = remove(doc, from)?;
            set(doc, path, taken)
        }
    }
}

/// Splits an RFC 6901 pointer into unescaped reference tokens.
fn tokens(pointer: &str) -> Result<Vec<String>, OpFailure> {
    if pointer.is_empty() {
        return Ok(Vec::new());
    }
    if !pointer.starts_with('/') {
        return Err(fail(
            pointer,
            "pointer_invalid",
            "an RFC 6901 pointer is empty or starts with `/`",
        ));
    }
    Ok(pointer[1..].split('/').map(unescape_token).collect())
}

/// RFC 6901: `~1` becomes `/` and `~0` becomes `~`, **in that order** — the reverse would
/// turn `~01` into `/` instead of `~1`.
fn unescape_token(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

/// RFC 6901 escaping, the inverse of [`unescape_token`]. Used to build error paths.
pub(crate) fn escape_token(token: &str) -> String {
    if token.contains(['~', '/']) {
        token.replace('~', "~0").replace('/', "~1")
    } else {
        token.to_string()
    }
}

/// Borrows the value a pointer addresses.
fn resolve<'a>(doc: &'a Value, pointer: &str) -> Result<&'a Value, OpFailure> {
    let path = tokens(pointer)?;
    let mut here = doc;
    for (depth, token) in path.iter().enumerate() {
        here = child(here, token, pointer, depth)?;
    }
    Ok(here)
}

fn child<'a>(
    parent: &'a Value,
    token: &str,
    pointer: &str,
    depth: usize,
) -> Result<&'a Value, OpFailure> {
    match parent {
        Value::Object(fields) => fields.get(token).ok_or_else(|| {
            fail(pointer, "path_not_found", format!("`{token}` is not present"))
        }),
        Value::Array(_) => Err(array_error(pointer, depth)),
        _ => Err(fail(
            pointer,
            "path_not_found",
            format!("`{token}` cannot be read: its parent is not an object"),
        )),
    }
}

fn array_error(pointer: &str, depth: usize) -> OpFailure {
    (
        pointer.to_string(),
        "array_in_document",
        format!(
            "an array at depth {depth}: the song document has none, because every collection \
             is a map keyed by entity id (ADR 0001 §3)"
        ),
    )
}

/// The object a pointer's parent addresses, plus the final token.
fn parent_of<'a>(
    doc: &'a mut Value,
    pointer: &str,
) -> Result<(&'a mut Map<String, Value>, String), OpFailure> {
    let path = tokens(pointer)?;
    let Some((last, parents)) = path.split_last() else {
        return Err(fail(pointer, "root_not_addressable", "the root has no parent"));
    };

    let mut here = doc;
    for (depth, token) in parents.iter().enumerate() {
        here = match here {
            Value::Object(fields) => fields.get_mut(token).ok_or_else(|| {
                fail(pointer, "parent_not_found", format!("`{token}` is not present"))
            })?,
            Value::Array(_) => return Err(array_error(pointer, depth)),
            _ => {
                return Err(fail(
                    pointer,
                    "parent_not_found",
                    format!("`{token}` is not an object"),
                ))
            }
        };
    }

    match here {
        Value::Object(fields) => Ok((fields, last.clone())),
        Value::Array(_) => Err(array_error(pointer, parents.len())),
        _ => Err(fail(pointer, "parent_not_found", "the parent is not an object")),
    }
}

/// Inserts or overwrites the value a pointer addresses.
fn set(doc: &mut Value, pointer: &str, value: Value) -> Result<(), OpFailure> {
    if pointer.is_empty() {
        *doc = value;
        return Ok(());
    }
    let (parent, key) = parent_of(doc, pointer)?;
    parent.insert(key, value);
    Ok(())
}

/// Removes the value a pointer addresses and returns it.
fn remove(doc: &mut Value, pointer: &str) -> Result<Value, OpFailure> {
    if pointer.is_empty() {
        return Err(fail(pointer, "remove_root", "the whole document cannot be removed"));
    }
    let (parent, key) = parent_of(doc, pointer)?;
    parent
        .remove(&key)
        .ok_or_else(|| fail(pointer, "path_not_found", format!("`{key}` is not present")))
}
