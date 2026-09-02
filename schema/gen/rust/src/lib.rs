//! Generated Rust types for the escribass song model.
//!
//! Every type here is generated from `schema/*.proto` by `schema/codegen.sh`; nothing in
//! this crate is written by hand except this module tree (docs/specs.md §4.1). The module
//! nesting mirrors the protobuf packages because `history` refers to `song` types through
//! `super::super::song::v1`.
//!
//! Canonical JSON (ADR 0002 §4) comes from the generated serde impls: proto field names,
//! defaults emitted, and `BTreeMap` maps that iterate in sorted key order. Serialise with
//! `serde_json::to_string_pretty`.

pub mod escribass {
    pub mod song {
        pub mod v1 {
            // Each generated file include!s its own .serde.rs at the end.
            include!("escribass/song/v1/escribass.song.v1.rs");
        }
    }
    pub mod history {
        pub mod v1 {
            include!("escribass/history/v1/escribass.history.v1.rs");
        }
    }
}

/// Re-exported so consumers can name the well-known types without adding the dep.
pub use pbjson_types;

pub use escribass::history::v1 as history;
pub use escribass::song::v1 as song;
