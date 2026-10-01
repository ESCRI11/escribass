//! Song model core.
//!
//! Owns everything that reads or writes song state: canonical persistence now; the validator,
//! the patch log and the project store as M0.2 continues. Model types come from
//! `escribass-schema` and are never redefined here (docs/specs.md §4.1).

pub mod assistant;
pub mod call;
pub mod canonical;
pub mod clock;
pub mod descriptor;
pub mod engine;
pub mod generator;
pub mod grpc;
pub mod history;
pub mod id;
pub mod mcp;
pub mod manifest;
pub mod merge;
pub mod patch;
pub mod project;
pub mod render;
pub mod session;
pub mod tools;
pub mod validate;
pub mod version;

pub use assistant::{Assistant, Halt, Health, Sidecar, Turn, TurnEnd, REFUSALS_PER_TURN};
pub use call::{call, Answer, CallError, ErrorKind, IMPLEMENTED, OFFERED};
pub use canonical::{from_canonical_json, to_canonical_json, CanonicalError};
pub use clock::{timestamp_from_ms, Clock, FixedClock, SystemClock};
pub use engine::{Engine, Preview};
pub use generator::{Compiled, NotCompiled, Sandbox};
pub use descriptor::{message_fields, offered_schemas, tool_names, tool_schemas, Field,
    ToolSchema};
pub use history::{check_refs, entry, entry_from_json, entry_to_json, ops_of, ops_text,
    refs_from_json, refs_to_json, History, HistoryError};
pub use id::{IdSource, SeededIds, UlidSource};
pub use patch::{apply, diff, Op, PatchError};
pub use manifest::Manifest;
pub use project::{asset_hash, authorship, Ai, Prepared, Project, ProjectError, ProjectLock,
    Toolchain, Toolchains, DEFAULT_MODEL, DEFAULT_PROVIDER};
pub use render::compile;
pub use mcp::{InArrivalOrder, SongTools};
pub use session::{new_song, Proposal, Session};
pub use validate::{validate, Violation};
pub use version::{bump_versions, restore_versions, stamp_provenance};
