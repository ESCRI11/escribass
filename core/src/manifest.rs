//! The build manifest: what this engine build can host (ADR 0010 §4).
//!
//! `escribass_engine --scan` writes it beside the engine binary at build time, and it is
//! **never committed** — ADR 0008 §4's argument one artefact over: a committed manifest is a
//! second description of a plugin binary that has to agree with the plugin binary, and the
//! stale one of two pins is the silent one.
//!
//! It is required wherever it appears, never an `Option`. An optional manifest would give
//! `plugin_unknown` and `param_unknown` a "skip if absent" arm, and a rule that skips
//! silently is the quiet failure M0.4 exists to prevent: the suite would pass on a machine
//! with no engine and prove nothing (ADR 0010 §4). The binaries refuse to start without one.
//!
//! `core` reads this file and never writes it (CLAUDE.md #6: the engine reports what it can
//! host; core decides what that means for a song).

use crate::project::ProjectError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// What one engine build can host.
///
/// Unknown fields are ignored rather than refused: each plugin entry also carries the `path`
/// the engine opens it at, which is the engine's business and machine-specific — ADR 0010 §1
/// keeps it out of `lock.json` for exactly that reason, and out of here for want of a reader.
///
/// `Serialize` is here so the desktop host can hand this to its webview, which is what §9's
/// seventh view is a form over (ADR 0014 §1). It serialises what `core` **read**, not the file:
/// the ignored fields are gone, so the window is offered exactly the parameters the validator
/// will resolve a `set_param` against and never a machine-specific path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Manifest {
    /// The engine's submodule commits by component, which is what ADR 0008 §5 has the engine
    /// report about itself. Copied into `lock.json`'s `engine` block on every write.
    pub engine: BTreeMap<String, String>,
    /// One entry per plugin this build can host, keyed by the id the plugin reports itself
    /// as: `"<vendor>/<class name>"` (ADR 0010 §4, refined in PR 6, because JUCE's own
    /// identifier string hashes the plugin's path and a path cannot go in `lock.json`).
    pub plugins: BTreeMap<String, Plugin>,
    /// Which of those classes plays a `SamplerRef` (ADR 0010 §1, amended 2026-09-07 in PR 13).
    ///
    /// A sampler's `sfz_hash` pins the *patch*, not the build that plays it, and until this
    /// existed a sampler-only project pinned nothing about sfizz at all: `lock.json` had no
    /// entry, so `lock_mismatch` had nothing to fire on when sfizz_ui moved, and §11's
    /// **[MUST]** was unsatisfied for a whole device kind. Read from here rather than known by
    /// `core`, because which plugin plays an SFZ is the renderer's fact and a second copy of it
    /// is the copy that stops agreeing (CLAUDE.md #6).
    ///
    /// Required, like everything else here: a manifest written by an engine that does not say
    /// is a manifest from before this, and the honest answer to it is `manifest_unreadable`
    /// and a rebuild, not a silent skip.
    pub sampler: String,
}

/// One hostable plugin.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Plugin {
    /// The vendored submodule commit the binary was built from — the pin `lock.json` carries.
    pub commit: String,
    /// The version the plugin reports, beside the commit for a person reading `lock.json`.
    pub version: String,
    /// The plugin's own parameter ids, each mapped to its display name. The **key** is what a
    /// `ParamRef.param` and a key of `Instrument.params`/`Effect.params` must match: VST3
    /// names are not unique — Surge XT repeats 176 of its 2855, one per unassigned effect
    /// slot — so a name cannot be the identifier, and the name travels beside its id as the
    /// label an editor and a model read (ADR 0010 §4).
    pub params: BTreeMap<String, String>,
}

impl Manifest {
    /// Reads the manifest an engine build wrote.
    ///
    /// Both failures are operator errors in ADR 0006 §2's sense — a caller cannot fix a
    /// missing engine build by calling differently — so both are `ProjectError`, the same
    /// shape and the same side of the line as `lock_mismatch`.
    pub fn read(path: impl AsRef<Path>) -> Result<Manifest, ProjectError> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| ProjectError {
            path: path.display().to_string(),
            rule: "manifest_missing",
            message: format!(
                "no build manifest here ({e}); it is written by \
                 `cmake --build engine/build --target manifest` (ADR 0010 §4)"
            ),
        })?;
        serde_json::from_str(&text).map_err(|e| ProjectError {
            path: path.display().to_string(),
            rule: "manifest_unreadable",
            message: format!("this is not a build manifest: {e}"),
        })
    }
}
