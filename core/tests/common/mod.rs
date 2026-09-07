//! Shared by the integration tests: the build manifest they validate against.
//!
//! Not a test target of its own — `tests/common/mod.rs` is a module, not a suite — so this
//! file holds only what more than one suite needs and cannot drift between them.
//!
//! `dead_code` off for the module: it is compiled into every suite that names it, and a helper
//! one suite does not happen to use is the normal case here, not a mistake.
#![allow(dead_code)]

use escribass_core::Manifest;
use std::path::{Path, PathBuf};
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
    Arc::new(Manifest::read(MANIFEST).expect("the manifest fixture is readable"))
}

/// Where it lives, for the tests that need the file rather than the parsed value.
pub const MANIFEST: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/manifest.json");

/// An engine that is a shell script: it records that it ran, keeps the plan it was given, and
/// then does whatever `body` says. Written into `dir`, which must exist.
///
/// Shared by `engine.rs`, which drives it through a `Session`, and by `mcp.rs`, which drives it
/// through a real server process over real pipes (M1 PR 13) — and one script rather than two,
/// because the second copy is the one that stops matching what `core` actually spawns.
///
/// `#[cfg(unix)]` because M1 claims Linux x86-64 and nothing else (ADR 0009 §1), so a shell is
/// a fair assumption where a golden already is.
#[cfg(unix)]
pub fn fake_engine(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("fake-engine");
    let script = format!(
        "#!/bin/sh\ncat > '{plan}'\necho \"$1\" > '{argument}'\n{body}\n",
        plan = dir.join("plan.binpb").display(),
        argument = dir.join("argument").display(),
    );
    std::fs::write(&path, script).expect("the fake engine is writable");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}
