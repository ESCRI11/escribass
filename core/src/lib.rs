//! Song model core.
//!
//! Owns everything that reads or writes song state: canonical persistence now; the validator,
//! the patch log and the project store as M0.2 continues. Model types come from
//! `escribass-schema` and are never redefined here (docs/specs.md §4.1).

pub mod canonical;
pub mod clock;
pub mod id;
pub mod patch;
pub mod validate;

pub use canonical::{from_canonical_json, to_canonical_json, CanonicalError};
pub use clock::{timestamp_from_ms, Clock, FixedClock, SystemClock};
pub use id::{IdSource, SeededIds, UlidSource};
pub use patch::{apply, Op, PatchError};
pub use validate::{validate, Violation};
