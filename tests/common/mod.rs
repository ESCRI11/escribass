//! What both suites in this package need to drive a real server process.
//!
//! Extracted in M1 PR 11, when `renders.rs` arrived needing the same four things
//! `determinism.rs` already had. The one that matters is [`refuse_if_stale`]: it is the guard
//! against a suite validating a build from before the change under test, and two copies of a
//! guard are one copy that stops being maintained. Everything else here is small enough that
//! it travels with it rather than being split across two files.

use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The instant every script runs at, and the seed every id is minted from.
///
/// 2026-09-02T00:00:00Z, the same instant the schema fixtures use. Under this seed an entity
/// id is a pure function of how many were minted before it: `new_song` mints four and
/// `Project::create` one, so the first id a tool mints is counter 6. That is why the scripts
/// name ids literally — a change in mint order changes what a script means, and it should say
/// so loudly rather than quietly still passing.
pub const AT: &str = "1788307200000";

// ---------------------------------------------------------------------------
// Finding the binary
// ---------------------------------------------------------------------------

/// The path to a binary this workspace builds.
///
/// `CARGO_BIN_EXE_<name>` is only set for integration tests of the package that *owns* the
/// binary, and this package does not own it — so the target directory is found from the test
/// executable's own path instead: `target/<profile>/deps/<test>` gives `target/<profile>` two
/// levels up. That survives `CARGO_TARGET_DIR` and a custom profile.
///
/// The binary is not built here on purpose: cargo holds the build-directory lock while tests
/// run, so a nested `cargo build` deadlocks. `cargo test` from the workspace root builds every
/// binary before any test runs, which is the supported way in; running this package alone can
/// find nothing to drive, and says so rather than hanging.
pub fn binary(name: &str) -> PathBuf {
    let executable = std::env::current_exe().expect("the test executable has a path");
    let target = executable
        .parent()
        .and_then(Path::parent)
        .expect("a test executable lives in <target>/<profile>/deps");
    let path = target.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.exists(),
        "`{name}` is not built at {}.\n\
         Run `cargo test` from the workspace root, which builds the binaries first; \
         `cargo test -p escribass-tests` on its own cannot.",
        path.display()
    );
    refuse_if_stale(name, &path);
    path
}

/// Refuses to run against a binary older than the source it was built from.
///
/// `cargo test -p escribass-tests` builds this package and the *libraries* it depends on — not
/// `escribass-core`'s binary targets, which is what the suite actually drives. Without this
/// check the suite happily validates a build from before your change and passes, which is worse
/// than failing: a determinism suite that green-lights stale bytes is the one kind of test that
/// must never be quietly wrong.
///
/// Found the hard way. Two deliberate mutations to `core` both "passed" here until the binary
/// was rebuilt by hand. The engine cannot be defended this way at all — it is built by CMake,
/// outside the cargo graph — which is why it reports the commits it was compiled from instead
/// and `renders.rs` compares them (ADR 0008 §5, trap 8).
fn refuse_if_stale(name: &str, path: &Path) {
    let built = std::fs::metadata(path).and_then(|m| m.modified()).expect("the binary has a time");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("a workspace root");

    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    // Every crate the binaries embed, not just `core`: the canonical writer lives in
    // `schema/`, and it is the thing the goldens exist to pin. `Cargo.lock` is watched too,
    // because §11's own drift example — a dependency changing how a float is written — moves
    // the lock and nothing else, and would otherwise be validated against the old binary.
    let mut pending = vec![
        workspace.join("core").join("src"),
        workspace.join("schema").join("src"),
        workspace.join("proto").join("src"),
    ];
    if let Ok(when) = std::fs::metadata(workspace.join("Cargo.lock")).and_then(|m| m.modified()) {
        newest = Some((when, workspace.join("Cargo.lock")));
    }
    while let Some(directory) = pending.pop() {
        let Ok(listing) = std::fs::read_dir(&directory) else { continue };
        for entry in listing.flatten() {
            let at = entry.path();
            if at.is_dir() {
                pending.push(at);
            } else if at.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(when) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(latest, _)| when > *latest) {
                        newest = Some((when, at));
                    }
                }
            }
        }
    }

    if let Some((when, source)) = newest {
        assert!(
            built >= when,
            "`{name}` is older than {}.\n\
             The suite drives the binary, and `cargo test -p escribass-tests` does not rebuild \
             it — only the libraries it links. Run `cargo test` from the workspace root, or \
             `cargo build` first. Passing against a stale build is worse than failing.",
            source.strip_prefix(workspace).unwrap_or(&source).display()
        );
    }
}

// ---------------------------------------------------------------------------
// Talking to it
// ---------------------------------------------------------------------------

/// Sends a scripted conversation to `escribass-mcp` and returns its answers.
///
/// The whole script goes in before anything is read, which is simple and bounded: a script
/// larger than the stdin pipe buffer would deadlock, so the size is asserted rather than left
/// to be discovered as a hang.
pub fn speak(flags: &[&str], project: &Path, requests: &[Value]) -> Vec<Value> {
    let mut conversation = String::new();
    for request in requests {
        conversation.push_str(&serde_json::to_string(request).expect("a request serialises"));
        conversation.push('\n');
    }
    assert!(
        conversation.len() < 60_000,
        "the script is {} bytes, near the stdin pipe buffer; \
         send it line by line rather than all at once",
        conversation.len()
    );

    let mut child = Command::new(binary("escribass-mcp"))
        .args(flags)
        .arg(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the server binary runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(conversation.as_bytes())
        .expect("the script is sent");

    let finished = child.wait_with_output().expect("the server exits when stdin closes");
    assert!(
        finished.status.success(),
        "the server failed: {}",
        String::from_utf8_lossy(&finished.stderr)
    );
    String::from_utf8(finished.stdout)
        .expect("stdout is utf-8")
        .lines()
        .map(|line| {
            serde_json::from_str(line).unwrap_or_else(|e| panic!("a frame is not JSON: {e}\n{line}"))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// A place to work
// ---------------------------------------------------------------------------

pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new(suite: &str, name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-{suite}-{name}-{}-{}.escri",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
