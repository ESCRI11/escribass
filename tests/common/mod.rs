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

/// The workspace root, from this package's manifest directory.
pub fn workspace() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("a workspace root")
}

/// The newest file under `roots`, where a root may be a file or a directory to walk.
///
/// Split out of [`refuse_if_stale`] in M1 PR 13, because the engine needs the same walk over a
/// different set of roots and a second copy of it would be the copy that stops being
/// maintained. Unreadable roots are skipped rather than refused: a root that is not there
/// cannot be newer than anything.
fn newest(roots: &[PathBuf]) -> Option<(std::time::SystemTime, PathBuf)> {
    let mut newest: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut consider = |at: PathBuf| {
        if let Ok(when) = std::fs::metadata(&at).and_then(|m| m.modified()) {
            if newest.as_ref().is_none_or(|(latest, _)| when > *latest) {
                newest = Some((when, at));
            }
        }
    };
    let mut pending: Vec<PathBuf> = Vec::new();
    for root in roots {
        if root.is_dir() {
            pending.push(root.clone());
        } else {
            consider(root.clone());
        }
    }
    while let Some(directory) = pending.pop() {
        let Ok(listing) = std::fs::read_dir(&directory) else { continue };
        for entry in listing.flatten() {
            let at = entry.path();
            if at.is_dir() {
                pending.push(at);
            } else {
                consider(at);
            }
        }
    }
    newest
}

/// Refuses to run against a build older than the source it was built from.
///
/// Without this a suite happily validates a build from before your change and passes, which is
/// worse than failing: a determinism suite that green-lights stale bytes is the one kind of
/// test that must never be quietly wrong.
///
/// Found the hard way, twice. `cargo test -p escribass-tests` builds this package and the
/// *libraries* it depends on, not `escribass-core`'s binary targets — two deliberate mutations
/// to `core` both "passed" here until the binary was rebuilt by hand. And in M1 PR 13 the same
/// hole was found under the engine: `renders.rs` compares the *submodule* commits the engine
/// embeds (ADR 0008 §5, trap 8), which is a check on the vendored trees and says nothing about
/// `engine/src`, so editing `main.cpp` and not rebuilding left the golden suite green. The
/// commit comparison and this are two different questions; both are asked.
pub fn refuse_if_older_than_source(what: &str, built: &Path, roots: &[PathBuf], remedy: &str) {
    let when_built =
        std::fs::metadata(built).and_then(|m| m.modified()).expect("the binary has a time");
    let Some((when, source)) = newest(roots) else { return };
    assert!(
        when_built >= when,
        "`{what}` is older than {}.\n{remedy}\nPassing against a stale build is worse than \
         failing.",
        source.strip_prefix(workspace()).unwrap_or(&source).display()
    );
}

/// What the engine binary must be newer than.
///
/// Named here, beside the walker, so a test can assert these roots actually reach
/// `engine/src/main.cpp` — the claim that was false before M1 PR 13 and that nothing checked.
///
/// The two `.proto` files are here because the engine's CMake generates its C++ from them
/// (ADR 0008 §4), so an engine built before a change to either speaks the old wire. They were
/// missing until M2 PR 11, and M2 PR 10 changed `render.proto` under the hole; the test that
/// walks these roots now reads the CMake for the protos it must reach.
pub fn engine_sources() -> Vec<PathBuf> {
    vec![
        workspace().join("engine").join("src"),
        workspace().join("engine").join("cmake"),
        workspace().join("engine").join("CMakeLists.txt"),
        workspace().join("proto").join("render.proto"),
        workspace().join("schema").join("song.proto"),
    ]
}

/// Where the generative compiler lives (ADR 0024 §8).
pub fn generative() -> PathBuf {
    workspace().join("compilers").join("generative")
}

/// What the generative compiler's **environment** must be newer than.
///
/// Named here beside [`engine_sources`], and the list is short for a reason the test that
/// walks it asserts rather than assumes: `uv sync` installs this project **editable**, so
/// `src/escribass_generative/*.py` and the generated `escribass_proto` and `escribass_schema`
/// it imports are read from the working tree on every start and cannot be stale. What *can*
/// be stale is everything the environment is built from rather than read through — the
/// dependency closure (`uv.lock`), the version `importlib.metadata` reports back as
/// `dsl_version` (`pyproject.toml`), and the interpreter `python_version` names
/// (`.python-version`) — and each of those three is a child reporting something the project
/// did not record, which is exactly the stale sandbox trap 5 names.
pub fn generator_sources() -> Vec<PathBuf> {
    vec![
        generative().join("pyproject.toml"),
        generative().join("uv.lock"),
        generative().join(".python-version"),
    ]
}

/// The command the suite tells `core` as its `--generator`, having refused a stale one.
///
/// `--no-sync`, deliberately: with a plain `uv run` the launcher would sync the environment
/// on the way past, so the staleness this guards against could never be observed and a test
/// would silently mutate the thing it is validating. ADR 0024 §4 measured the limits binding
/// under exactly this command.
///
/// One string, because `--generator` is one flag split on whitespace — the bargain the Tauri
/// host's `--ai` already makes, with the same upgrade path (`core/src/bin/`). A checkout at a
/// path with a space in it would be expressed wrongly rather than refused, so it is refused
/// here, where the message can say what the limit is.
pub fn generator_command() -> String {
    refuse_if_sandbox_is_stale();
    let project = generative().display().to_string();
    assert!(
        !project.contains(char::is_whitespace),
        "`--generator` is one string split on whitespace and this checkout is at `{project}`; \
         the flag cannot express it (core/src/bin/, the `ponytail:` note on --generator)"
    );
    format!("uv run --no-sync --project {project} escribass-generative")
}

/// Refuses to compile against an environment older than what it was built from (trap 5).
///
/// M0.4 found the suite could validate a stale binary; M1 PR 13 found the render suite had
/// never compared the engine against `engine/src`. This is the same question asked of the
/// third child: a determinism golden carries `lock.json`'s `toolchains` block and every note
/// the DSL produced, and a `uv sync` that was not run passes all of it.
///
/// What stands in for "the binary" is the installed distribution's `RECORD`, which `uv sync`
/// rewrites every time it reinstalls the project — and the project is reinstalled on every
/// sync, because it is a path dependency. Its directory name carries the version, so a
/// `pyproject.toml` bumped and not synced has no matching `RECORD` at all, which this reports
/// as the same thing for the same reason.
fn refuse_if_sandbox_is_stale() {
    let site = generative().join(".venv/lib");
    let remedy = "Run `uv sync --locked` in compilers/generative/. The suite drives the \
                  installed environment, and cargo cannot build one — so without this a \
                  golden would be compared against a compiler from before your change.";
    let installed = std::fs::read_dir(&site)
        .into_iter()
        .flatten()
        .flatten()
        .map(|python| python.path().join("site-packages"))
        .flat_map(|packages| std::fs::read_dir(packages).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with("escribass_generative-") && name.ends_with(".dist-info")
                })
        });
    let Some(installed) = installed else {
        panic!(
            "`compilers/generative/` has no installed `escribass-generative` under {}.\n{remedy}",
            site.display()
        );
    };
    // The claim the short root list rests on, asserted rather than remembered: the install is
    // **editable**, so the compiler's own source is read from the working tree and is not a
    // root below. The day it stops being editable, `src/` becomes one and this says so.
    let editable = std::fs::read_to_string(
        installed.parent().expect("site-packages").join("escribass_generative.pth"),
    )
    .unwrap_or_default();
    assert!(
        editable.trim().ends_with("compilers/generative/src"),
        "`escribass-generative` is no longer installed editable (its .pth says `{}`), so its \
         source can now be stale and `generator_sources()` must gain \
         `compilers/generative/src`",
        editable.trim()
    );
    let record = installed.join("RECORD");
    // Named here rather than left to `refuse_if_older_than_source`'s `expect`, which would
    // say only that something has no modification time.
    assert!(record.exists(), "`{}` has no RECORD.\n{remedy}", installed.display());
    refuse_if_older_than_source(
        "the generative compiler's environment",
        &record,
        &generator_sources(),
        remedy,
    );
}

/// The cargo half of the check above: a workspace binary against every crate it embeds.
fn refuse_if_stale(name: &str, path: &Path) {
    // Every crate the binaries embed, not just `core`: the canonical writer lives in
    // `schema/`, and it is the thing the goldens exist to pin. `Cargo.lock` is watched too,
    // because §11's own drift example — a dependency changing how a float is written — moves
    // the lock and nothing else, and would otherwise be validated against the old binary.
    refuse_if_older_than_source(
        name,
        path,
        &[
            workspace().join("core").join("src"),
            workspace().join("schema").join("src"),
            workspace().join("proto").join("src"),
            workspace().join("Cargo.lock"),
        ],
        "The suite drives the binary, and `cargo test -p escribass-tests` does not rebuild it \
         — only the libraries it links. Run `cargo test` from the workspace root, or \
         `cargo build` first.",
    );
}

// ---------------------------------------------------------------------------
// Talking to it
// ---------------------------------------------------------------------------

/// Sends a scripted conversation to `escribass-mcp` and returns its answers.
///
/// The whole script goes in before anything is read, which is simple and bounded: a script
/// larger than the stdin pipe buffer would deadlock, so the size is asserted rather than left
/// to be discovered as a hang.
///
/// Closing stdin straight after is safe because the server makes it so, not because the scripts
/// are quick: until M2 PR 11 rmcp dropped every answer still unwritten five seconds after EOF,
/// and this suite was exposed the moment a scripted call was slow. `escribass-mcp` now reads EOF
/// only once every call it read has been answered, in the order sent (`core::InArrivalOrder`).
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
