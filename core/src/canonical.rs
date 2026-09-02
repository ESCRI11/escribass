//! Canonical JSON persistence (docs/specs.md §4.1, ADR 0002 §4).
//!
//! The canonical form is: proto field names, every no-presence field emitted, map keys
//! sorted, two-space pretty print, one trailing newline. All of that comes from the generated
//! `serde` impls — this module never re-implements them, and in particular never serialises
//! through `serde_json::Value`, whose `Map` is a `BTreeMap` and would sort *struct field*
//! names alphabetically as well as map keys.
//!
//! What this module adds is a guard the generated impls cannot give: **non-finite doubles are
//! rejected**. NaN and the infinities serialise as `null`, and `null` is not a value any field
//! reads back, so one NaN from a plugin parameter would otherwise write a project that cannot
//! be opened.
//!
//! Two related normalisations deliberately live elsewhere, because a writer that silently
//! rewrites values would put the in-memory model and the file out of agreement:
//!
//! - `-0.0` is normalised where values enter, by the tool API, and rejected by the validator.
//! - Timestamp precision is the injectable clock's: it yields milliseconds, so nothing here
//!   truncates.

use escribass_schema::song::Song;
use serde_json::Value;

/// A song that cannot be written as canonical JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    /// A value with no JSON representation, at a JSON Pointer path (RFC 6901).
    ///
    /// In this schema the only way to produce one is a non-finite `double`.
    Unrepresentable { path: String },
}

impl std::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanonicalError::Unrepresentable { path } => {
                let path = if path.is_empty() { "<root>" } else { path };
                write!(
                    f,
                    "{path}: value cannot be written as JSON (a non-finite double: NaN, \
                     infinity or negative infinity)"
                )
            }
        }
    }
}

impl std::error::Error for CanonicalError {}

/// Serialises a song to its canonical JSON form.
///
/// Byte-stable: the same song always produces the same bytes, and parsing the output and
/// serialising it again produces those bytes again (docs/specs.md §11).
pub fn to_canonical_json(song: &Song) -> Result<String, CanonicalError> {
    // A `Value` only to locate anything unrepresentable and name its path. The output below
    // is serialised from the song itself — see the module note on field ordering.
    let probe = serde_json::to_value(song).map_err(|_| CanonicalError::Unrepresentable {
        path: String::new(),
    })?;
    check_representable(&probe, &mut String::new())?;

    let mut text =
        serde_json::to_string_pretty(song).expect("a representable song always serialises");
    text.push('\n');
    Ok(text)
}

/// Parses canonical JSON back into a song.
///
/// Unknown fields are an error rather than silently dropped, so a typo in a hand-edited
/// project file is not mistaken for a default.
pub fn from_canonical_json(text: &str) -> Result<Song, serde_json::Error> {
    serde_json::from_str(text)
}

/// Walks the document for values that cannot be written.
///
/// `path` accumulates an RFC 6901 JSON Pointer, so an error names the offending field rather
/// than the document.
fn check_representable(value: &Value, path: &mut String) -> Result<(), CanonicalError> {
    match value {
        // Nothing in this schema is nullable: an unset `optional` or message field is omitted
        // rather than written as null. A null therefore means the serialiser held a value it
        // could not represent, which here means a non-finite double.
        Value::Null => Err(CanonicalError::Unrepresentable { path: path.clone() }),

        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                let len = push_token(path, &i.to_string());
                check_representable(item, path)?;
                path.truncate(len);
            }
            Ok(())
        }

        Value::Object(fields) => {
            for (key, field) in fields {
                let len = push_token(path, key);
                check_representable(field, path)?;
                path.truncate(len);
            }
            Ok(())
        }

        Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(()),
    }
}

/// Appends one RFC 6901 reference token, returning the length to truncate back to.
fn push_token(path: &mut String, token: &str) -> usize {
    let len = path.len();
    path.push('/');
    // RFC 6901: `~` is `~0`, `/` is `~1`. Keys are ULIDs and field names are identifiers, so
    // this is defensive rather than reachable today.
    if token.contains(['~', '/']) {
        path.push_str(&token.replace('~', "~0").replace('/', "~1"));
    } else {
        path.push_str(token);
    }
    len
}
