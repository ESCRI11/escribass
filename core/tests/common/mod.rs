//! Shared by the integration tests: the build manifest they validate against.
//!
//! Not a test target of its own — `tests/common/mod.rs` is a module, not a suite — so this
//! file holds only what more than one suite needs and cannot drift between them.

use escribass_core::Manifest;
use std::sync::Arc;

/// The manifest fixture (`/tests/fixtures/manifest.json`, `tests/AGENTS.md`).
///
/// The real manifest is generated at build time and never committed (ADR 0010 §4), so the
/// tests read a **committed subset of a real one**: the plugin id and the parameter ids the
/// fixtures use, taken from `escribass_engine --scan` of a built engine, with the machine's
/// plugin paths dropped. It is a fixture, not a build artefact — which is why it can be
/// committed and the manifest cannot.
///
/// What keeps it honest is that it is a *subset*: PR 11's render suite, which has a built
/// engine, asserts every id in here appears in the manifest that build wrote. That is the
/// guard against it drifting back into the invented ids this fixture replaced.
///
/// Reading it rather than building one in Rust is deliberate: the parse path is what the
/// binaries use, so a manifest this crate cannot read fails here rather than at startup.
pub fn manifest() -> Arc<Manifest> {
    const AT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/manifest.json");
    Arc::new(Manifest::read(AT).expect("the manifest fixture is readable"))
}
