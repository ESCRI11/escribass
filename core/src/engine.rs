//! Handing one plan to one engine process (ADR 0008 §1).
//!
//! The whole protocol: spawn the binary with the manifest as its argument, write exactly one
//! `RenderPlan` to its stdin and close it, read exactly one `RenderResult` from its stdout,
//! and take the exit code as the verdict. There is no framing because there is nothing to
//! frame — one message each way, delimited by end of stream — and no session, because the
//! process is born with its work and dies with its answer (ADR 0008 §2).
//!
//! **Every failure here is an operator's** (ADR 0006 §2). By the time a plan is written to a
//! pipe, the validator has refused an unresolvable reference, ADR 0010's load check has
//! refused a plugin this build lacks, and `compile` has refused what M1 cannot render — so
//! what is left is a binary that is missing, will not start, crashes, or answers with
//! something that is not a `RenderResult`. None of those is fixable by calling differently,
//! and reporting one as a refusal would put a model in a retry loop that cannot succeed.
//!
//! Nothing here reads a clock or takes entropy: a subprocess is a place both could enter
//! `core` by the back door (CLAUDE.md #3). The plan is a pure function of the document, the
//! engine's answer is a function of the plan, and the environment the child inherits is the
//! operator's — `LD_LIBRARY_PATH` for a plugin's own shared libraries is the reason it is
//! inherited rather than cleared.

use crate::project::ProjectError;
use escribass_proto::render::{RenderPlan, RenderResult};
use prost::Message;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Where the engine is, and the manifest it is handed.
///
/// Both are **told, never searched** — the rule ADR 0010 §4 set for the manifest, for the same
/// reason one level over: a search path that silently finds a stale engine renders a project
/// against a build nobody chose, and the symptom is a golden that moved with no PR to blame
/// (`docs/plan.md`, trap 8). The manifest is the one `core` itself validated against, so the
/// engine hosts the plugins the validator resolved rather than whatever a second file says.
#[derive(Debug, Clone)]
pub struct Engine {
    pub binary: PathBuf,
    pub manifest: PathBuf,
}

impl Engine {
    pub fn new(binary: impl Into<PathBuf>, manifest: impl Into<PathBuf>) -> Self {
        Self { binary: binary.into(), manifest: manifest.into() }
    }

    /// Renders one plan, in a process of its own, and returns what the engine reported.
    ///
    /// The WAV lands at `plan.output_path`; this returns the engine's own answer about it —
    /// the hash of the `data` chunk and the commits the binary was built from (ADR 0008 §5).
    pub fn render(&self, plan: &RenderPlan) -> Result<RenderResult, ProjectError> {
        let mut child = Command::new(&self.binary)
            .arg(&self.manifest)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| self.broke("engine_missing", format!("cannot start the engine: {e}")))?;

        // The plan goes in whole and stdin is closed, because end-of-stream is the framing
        // (ADR 0008 §1). A write failure is not reported here: the engine refuses a bad
        // manifest before it reads a byte, and the useful answer to a broken pipe is the exit
        // code and the line it printed, which the wait below has.
        //
        // ponytail: write-then-read is safe because the engine reads stdin to EOF before it
        // writes anything but a fatal line. A child that talked while consuming its input
        // could fill its stderr pipe and deadlock; the upgrade is a thread per pipe.
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&plan.encode_to_vec());
        }

        let finished = child
            .wait_with_output()
            .map_err(|e| self.broke("engine_failed", format!("the engine did not finish: {e}")))?;
        if !finished.status.success() {
            return Err(self.broke(
                "engine_failed",
                format!(
                    "the engine {}{}",
                    match finished.status.code() {
                        Some(code) => format!("exited {code}"),
                        None => "was killed by a signal".to_string(),
                    },
                    said(&finished.stderr),
                ),
            ));
        }

        let result = RenderResult::decode(&finished.stdout[..]).map_err(|e| {
            self.broke(
                "engine_unreadable",
                format!(
                    "the engine exited 0 and its stdout is not a RenderResult ({e}); \
                     stdout carries protobuf bytes and nothing else (ADR 0008 §1){}",
                    said(&finished.stderr),
                ),
            )
        })?;

        // Decoding is not enough to have been answered. proto3 has no required fields, so an
        // empty stdout — and all-zero stdout with it — decodes to a `RenderResult` with every
        // field at its default, and an engine that exited 0 having written nothing would come
        // back as a successful render with an empty hash and no file. `song_tools.proto` §8
        // says the opposite in as many words: an engine that "writes nothing is an operator
        // error and never a refusal". The hash is the answer, so its absence is the absence of
        // an answer — checked on `pcm_sha256` rather than on the byte count because the bytes
        // do not distinguish "said nothing" from "said all zeroes".
        if result.pcm_sha256.is_empty() {
            return Err(self.broke(
                "engine_unreadable",
                format!(
                    "the engine exited 0 and reported no pcm_sha256; a RenderResult without a \
                     hash is not an answer, and nothing was written (ADR 0008 §1){}",
                    said(&finished.stderr),
                ),
            ));
        }
        Ok(result)
    }

    fn broke(&self, rule: &'static str, message: String) -> ProjectError {
        ProjectError { path: self.binary.display().to_string(), rule, message }
    }
}

/// The engine's last words, for the message a person reads.
///
/// Trimmed to the tail: a render's stderr carries JUCE's and every plugin's chatter, and what
/// says why it failed is the end of it.
fn said(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr);
    let tail: Vec<&str> = text.lines().rev().take(3).collect();
    if tail.is_empty() {
        return " and said nothing".to_string();
    }
    format!(": {}", tail.into_iter().rev().collect::<Vec<_>>().join("; "))
}
