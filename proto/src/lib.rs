//! Generated Rust types for the escribass tool API.
//!
//! Every type here is generated from `proto/song_tools.proto` by `proto/codegen.sh` into
//! `proto/gen/rust/`, which that script deletes and rewrites on every run. Nothing in this
//! crate is written by hand except this module tree.
//!
//! **There is no model type in this crate.** A request that carries a whole entity carries the
//! one from `song.proto`, resolved through `extern_path` in `proto/buf.gen.yaml` to
//! `escribass_schema` — so `AddClipRequest.note_clip` *is* `escribass_schema::song::NoteClip`,
//! not a copy of it (CLAUDE.md #1, ADR 0006 §4).
//!
//! The gRPC service stubs are not generated yet: this crate is the wire contract, and the
//! server that speaks it arrives with `tonic` in a later step.

pub mod escribass {
    pub mod tools {
        pub mod v1 {
            // The generated file include!s its own .serde.rs at the end, resolved relative to
            // itself, so it finds its sibling under gen/rust.
            include!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/gen/rust/escribass/tools/v1/escribass.tools.v1.rs"
            ));
        }
    }
}

pub use escribass::tools::v1 as tools;
