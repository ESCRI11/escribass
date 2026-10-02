//! The tool API, implemented once (§5, ADR 0006).
//!
//! Both transports — gRPC and MCP — dispatch here. They translate; they decide nothing. Every
//! rule about what a tool call means, what it returns, and what counts as an error lives in
//! this file, so the two surfaces cannot drift into disagreeing about the same call.
//!
//! **The split ADR 0006 §2 makes, made once, by return type.** `Ok(ToolResult)` with
//! `valid = false` is something the caller can fix by calling differently; `Err(ProjectError)`
//! is something only an operator can fix. It is not a classifier over rule names: `prepare`
//! is pure and cannot fail for an operator reason, and `record` only fails for one, so the
//! signatures already carry the distinction and this module just keeps it.
//!
//! **A dry run is the real path's first half**, not a copy of it (ADR 0006 §3). The patch a
//! dry run returns is produced by the same `ops_text` that fills the entry a commit writes.

use crate::clock::Clock;
use crate::engine::{Engine, Preview};
use crate::generator::Sandbox;
use crate::history::{ops_of, ops_text, HistoryError};
use crate::id::IdSource;
use crate::patch::{diff, Op};
use crate::call::{Answer, CallError, ErrorKind, OFFERED};
use crate::project::{Prepared, Project, ProjectError, DEFAULT_MODEL, DEFAULT_PROVIDER};
use crate::version::restore_versions;
use crate::validate::Violation;
use crate::tools;
use escribass_proto::generate::compile_response::Result as Answered;
use escribass_proto::tools::{
    AddAssetRequest, AddAutomationRequest, AddClipRequest, AddEffectRequest, AddSectionRequest,
    AddTrackRequest, ApplyPatchRequest, AssetResponse, CompileGeneratorRequest,
    CreateBranchRequest, DefineGeneratorRequest, DeleteBranchRequest, GetSongAtRequest,
    HistoryResponse, MergeBranchRequest, MergeSide, MoveSectionRequest, PreviewResponse,
    QuantizeRequest, RedoRequest, RenderExportRequest, RenderPreviewRequest, RenderResponse,
    SetNotesRequest, SetParamRequest, SetTempoRequest, SetTrackInstrumentRequest, SongResponse,
    SwitchBranchRequest, ToolResult, TransposeRequest, UndoRequest,
};
use escribass_schema::song::{Author, Provenance, Song};
use serde_json::{Map, Value};

/// One open project, and the sources of the two things §11 forbids `core` from reaching for
/// on its own.
///
/// This is ADR 0001's "session constructor". The project is named when the process starts and
/// never changes: an MCP stdio process is not a session, and two writers on one `.escri` would
/// race the write ordering ADR 0004 depends on (ADR 0006 §5).
pub struct Session {
    project: Project,
    // `+ Send` so a session can be owned by an async server task. Not `Sync`: there is one
    // writer (ADR 0001 §2), and the MCP handler holds the session behind a `Mutex`.
    ids: Box<dyn IdSource + Send>,
    clock: Box<dyn Clock + Send>,
    author: Author,
    /// Where the engine is, when this process was told (`--engine`). `None` is not a silent
    /// skip: `render_export` says so and refuses, as an operator error (see it below).
    engine: Option<Engine>,
    /// Where the generative compiler is, when this process was told (`--generator`). `None`
    /// is not a silent skip: `compile_generator` says so and refuses, as an operator error
    /// (ADR 0024 §1).
    sandbox: Option<Sandbox>,
    /// The preview playing, if one is (ADR 0013 §3).
    ///
    /// The one piece of session state here, because it is a fact about a running process and
    /// not about the song, and the log records none of it. One at most, because the stream is
    /// the session's identifier for a transport and there is one transport. It is **never**
    /// what `render_export` uses — an export spawns its own process, whatever is playing.
    ///
    /// There is deliberately no undo cursor beside it. Until M2 PR 11 there was one, empty in
    /// every fresh session, and a relaunched window's ⌘Z walked past undos it had not made and
    /// re-applied an edit; the log records every undo and redo, so it already answers how far
    /// back ⌘Z has walked (ADR 0005 §4, amended 2026-09-15).
    preview: Option<Preview>,
    /// The turn in progress, if one is (ADR 0019 §1).
    ///
    /// Session state beside the preview and for the same reason: it is a fact about something
    /// running, never the document and never the log. **One at most** — a second fork stacked
    /// on the first would be a proposal nobody has read being built on top of one nobody has
    /// approved (ADR 0019 §3). `Box` because a proposal *is* a session, on a copy of this
    /// project, which is what lets the model's calls go through the one dispatch rather than a
    /// second one written beside it (ADR 0006 §1).
    proposal: Option<Box<Proposal>>,
    /// Whose provenance the call being run writes, when it is not this session's plain author.
    made: Made,
}

/// What a call writes into the two §4.3 fields core owns, and which of them it decides.
///
/// Three callers, three answers, and the reason they cannot be one: until M3 every call that
/// reached [`Session::run`] was somebody's typed at this machine, so "the session's author,
/// now" answered both questions at once (ADR 0006 §4).
enum Made {
    /// Every call that arrives with no other word — the binaries' `--author`, the window's
    /// `Human`. Provenance is this session's author and clock.
    Session,
    /// A **proposal's call**, run on the fork: the three ids of ADR 0021 §2, so an entity the
    /// model mints is stamped with the call that minted it. Nothing is recorded and nothing is
    /// written; the prepared document is kept on the fork (ADR 0019 §1).
    Call(Provenance),
    /// A **proposal's patch**, committed: this is the *entry's* provenance — `AUTHOR_MODEL`,
    /// the model the turn's last response named, the `prompt_id`, and no `tool_call_id`, since
    /// a composed proposal has several and its entities carry theirs (ADR 0021 §2). The ops
    /// already carry the provenance `prepare` stamped on the fork, so nothing re-decides it
    /// ([`Project::prepare_proposal`]).
    Proposal(Provenance),
    /// **Edit**: a proposal's patch that a person rewrote and applied as their own
    /// (ADR 0019 §3). `AUTHOR_HUMAN`, no `model_id`, and the `prompt_id` kept so the entry
    /// still leads to the conversation the patch was made from.
    ///
    /// It is not [`Made::Proposal`], and the difference is the whole of why it is a fourth
    /// arm rather than a flag. A proposal's patch keeps the provenance `prepare` decided on
    /// the fork; an edited one must **not**, because its new entities carry the model's and
    /// should get the person's — which is ADR 0021 §1's own worked example, and is what the
    /// ordinary `prepare` does. It is not [`Made::Call`] either: a call keeps its document on
    /// a fork and records nothing, and an edit commits.
    Edit(Provenance),
}

impl Session {
    pub fn new(
        project: Project,
        ids: Box<dyn IdSource + Send>,
        clock: Box<dyn Clock + Send>,
        author: Author,
    ) -> Self {
        Self {
            project,
            ids,
            clock,
            author,
            engine: None,
            sandbox: None,
            preview: None,
            proposal: None,
            made: Made::Session,
        }
    }

    /// Names the engine binary this session renders with (ADR 0008 §2).
    ///
    /// Separate from `new` because a session that never renders needs none, and every other
    /// caller of `new` — M0's tests, both binaries opening a project — is one of those. A
    /// process told nothing here still edits, validates and previews a render; it refuses only
    /// the call that would need a binary.
    pub fn set_engine(&mut self, engine: Engine) {
        self.engine = Some(engine);
    }

    /// Names the command this session compiles a generator with (ADR 0024 §1).
    ///
    /// Separate from `new` for [`set_engine`](Self::set_engine)'s reason, one child over: a
    /// session that never compiles needs none, and a process told nothing here still edits,
    /// validates, renders and previews — it refuses only the call that would need a compiler.
    /// A command, not a path, and never inspected (see [`crate::generator`]).
    pub fn set_sandbox(&mut self, sandbox: Sandbox) {
        self.sandbox = Some(sandbox);
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    // ---- reads (ADR 0006 §1) ----

    pub fn get_song(&self) -> SongResponse {
        SongResponse { song: Some(self.project.song().clone()) }
    }

    /// The document as it stood at one entry, by replaying the log to it (ADR 0001 §2).
    pub fn get_song_at(&self, request: &GetSongAtRequest) -> Result<SongResponse, ProjectError> {
        let replayed = self
            .project
            .history()
            .materialise(&request.entry_id)
            .map_err(|e| history_error(&e))?;
        let song: Song = serde_json::from_value(replayed).map_err(|e| ProjectError {
            path: request.entry_id.clone(),
            rule: "replay_failed",
            message: format!("the replayed document is not a valid song: {e}"),
        })?;
        Ok(SongResponse { song: Some(song) })
    }

    pub fn get_history(&self) -> HistoryResponse {
        HistoryResponse {
            entries: self.project.history().entries().clone().into_iter().collect(),
            refs: Some(self.project.history().refs().clone()),
        }
    }

    // ---- the raw pipeline ----

    /// Apply an RFC 6902 patch (ADR 0006, `apply_patch`).
    ///
    /// The typed tools are the supported surface; this one exists because M0.4 has to drive
    /// `core` through the tool API rather than the file (CLAUDE.md #2), and because §9 lets a
    /// user edit a proposed diff before applying it.
    pub fn apply_patch(&mut self, request: &ApplyPatchRequest) -> Result<ToolResult, ProjectError> {
        let ops: Vec<Op> = match serde_json::from_slice(&request.patch) {
            Ok(ops) => ops,
            Err(e) => {
                return Ok(refused(vec![Violation {
                    path: String::new(),
                    rule: "patch_unreadable",
                    message: format!("`patch` is not an RFC 6902 array: {e}"),
                }]))
            }
        };
        self.run("apply_patch", &ops, request.dry_run)
    }

    // ---- typed tools (§5) ----

    /// §5 `add_track`.
    pub fn add_track(&mut self, request: &AddTrackRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::add_track(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("add_track", built, request.dry_run, ids)
    }

    /// §5 `set_track_instrument`.
    pub fn set_track_instrument(
        &mut self,
        request: &SetTrackInstrumentRequest,
    ) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::set_track_instrument(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("set_track_instrument", built, request.dry_run, ids)
    }

    /// §5 `add_effect`.
    pub fn add_effect(&mut self, request: &AddEffectRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::add_effect(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("add_effect", built, request.dry_run, ids)
    }

    /// §5 `set_param`.
    pub fn set_param(&mut self, request: &SetParamRequest) -> Result<ToolResult, ProjectError> {
        let built = tools::set_param(self.project.song(), request);
        self.from_tool("set_param", built, request.dry_run, self.ids.fork())
    }

    /// §5 `add_clip`.
    pub fn add_clip(&mut self, request: &AddClipRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::add_clip(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("add_clip", built, request.dry_run, ids)
    }

    /// §5 `set_notes`.
    pub fn set_notes(&mut self, request: &SetNotesRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::set_notes(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("set_notes", built, request.dry_run, ids)
    }

    /// §5 `transpose`.
    pub fn transpose(&mut self, request: &TransposeRequest) -> Result<ToolResult, ProjectError> {
        let built = tools::transpose(self.project.song(), request);
        self.from_tool("transpose", built, request.dry_run, self.ids.fork())
    }

    /// §5 `quantize`.
    pub fn quantize(&mut self, request: &QuantizeRequest) -> Result<ToolResult, ProjectError> {
        let built = tools::quantize(self.project.song(), request);
        self.from_tool("quantize", built, request.dry_run, self.ids.fork())
    }

    /// §5 `add_automation`.
    pub fn add_automation(
        &mut self,
        request: &AddAutomationRequest,
    ) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::add_automation(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("add_automation", built, request.dry_run, ids)
    }

    /// §5 `set_tempo`.
    pub fn set_tempo(&mut self, request: &SetTempoRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::set_tempo(self.project.song(), request, &mut *ids);
        self.from_tool("set_tempo", built, request.dry_run, ids)
    }

    /// §5 `add_section`.
    pub fn add_section(&mut self, request: &AddSectionRequest) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::add_section(request, &mut *ids, &*self.clock, self.author);
        self.from_tool("add_section", built, request.dry_run, ids)
    }

    /// §5 `move_section`.
    pub fn move_section(
        &mut self,
        request: &MoveSectionRequest,
    ) -> Result<ToolResult, ProjectError> {
        let built = tools::move_section(self.project.song(), request);
        self.from_tool("move_section", built, request.dry_run, self.ids.fork())
    }

    // ---- generators (§7.1, ADR 0026 §2) ----

    /// §5 `define_generator`: adds a generator and **compiles nothing** (ADR 0026 §2).
    ///
    /// Two calls rather than one, because compiling on define costs a process for every
    /// definition a person then edits before compiling, and puts two things a dry run cannot
    /// show apart into one entry (ADR 0024 §5). A dry run of this is the ordinary validator
    /// pass over the added entity.
    pub fn define_generator(
        &mut self,
        request: &DefineGeneratorRequest,
    ) -> Result<ToolResult, ProjectError> {
        let mut ids = self.ids.fork();
        let built = tools::define_generator(
            self.project.song(), request, &mut *ids, &*self.clock, self.author);
        self.from_tool("define_generator", built, request.dry_run, ids)
    }

    /// §5 `compile_generator`: spawn the sandbox, replace the target clip's notes whole, and
    /// write `compiled_hash` and `toolchain_version` — one entry (ADR 0024 §5).
    ///
    /// **ADR 0006 §2's line runs through this method four times**, and keeping the sides apart
    /// is the whole job. What the caller can fix comes back as `valid = false`:
    /// `generator_unknown`, `target_not_note_clip`, and `generator_error` — the child's own
    /// `line:column` and text, which is the one refusal in this system whose message is the
    /// whole fix (Plate 3). What only an operator can fix is `Err`: `generator_missing` (this
    /// process was told no `--generator`), `generator_failed` (the child would not start,
    /// named no socket, or went away without answering), `generator_timeout` (it answered
    /// nothing at all inside the wall clock — ADR 0024 §7, amended 2026-10-01, since no source
    /// reaches that bound and the message carries no line) and `toolchain_mismatch` (ADR 0027
    /// §2). A model told to retry any of those four would spend §6's three refusals learning
    /// it cannot.
    ///
    /// **A dry run compiles**, and answers with the diff the commit would write — the code
    /// view's *Compile*, whose *Apply* is the commit (ADR 0017 §4) — except when the request
    /// already hashes to `compiled_hash`, which is *up to date* and **spawns no process**
    /// (ADR 0024 §6). So a status read on a fresh generator costs nothing and one on a stale
    /// generator costs the compile a person is about to ask for anyway.
    pub fn compile_generator(
        &mut self,
        request: &CompileGeneratorRequest,
    ) -> Result<ToolResult, ProjectError> {
        let Some(generator) = self.project.song().generators.get(&request.generator_id).cloned()
        else {
            return Ok(refused(vec![Violation {
                path: "/generator_id".to_string(),
                rule: "generator_unknown",
                message: format!(
                    "`{}` is not a generator in this song",
                    request.generator_id
                ),
            }]));
        };
        let (clip_id, compiling) =
            match crate::generator::to_compile(self.project.song(), &generator) {
                Ok(both) => both,
                Err(violation) => return Ok(refused(vec![violation])),
            };
        let hash = crate::generator::compiled_hash(&compiling);

        // ADR 0024 §6, and the reason the hash is computed before the child is started rather
        // than after: *up to date* is an answer no process has to be spawned to give.
        if request.dry_run && hash == generator.compiled_hash {
            return Ok(described("up to date".to_string()));
        }

        let sandbox = self.sandbox.as_ref().ok_or_else(|| ProjectError {
            path: "--generator".to_string(),
            rule: "generator_missing",
            message: "this process was not told how to run the generative compiler, and \
                      does not look for one: pass `--generator <command>`, which on a build \
                      tree is `uv run --no-sync --project compilers/generative \
                      escribass-generative` (ADR 0024 §1)"
                .to_string(),
        })?;
        // Every way of not answering is an operator error — `generator_timeout` and
        // `generator_failed` alike (ADR 0024 §7, amended 2026-10-01 in M4 PR 6) — **except
        // one**, which is the amendment of 2026-10-02: a child killed by its own hard CPU
        // limit said nothing, and is still the author's to fix. `Sandbox::compile` is what can
        // tell that case apart, so it names the rule a refusal carries and this turns it into
        // one. The other way a *source* fails is the `Diagnostic` arm below, which the child's
        // own soft CPU limit and its memory limit come back as, carrying the line they fired on.
        let answer = match sandbox.compile(&compiling) {
            Ok(answer) => answer,
            Err(e) if e.rule == "generator_error" => {
                return Ok(refused(vec![Violation {
                    path: format!("/generators/{}/source", generator.id),
                    rule: "generator_error",
                    // No position prefix, because there is none: the kernel stopped the child
                    // mid-call and nothing in it ever regained control to name a line.
                    message: e.message,
                }]))
            }
            Err(e) => return Err(e),
        };
        let crate::generator::Compiled { dsl_version, python_version, result } = answer;
        let compiled = match result {
            // The child's own words, whole, with `core`'s rule around them: it does not
            // rewrite them, because the same text the user reads is what the model retries
            // against (ADR 0026 §3). A column of 0 means the child had no position for it.
            Answered::Diagnostic(said) => {
                let at = if said.column > 0 {
                    format!("{}:{}", said.line, said.column)
                } else {
                    said.line.to_string()
                };
                return Ok(refused(vec![Violation {
                    path: format!("/generators/{}/source", generator.id),
                    rule: "generator_error",
                    message: format!("{at}: {}", said.message),
                }]));
            }
            Answered::Notes(notes) => notes.notes,
        };

        // **Before anything is written**, and nothing is written if it disagrees: not the
        // notes, not the hash, not the block (ADR 0027 §2). An operator error, because every
        // fix is an operator action — `uv sync --locked` in `compilers/generative/`, install
        // the pinned interpreter, or edit `lock.json` deliberately — and a model retrying a
        // tool call can produce none of them.
        //
        // After the diagnostic above and not before it, because what this guards is *writing*
        // and a diagnostic writes nothing: an author whose source does not compile is handed
        // their line, and meets this on the next call, which is the one that would have
        // written notes a golden compares.
        if let Some(e) = self.toolchain_mismatch(&generator, &dsl_version, &python_version) {
            return Err(e);
        }

        let mut ids = self.ids.fork();
        let built = tools::compile_generator(
            &clip_id,
            &generator.id,
            &hash,
            &dsl_version,
            &compiled,
            &mut *ids,
            &*self.clock,
            self.author,
        );
        let committed = self.from_tool("compile_generator", built, request.dry_run, ids)?;

        // ADR 0027 §1: written on the first compile that **commits**, from what the child
        // reported. `entry_id` is the only honest test of that — a dry run and a proposal's
        // call both answer `valid` and write nothing, and a project pinned by a compile
        // nobody kept would be a pin no log explains.
        //
        // **A proposal's compile therefore writes the generator's `toolchain_version` and not
        // the project's block** — and from M4 PR 6, where `compile_generator` joined `OFFERED`,
        // that path is reachable and the behaviour is **accepted rather than tolerated**
        // (ADR 0027 §1, amended 2026-10-01). The version is in the patch a person applies; the
        // block is not, because no entry exists to explain it until one commits. The two
        // cannot then disagree, because `toolchain_mismatch` compares the generator's own
        // version as well as the block and a project with no block still has that comparison.
        // What it costs is one sentence: a project whose compiles have all been a model's is
        // unpinned until a person compiles once. `core/tests/generator.rs`,
        // `a_models_compile_writes_the_generators_version_and_the_block_waits_for_a_commit`,
        // is where that is held still.
        if committed.valid && !committed.entry_id.is_empty() {
            self.project.record_toolchain(&dsl_version, &python_version)?;
        }
        Ok(committed)
    }

    /// Whether the child that just answered is the one this project and this generator were
    /// compiled by (ADR 0027 §2).
    ///
    /// Two comparisons, because there are two pins and they are written by one answer: the
    /// project's `toolchains.generator` block, if it has one, and the generator's own
    /// `toolchain_version`, if it has ever been compiled. A project that has neither is being
    /// compiled for the first time and has nothing to disagree with.
    ///
    /// It is **also trap 5's guard**: a `uv sync` that was not run, or an environment from
    /// before the change, is a child stating a version the project did not record, and the
    /// compile is refused rather than a golden passing on a stale sandbox.
    fn toolchain_mismatch(
        &self,
        generator: &escribass_schema::song::Generator,
        dsl_version: &str,
        python_version: &str,
    ) -> Option<ProjectError> {
        const REPIN: &str = "run `uv sync --locked` in compilers/generative/, install the \
                             pinned interpreter, or edit lock.json and the generator \
                             deliberately (ADR 0027 §2)";
        let mismatch = |what: &str, recorded: &str, stated: &str| ProjectError {
            path: self.project.root().display().to_string(),
            rule: "toolchain_mismatch",
            message: format!(
                "this project was compiled with {what} `{recorded}` and the compiler that \
                 just answered is `{stated}`; nothing was written. {REPIN}"
            ),
        };
        if let Some(pinned) = self.project.toolchains() {
            if pinned.generator.dsl != dsl_version {
                return Some(mismatch(
                    "the generator DSL at version",
                    &pinned.generator.dsl,
                    dsl_version,
                ));
            }
            if pinned.generator.python != python_version {
                return Some(mismatch("Python", &pinned.generator.python, python_version));
            }
        }
        if !generator.toolchain_version.is_empty() && generator.toolchain_version != dsl_version {
            return Some(mismatch(
                "this generator's own toolchain_version at",
                &generator.toolchain_version,
                dsl_version,
            ));
        }
        None
    }

    // ---- assets (§10) ----

    /// `add_asset`: the hash that names `content` in `assets/`, written unless this is a dry
    /// run.
    ///
    /// Not through `run`, and not a `ToolResult`: there are no ops, no entry and no
    /// caller-fixable refusal — core inspects no content, so the only way this fails is an
    /// unwritable directory, an operator's problem (ADR 0006 §2). What the caller needs is
    /// the address, so that is what comes back, the way a read returns what it read.
    ///
    /// A dry run is the same hash without the write, which is the first half of the real
    /// path (ADR 0006 §3): the name an asset gets is decided before anything touches disk.
    pub fn add_asset(&self, request: &AddAssetRequest) -> Result<AssetResponse, ProjectError> {
        let asset_hash = if request.dry_run {
            crate::project::asset_hash(&request.content)
        } else {
            self.project.add_asset(&request.content)?
        };
        Ok(AssetResponse { asset_hash })
    }

    // ---- rendering (§8) ----

    /// §5 `render_export`: compile the song, hand the plan to a fresh engine, answer with what
    /// the engine reported (ADR 0007 §4, ADR 0008 §1).
    ///
    /// The two halves of ADR 0006 §2's line meet here, and keeping them apart is the whole
    /// job. `compile`'s refusals — a device M1 cannot render, an asset the project does not
    /// hold — are `Vec<Violation>` and come back as `valid = false`, because the caller fixes
    /// them by calling differently. An engine that is missing, will not start, crashes or
    /// answers with nothing is `Err(ProjectError)`, because no retry fixes a crash and a model
    /// told to retry one would spend §6's three turns learning that.
    ///
    /// Nothing in the song changes, so nothing is recorded: a render reads the document. The
    /// answer is not a `ToolResult` for that reason and one more — the hash the engine
    /// computed has nowhere to go in one (`proto/song_tools.proto`, `RenderResponse`).
    pub fn render_export(
        &self,
        request: &RenderExportRequest,
    ) -> Result<RenderResponse, ProjectError> {
        // Both refusals are collected before either is returned, sorted, which is compile's own
        // rule (ADR 0007 §4) for the reason ADR 0006 §1 gives: a model fixing one problem at a
        // time spends §6's three retries on one call.
        let mut refused = Vec::new();
        // A relative path would resolve against this process's working directory, which the
        // caller cannot see and the engine inherits. `render.proto` says the path is absolute;
        // this is where a caller learns it, and it is caller-fixable like compile's own.
        if !std::path::Path::new(&request.output_path).is_absolute() {
            refused.push(Violation {
                path: "/output_path".to_string(),
                rule: "output_path_relative",
                message: format!(
                    "`{}` is not an absolute path, and a render writes where it is told",
                    request.output_path
                ),
            });
        }
        let compiled = match crate::compile(self.project.song(), &self.project.assets()?) {
            Ok(plan) => Some(plan),
            Err(violations) => {
                refused.extend(violations);
                None
            }
        };
        let Some(mut plan) = compiled.filter(|_| refused.is_empty()) else {
            refused.sort();
            return Ok(render_refused(refused));
        };
        // `compile` leaves it empty: where the WAV goes is this call's argument, not the
        // document's (`core/src/render.rs`).
        plan.output_path = request.output_path.clone();
        let summary = describe(&plan);

        // A dry run is the first half of the real path, as it is for every tool (ADR 0006 §3):
        // the plan is compiled and described, and no process is started. That is also what
        // lets the determinism suite exercise this tool on a machine with no engine build —
        // what it cannot cover, `tests/AGENTS.md` names rather than skips.
        if request.dry_run {
            return Ok(RenderResponse { valid: true, errors: vec![], summary, result: None });
        }

        let engine = self.engine.as_ref().ok_or_else(|| ProjectError {
            path: "--engine".to_string(),
            rule: "engine_unset",
            message: "this process was not told where the engine is, and does not look for one: \
                      pass `--engine <path to escribass_engine>` (ADR 0008 §2)"
                .to_string(),
        })?;
        let result = engine.render(&plan)?;
        Ok(RenderResponse { valid: true, errors: vec![], summary, result: Some(result) })
    }

    /// §5 `render_preview`: play the song as it stands through a live engine, or move or stop
    /// what is playing (ADR 0013 §2, §3).
    ///
    /// ADR 0006 §2's line is drawn where `render_export` draws it. What the caller can fix is a
    /// refusal — compile's own, a negative tick, and a command for a transport that is not there
    /// (`preview_idle`, since the first command carries the plan). What only an operator can
    /// fix is `Err`: no engine told, an engine that will not start or finds no audio device, and
    /// a preview that ends instead of answering — after which there is no preview, and the next
    /// `play` starts another.
    ///
    /// Nothing in the song changes and nothing is recorded, so the answer is not a `ToolResult`
    /// (`proto/song_tools.proto`, `PreviewResponse`). Its summary is derived from the command
    /// and, for a play, from the plan — never from where the transport is, which is the one
    /// thing here that is not a function of the document (§11).
    pub fn render_preview(
        &mut self,
        request: &RenderPreviewRequest,
    ) -> Result<PreviewResponse, ProjectError> {
        use escribass_proto::render::{
            preview_command, PreviewCommand, PreviewLoop, PreviewPlay, PreviewSeek, PreviewStop,
        };
        use escribass_proto::tools::render_preview_request::Command;

        let mut refused = Vec::new();
        let negative = |refused: &mut Vec<Violation>, path: &str, tick: i32| {
            if tick < 0 {
                refused.push(Violation {
                    path: path.to_string(),
                    rule: "tick_negative",
                    message: format!("tick {tick} is before the start of the song"),
                });
            }
        };
        // Seek, loop and stop are commands to a transport, and the first command on a stream
        // carries the plan (ADR 0013 §2). With nothing playing there is no transport to move,
        // and the caller fixes that by calling `play` first.
        let idle = |refused: &mut Vec<Violation>, live: bool, path: &str| {
            if !live {
                refused.push(Violation {
                    path: path.to_string(),
                    rule: "preview_idle",
                    message: "nothing is playing: a preview starts with `play`".to_string(),
                });
            }
        };
        let live = self.preview.is_some();

        let (command, summary) = match &request.command {
            None => {
                refused.push(Violation {
                    path: "/command".to_string(),
                    rule: "oneof_unset",
                    message: "one of `play`, `seek`, `loop` or `stop` is required".to_string(),
                });
                (None, String::new())
            }
            Some(Command::Play(from)) => {
                if let Some(tick) = from.start_tick {
                    negative(&mut refused, "/play/start_tick", tick);
                }
                match crate::compile(self.project.song(), &self.project.assets()?) {
                    Ok(plan) => {
                        // Absent is "from where it has reached" (ADR 0013 §2), which only the
                        // transport knows — so on a dry run with nothing playing, and in every
                        // determinism script, it is the start.
                        let reached = match self.preview.as_mut().map(Preview::latest) {
                            Some(Ok(event)) => event.map_or(0, |event| event.tick),
                            Some(Err(ended)) => {
                                self.preview = None;
                                return Err(ended);
                            }
                            None => 0,
                        };
                        let summary = match from.start_tick {
                            Some(tick) => format!("play from tick {tick}: {}", describe(&plan)),
                            None => format!("play from where it is: {}", describe(&plan)),
                        };
                        let start_tick = from.start_tick.unwrap_or(reached);
                        let play = preview_command::Command::Play(PreviewPlay {
                            plan: Some(plan),
                            start_tick,
                        });
                        (Some(play), summary)
                    }
                    Err(violations) => {
                        refused.extend(violations);
                        (None, String::new())
                    }
                }
            }
            Some(Command::Seek(seek)) => {
                idle(&mut refused, live, "/seek");
                negative(&mut refused, "/seek/tick", seek.tick);
                let summary = format!("seek to tick {}", seek.tick);
                (Some(preview_command::Command::Seek(PreviewSeek { tick: seek.tick })), summary)
            }
            Some(Command::Loop(range)) => {
                idle(&mut refused, live, "/loop");
                negative(&mut refused, "/loop/start_tick", range.start_tick);
                negative(&mut refused, "/loop/end_tick", range.end_tick);
                let summary = if range.end_tick > range.start_tick {
                    format!("loop ticks {} to {}", range.start_tick, range.end_tick)
                } else {
                    "no loop".to_string()
                };
                let command = PreviewLoop { start_tick: range.start_tick, end_tick: range.end_tick };
                (Some(preview_command::Command::Loop(command)), summary)
            }
            Some(Command::Stop(_)) => {
                idle(&mut refused, live, "/stop");
                (Some(preview_command::Command::Stop(PreviewStop {})), "stop".to_string())
            }
        };

        let Some(command) = command.filter(|_| refused.is_empty()) else {
            refused.sort();
            return Ok(PreviewResponse {
                valid: false,
                errors: refused.into_iter().map(wire).collect(),
                summary: String::new(),
                event: None,
            });
        };

        // The first half of the real path, as ever (ADR 0006 §3): compiled and checked, and no
        // process started and nothing sent. Its event is the transport's latest word, which a
        // dry run cannot have changed.
        //
        // Not quite pure, and deliberately: a transport that has ended since the last call is
        // reported here, and the dead `Preview` is dropped — which reaps its child — rather than
        // kept for the next real call to discover. That is session state and never the document
        // or the log, and it is what that next call would do anyway; `proto/song_tools.proto`
        // says so beside the dry-run claim (M2 PR 11).
        if request.dry_run {
            let event = match self.preview.as_mut().map(Preview::latest) {
                Some(Ok(event)) => event,
                Some(Err(ended)) => {
                    self.preview = None;
                    return Err(ended);
                }
                None => None,
            };
            return Ok(PreviewResponse { valid: true, errors: vec![], summary, event });
        }

        if self.preview.is_none() {
            let engine = self.engine.as_ref().ok_or_else(|| ProjectError {
                path: "--engine".to_string(),
                rule: "engine_unset",
                message: "this process was not told where the engine is, and does not look for one: \
                          pass `--engine <path to escribass_engine>` (ADR 0008 §2)"
                    .to_string(),
            })?;
            self.preview = Some(engine.preview()?);
        }
        let preview = self.preview.as_mut().expect("started above");
        match preview.send(PreviewCommand { command: Some(command) }) {
            Ok(event) => {
                Ok(PreviewResponse { valid: true, errors: vec![], summary, event: Some(event) })
            }
            Err(ended) => {
                self.preview = None;
                Err(ended)
            }
        }
    }

    // ---- undo and redo (ADR 0005 §4) ----
    //
    // The mechanism was decided a milestone ago and is not re-decided here: undo computes
    // `diff(current, materialise(parents[0]))` and commits it through the ordinary pipeline
    // under its own tool name. It never rewinds a ref, because a rewind *decrements* entity
    // `version` and §4.3's optimistic concurrency needs it monotonic — a client holding
    // version 5 of a track would later be handed a different version 5.
    //
    // What M2 adds is the pair of tools and the one thing ADR 0005 §4 named without
    // specifying: which change a press reverses. M2 PR 5 answered with a list held by the
    // session; M2 PR 11 reads the same answer off the log, which is where it was all along.

    /// The changes on this branch, oldest first, and how many of them are in effect: ⌘Z
    /// reverses `changes[done - 1]` and ⇧⌘Z restores `changes[done]`.
    ///
    /// **Read off the log, never held** (ADR 0005 §4, amended 2026-09-15). The first-parent
    /// chain from `HEAD` is replayed as an editor's undo stack: an `undo` entry steps back, a
    /// `redo` entry steps forward, and any other entry is a new change that discards whatever
    /// had been undone and not redone. That is the model M2 PR 5 implemented with a list in the
    /// session — and a list in the session is empty in every process that has just opened the
    /// project, so a relaunched window's ⌘Z started at `HEAD`, skipped the undos it had not
    /// made, and re-applied an edit they had already reversed. Every undo and redo is an entry
    /// (§5's audit trail), so the log already says where the key stands, in every session and on
    /// every branch alike.
    ///
    /// Three consequences, each of which the session's list got wrong or had to be told:
    ///
    /// - The log's own `undo` and `redo` entries are never changes to reverse, because ⌘Z
    ///   means the change before this one and not the entry before this one — after edit · undo
    ///   · redo, reversing the redo and then the undo takes the document *forwards*.
    /// - An edit made after an undo supersedes what was undone, so the undone edit leaves the
    ///   stack. The walk that skipped undo entries went on to land on it anyway, and ⌘Z there
    ///   was "no change" and ⇧⌘Z later restored it.
    /// - An entry with no operations is not a change either: a merge that keeps this branch's
    ///   value everywhere records one for the join (ADR 0015 §3), reversing it restores what is
    ///   already there, and nothing written could ever step past it. It still discards the undone.
    ///
    /// A branch switch needs no rule: another branch is another chain, so a redo can never
    /// restore a document from another line of history, and switching back finds its own
    /// history as it left it.
    ///
    /// `ponytail:` O(history) per press, parsing every entry's ops, which is `materialise`'s
    /// own cost and is paid beside it. Memoise per `HEAD` if a long log ever makes ⌘Z drag.
    fn undo_stack(
        &self,
    ) -> Result<(Vec<&escribass_schema::history::PatchEntry>, usize), ProjectError> {
        let history = self.project.history();
        let Some(head) = history.head_id() else { return Ok((Vec::new(), 0)) };
        let mut changes = Vec::new();
        let mut done: usize = 0;
        for entry in history.first_parents(head).map_err(|e| history_error(&e))? {
            match entry.tool.as_str() {
                // Saturating, and bounded by the changes there are, because only a hand-written
                // log can step past either end: the tools refuse to.
                "undo" => done = done.saturating_sub(1),
                "redo" => done = (done + 1).min(changes.len()),
                _ => {
                    changes.truncate(done);
                    if !ops_of(entry).map_err(|e| history_error(&e))?.is_empty() {
                        changes.push(entry);
                    }
                    done = changes.len();
                }
            }
        }
        Ok((changes, done))
    }

    /// §5 `undo`.
    pub fn undo(&mut self, request: &UndoRequest) -> Result<ToolResult, ProjectError> {
        // The first entry in a log has nothing before it, and neither does a chain of undos
        // that reaches it. Both are refusals a caller reads rather than operator errors:
        // pressing ⌘Z at the beginning of a history is an ordinary thing to do.
        let (changes, done) = self.undo_stack()?;
        let before = done
            .checked_sub(1)
            .and_then(|last| changes[last].parents.first().cloned());
        let Some(before) = before else {
            return Ok(refused(vec![Violation {
                path: "/".to_string(),
                rule: "nothing_to_undo",
                message: "there is no earlier state on this branch to go back to".to_string(),
            }]));
        };

        let restored =
            self.project.history().materialise(&before).map_err(|e| history_error(&e))?;
        self.restore("undo", &restored, request.dry_run)
    }

    /// §5 `redo`.
    pub fn redo(&mut self, request: &RedoRequest) -> Result<ToolResult, ProjectError> {
        let (changes, done) = self.undo_stack()?;
        let Some(target) = changes.get(done).map(|entry| entry.id.clone()) else {
            return Ok(refused(vec![Violation {
                path: "/".to_string(),
                rule: "nothing_to_redo",
                message: "nothing on this branch has been undone since its last change"
                    .to_string(),
            }]));
        };

        // The entry that was undone, materialised: the document as it stood *including* that
        // change. Redo is undo pointed the other way and shares every property with it,
        // including appending rather than rewinding.
        let restored =
            self.project.history().materialise(&target).map_err(|e| history_error(&e))?;
        self.restore("redo", &restored, request.dry_run)
    }

    /// Commits the patch that takes the current document to `restored`.
    ///
    /// Through `prepare_merge` and not `prepare`, for the reason a merge uses it: these ops
    /// carry entity `version`s, and they are **core's own** — read back out of a document core
    /// wrote. `prepare`'s guard would refuse the API's own history as though a caller had tried
    /// to write a field it does not own (ADR 0005 §3). The `max(ours, theirs) + 1` rule then
    /// takes every restored entity to *one past where it is now*, which is what keeps undo
    /// monotonic and is the whole reason ADR 0005 chose decision 2 to make decision 4 possible.
    fn restore(
        &mut self,
        tool: &'static str,
        restored: &Value,
        dry_run: bool,
    ) -> Result<ToolResult, ProjectError> {
        let current = serde_json::to_value(self.project.song()).expect("a Song serialises");
        let ops = diff(&current, restored);

        let prepared = match self.project.prepare_merge(&ops) {
            Ok(prepared) => prepared,
            Err(violations) => return Ok(refused(violations)),
        };
        // A call that changes nothing records nothing, as everywhere else. `undo_stack` skips
        // every entry that changed nothing, so on a log the tools wrote this is unreachable;
        // it is answered rather than recorded because an entry claiming a change is worse.
        if prepared.ops().is_empty() {
            return Ok(described("no change".to_string()));
        }

        let patch = ops_text(prepared.ops()).into_bytes();
        let summary = summarise(prepared.ops());
        if dry_run {
            return Ok(ToolResult {
                valid: true,
                errors: vec![],
                patch,
                summary,
                entry_id: String::new(),
            });
        }

        let Some(head) = self.project.history().head_id().map(str::to_string) else {
            return Err(ProjectError {
                path: self.project.root().display().to_string(),
                rule: "head_unset",
                message: "HEAD names no entry".to_string(),
            });
        };
        let entry_id = self.append(prepared, tool, vec![head])?;
        Ok(ToolResult { valid: true, errors: vec![], patch, summary, entry_id })
    }

    // ---- branches (ADR 0001 §2) ----
    //
    // These three do not go through `run`: they append no entry, so there is no patch to
    // prepare and nothing for the version rule to touch. What they change is `refs.json`, and
    // for `switch_branch` the document itself. `entry_id` stays empty, which is true rather
    // than a placeholder — a branch operation is not a commit.

    /// §5 `create_branch`.
    pub fn create_branch(
        &mut self,
        request: &CreateBranchRequest,
    ) -> Result<ToolResult, ProjectError> {
        let at = match tools::check_create_branch(self.project.history(), request) {
            Ok(at) => at,
            Err(violations) => return Ok(refused(violations)),
        };
        let summary = format!("branch `{}` at {at}", request.name);
        if request.dry_run {
            return Ok(described(summary));
        }
        self.project.create_branch(&request.name, &at)?;
        Ok(described(summary))
    }

    /// §5 `switch_branch`. Returns the patch that took the document to that branch.
    pub fn switch_branch(
        &mut self,
        request: &SwitchBranchRequest,
    ) -> Result<ToolResult, ProjectError> {
        let ops =
            match tools::check_switch_branch(self.project.history(), self.project.song(), request) {
                Ok(ops) => ops,
                Err(violations) => return Ok(refused(violations)),
            };
        let summary = format!("switch to `{}`: {}", request.name, summarise(&ops));
        let patch = ops_text(&ops).into_bytes();

        if !request.dry_run {
            self.project.switch_branch(&request.name)?;
        }
        Ok(ToolResult { valid: true, errors: vec![], patch, summary, entry_id: String::new() })
    }

    /// §5 `delete_branch`.
    pub fn delete_branch(
        &mut self,
        request: &DeleteBranchRequest,
    ) -> Result<ToolResult, ProjectError> {
        if let Err(violations) = tools::check_delete_branch(self.project.history(), request) {
            return Ok(refused(violations));
        }
        let summary = format!("delete `{}`; its entries stay in the log", request.name);
        if request.dry_run {
            return Ok(described(summary));
        }
        self.project.delete_branch(&request.name)?;
        Ok(described(summary))
    }

    /// §5 `merge_branch` (ADR 0001 §4, ADR 0015 §3).
    ///
    /// One entry with two parents, or a list of conflicts and nothing written. Since M2 PR 8
    /// there is a way out of the second: the conflicts come back exactly as they always did,
    /// a person names a side per path, and **the same call** is made again carrying the picks.
    /// No second tool, no session that remembers a half-finished merge, no merge that is
    /// partly committed — and `dry_run` on the second call means what it means on the first.
    pub fn merge_branch(
        &mut self,
        request: &MergeBranchRequest,
    ) -> Result<ToolResult, ProjectError> {
        let history = self.project.history();
        let Some(theirs_head) = history.refs().refs.get(&request.name).cloned() else {
            return Ok(refused(vec![Violation {
                path: "/name".to_string(),
                rule: "ref_missing",
                message: format!("`{}` is not a branch in this project", request.name),
            }]));
        };
        let Some(ours_head) = history.head_id().map(str::to_string) else {
            return Err(ProjectError {
                path: self.project.root().display().to_string(),
                rule: "head_unset",
                message: "HEAD names no entry".to_string(),
            });
        };
        if theirs_head == ours_head {
            return Ok(described(format!("`{}` is already here", request.name)));
        }

        let base = match history.merge_base(&ours_head, &theirs_head) {
            Ok(base) => base,
            Err(e) => {
                return Ok(refused(vec![Violation {
                    path: "/name".to_string(),
                    rule: e.rule,
                    message: e.message,
                }]))
            }
        };

        // Both sides as patches from the base. `ours` is re-derived from the document rather
        // than replayed, so a merge sees exactly what is on disk (ADR 0004).
        let ours_doc = serde_json::to_value(self.project.song()).expect("a Song serialises");
        let base_doc = history.materialise(&base).map_err(|e| history_error(&e))?;
        let theirs_doc = history.materialise(&theirs_head).map_err(|e| history_error(&e))?;
        let ours = diff(&base_doc, &ours_doc);
        let theirs = diff(&base_doc, &theirs_doc);

        // Asked before the conflicts are computed, and kept that way: a branch with nothing
        // this one lacks has nothing to conflict about either, and the two answers are not the
        // same. "Nothing to merge" records no entry; a merge whose every conflict was resolved
        // in this branch's favour has an empty patch and still records one, below.
        if theirs.is_empty() {
            return Ok(described(format!("`{}` has nothing this branch lacks", request.name)));
        }

        let conflicts = crate::merge::conflicts(&base_doc, &ours, &theirs);
        let picked = |path: &str| {
            MergeSide::try_from(request.resolve.get(path).copied().unwrap_or_default())
                .unwrap_or(MergeSide::Unspecified)
        };

        // A resolution names a path *this* merge is in conflict about. One that does not is
        // refused rather than ignored: taken at face value it would drop the other side's
        // change at a path nothing disagreed about — editing the merge rather than resolving
        // it — and taken as a typo it would leave a person believing they had chosen something
        // (ADR 0015 §3). It is also how a caller learns the conflicts moved under a held
        // preview, which is the merge's version of ADR 0012 §4's optimistic apply.
        let disputed: std::collections::BTreeSet<&str> =
            conflicts.iter().map(|c| c.path.as_str()).collect();
        let unknown: Vec<Violation> = request
            .resolve
            .keys()
            .filter(|path| !disputed.contains(path.as_str()))
            .map(|path| Violation {
                // The path as the caller sent it, which is a pointer into the document like
                // every other `Violation.path` — and not `/resolve/<path>`, which would need
                // RFC 6901 to escape a pointer inside a pointer for no reader's benefit.
                path: path.clone(),
                rule: "resolution_unknown",
                message: format!(
                    "`{path}` is not one of this merge's conflicts; resolve the paths \
                     `merge_conflict` named and nothing else"
                ),
            })
            .collect();
        if !unknown.is_empty() {
            return Ok(refused(unknown));
        }

        // Every conflict nobody has settled is still a conflict, reported as it was before
        // there was anything to settle it with. `MERGE_SIDE_UNSPECIFIED` is not a third choice
        // — it is the absence of one, which is what a path left alone in the map means too.
        let unsettled: Vec<Violation> = conflicts
            .iter()
            .filter(|c| picked(&c.path) == MergeSide::Unspecified)
            .cloned()
            .collect();
        if !unsettled.is_empty() {
            return Ok(refused(unsettled));
        }

        // Resolution is a **filter on the other side's operations**, and that is the whole
        // mechanism. Theirs wins at a path by being applied, which is what an unconflicted
        // merge already does to every op; ours wins by that op not being applied at all. So
        // there is no third value to compute, no strategy to pick and no code path a merge
        // without conflicts does not already take (ADR 0015 §3).
        //
        // `merge::conflicts` reports a conflict at the *other* side's op path, so the map's
        // keys and the ops filtered here are the same strings; nothing has to translate.
        //
        // `ponytail:` choosing theirs at a path whose parent this branch removed is refused
        // rather than grafted — the op arrives as `path_not_found` from `prepare_merge`, which
        // is loud. Resolving at the parent path is the answer; a merge that reconstructs a
        // subtree is the strategy framework this decision exists to avoid.
        let theirs: Vec<Op> =
            theirs.into_iter().filter(|op| picked(op.path()) != MergeSide::Ours).collect();

        // The incoming side's ops applied on top of ours. `prepare_merge` rather than
        // `prepare`: these carry entity versions, which are core's own and are what ADR 0005
        // §2 resolves to max + 1.
        let prepared = match self.project.prepare_merge(&theirs) {
            Ok(prepared) => prepared,
            Err(violations) => return Ok(refused(violations)),
        };
        let patch = ops_text(prepared.ops()).into_bytes();
        let summary = format!("merge `{}`: {}", request.name, summarise(prepared.ops()));

        if request.dry_run {
            return Ok(ToolResult {
                valid: true,
                errors: vec![],
                patch,
                summary,
                entry_id: String::new(),
            });
        }

        // Two parents, which is the only thing that distinguishes a merge from a commit at
        // this level (ADR 0001 §1) — and the reason this is the one place an entry with no
        // operations is written rather than refused as "no change". Resolving every conflict
        // in this branch's favour changes the document not at all and still joins the two
        // lines of history: without the entry the branches stay unmerged, `merge_base` finds
        // the old base, and the same conflict is reported again for ever.
        let entry_id = self.append(prepared, "merge_branch", vec![ours_head, theirs_head])?;
        Ok(ToolResult { valid: true, errors: vec![], patch, summary, entry_id })
    }

    // ---- the proposal (ADR 0019) ----
    //
    // A turn's calls do not reach the project. They reach a copy of it, and what a person
    // approves is the one patch from the document as it stands to the document those calls
    // produced. The reason is measured rather than assumed: of 116 successful spike runs, not
    // one built a multi-call edit out of dry runs, because a dry run mints its ids from a fork
    // it then puts back — so the second call of "add a track, then a clip on it" names a track
    // that exists in a document that was thrown away (docs/plan.md, M3 trap 8).

    /// Opens the proposal one turn's calls run against (ADR 0019 §1).
    ///
    /// `prompt_id` is the SHA-256 of the prompt's text, computed by the host with
    /// [`crate::asset_hash`] — content-addressed as an asset is, so the same text asked twice
    /// names the same id and a scripted transcript replays to the same log (ADR 0021 §2).
    ///
    /// **One at a time.** A second prompt while one is pending is refused rather than stacking
    /// a second fork on the first or dropping what a person has not read (ADR 0019 §3). It is
    /// an `Err` and not a `Violation` because nothing the caller can say differently fixes it:
    /// a person applies or rejects, and only then does the next prompt go anywhere.
    pub fn propose(&mut self, prompt_id: &str) -> Result<(), ProjectError> {
        if self.proposal.is_some() {
            return Err(ProjectError {
                path: "/".to_string(),
                rule: "proposal_pending",
                message: "apply or reject the pending proposal first: one turn is one proposal \
                          (ADR 0019 §3)"
                    .to_string(),
            });
        }
        self.proposal = Some(Box::new(Proposal::forked(self, prompt_id.to_string())));
        Ok(())
    }

    /// The proposal this session is holding, if it is holding one.
    pub fn proposal(&self) -> Option<&Proposal> {
        self.proposal.as_deref()
    }

    /// The same, to run a call against.
    pub fn proposal_mut(&mut self) -> Option<&mut Proposal> {
        self.proposal.as_deref_mut()
    }

    /// Drops the proposal. Nothing was written, so nothing is undone and the log records
    /// nothing (ADR 0019 §3).
    ///
    /// Answers whether there was one, because "rejected" and "there was nothing to reject" are
    /// different things for the panel to say.
    pub fn reject(&mut self) -> bool {
        self.proposal.take().is_some()
    }

    /// Commits the proposal as **one entry**, under the tool name `proposal` (ADR 0019 §2).
    ///
    /// `model_id` is the model the turn's **last response** named — what actually answered,
    /// never the id that was asked for (ADR 0021 §2).
    ///
    /// Three things happen here and each is somebody's decision, not this function's. The
    /// patch is the proposal's ops, which are `diff` from the document as it stood to the
    /// document the calls produced with the fork's per-call version bumps put back
    /// ([`Proposal::ops`]). It goes through [`Session::run`] like every other mutating tool,
    /// so there is no second commit path and the version rule is applied by the same code to
    /// the same shape. And the document having **moved** underneath is refused — by
    /// [`Proposal::moved_under`] first, which compares the paths the patch touches against the
    /// document the patch was computed from, and by ADR 0005 §3's guard inside `run` after it
    /// — after which the proposal is still pending, because a refusal is something a person
    /// acts on rather than a reason to throw their turn away.
    pub fn apply_proposal(&mut self, model_id: &str) -> Result<ToolResult, ProjectError> {
        let Some(proposal) = self.proposal.take() else {
            return Ok(refused(vec![Violation {
                path: "/".to_string(),
                rule: "no_proposal",
                message: "there is no proposal to apply".to_string(),
            }]));
        };
        // **Before anything is prepared** (ADR 0019 §2, amended 2026-09-24). The version rule
        // cannot see an edit that moved an entity by exactly one, and a proposal sits pending
        // for as long as a person takes to read it — minutes, where a drag is milliseconds —
        // so that is the likeliest concurrent edit there is, not the rarest.
        let moved = proposal.moved_under(self.project.song());
        if !moved.is_empty() {
            let why = moved
                .into_iter()
                .map(|path| Violation {
                    path,
                    rule: "document_moved",
                    message: "this changed after the proposal was composed, and applying the \
                              proposal would write over it. Reject and ask again — the next \
                              prompt reads the document as it is now (ADR 0019 §2)"
                        .to_string(),
                })
                .collect();
            self.proposal = Some(proposal);
            return Ok(refused(why));
        }
        // The patch as it was computed against the document the fork was taken from, carrying
        // version claims of `before + 1`. `run` prepares it again against the document as it
        // stands *now*, and that second prepare is where a person's concurrent edit is refused.
        let prepared = match proposal.patch() {
            Ok(prepared) => prepared,
            Err(violations) => {
                self.proposal = Some(proposal);
                return Ok(refused(violations));
            }
        };
        let ops = prepared.ops().to_vec();
        // The entry's own provenance: the model, the prompt, and **no** `tool_call_id` — a
        // composed proposal has several, and the entities carry theirs (ADR 0021 §2, §4).
        let entry = Provenance {
            author: Author::Model as i32,
            model_id: Some(model_id.to_string()),
            prompt_id: Some(proposal.prompt_id.clone()),
            tool_call_id: None,
            created_at: Some(self.clock.now()),
        };
        let made = Made::Proposal(entry);
        // **The session moves past every id the proposal minted, and this is where.**
        // `from_tool` does the same for a single call and for the same two reasons: the entry
        // `record` mints comes *after* the ids the call used, and the session never mints one
        // an entity already has. Without it the entry took the number the proposal's first
        // track had — one ULID on two things — and the session's next call would have minted
        // the instrument's. Measured rather than reasoned: the end-to-end golden's `proposal`
        // entry and its Lead track were the same 26 characters.
        //
        // `ahead` rather than "adopt the fork", because a turn is long enough for a person to
        // edit while it runs, and their entries mint from the session's line. Taking the fork
        // back then takes the session *backwards*, which is `entry_exists` on the next commit
        // — which is what the test for the guard's blind spot caught when this was a swap.
        let onwards = ahead(self.ids.fork(), proposal.fork.ids.fork());
        let spent = std::mem::replace(&mut self.ids, onwards);
        let result = self.making(made, |session| session.run("proposal", &ops, false));
        // Kept on anything but a clean commit, including a refusal: Reject is a person's
        // decision and this is not it (ADR 0019 §3). The ids go back with it, because a
        // proposal that did not apply has minted nothing that exists.
        if !matches!(&result, Ok(outcome) if outcome.valid) {
            self.ids = spent;
            self.proposal = Some(proposal);
        }
        result
    }

    /// **Edit**: the proposal's patch, rewritten by a person and applied as the person's
    /// (ADR 0019 §3).
    ///
    /// §9's third control, and the one place in the application where a person writes RFC 6902
    /// by hand. It is an ordinary `apply_patch` — the entry's `tool` says so — with two things
    /// the raw pipeline cannot know on its own: the provenance is `AUTHOR_HUMAN` with **no**
    /// `model_id`, because a person who changed the bytes owns the bytes; and the `prompt_id`
    /// is kept, so the entry still leads to the conversation the patch was made from.
    ///
    /// **The refusal path is ADR 0017 §3's**: a patch that will not apply comes back as a
    /// refusal with its rules and the proposal is **still pending**, because a patch a person
    /// mistyped is not a reason to throw their turn away. The same is true of a patch that
    /// applies to nothing — `run` answers "no change" and records no entry, and there is still
    /// a proposal to decide.
    ///
    /// The session takes the proposal's ids when the edit commits, for [`Session::apply_proposal`]'s
    /// reason and by the same comparison: the text a person edited names ids the **fork**
    /// minted, so applying it puts them in the project, and a session that had not moved past
    /// them would mint one of them again.
    pub fn edit_proposal(&mut self, patch: &[u8]) -> Result<ToolResult, ProjectError> {
        let Some(proposal) = self.proposal.take() else {
            return Ok(refused(vec![Violation {
                path: "/".to_string(),
                rule: "no_proposal",
                message: "there is no proposal to edit".to_string(),
            }]));
        };
        // Parsed here rather than through `apply_patch`, because this is the one call that has
        // to keep the proposal when it is refused and `apply_patch` knows nothing about one.
        let ops: Vec<Op> = match serde_json::from_slice(patch) {
            Ok(ops) => ops,
            Err(e) => {
                self.proposal = Some(proposal);
                return Ok(refused(vec![Violation {
                    path: String::new(),
                    rule: "patch_unreadable",
                    message: format!("the edited patch is not an RFC 6902 array: {e}"),
                }]));
            }
        };
        let made = Made::Edit(Provenance {
            author: Author::Human as i32,
            model_id: None,
            prompt_id: Some(proposal.prompt_id.clone()),
            tool_call_id: None,
            created_at: Some(self.clock.now()),
        });
        let onwards = ahead(self.ids.fork(), proposal.fork.ids.fork());
        let spent = std::mem::replace(&mut self.ids, onwards);
        let result = self.making(made, |session| session.run("apply_patch", &ops, false));
        // An entry is what says the edit happened. A refusal keeps the proposal for the reason
        // above; so does "no change", where a person's patch applied to nothing at all and the
        // proposal they were editing is still the thing in front of them.
        if !matches!(&result, Ok(outcome) if outcome.valid && !outcome.entry_id.is_empty()) {
            self.ids = spent;
            self.proposal = Some(proposal);
        }
        result
    }

    /// The provider and model this project records, or the baseline default when it records
    /// none (ADR 0021 §4).
    pub fn ai(&self) -> (&str, &str) {
        match self.project.ai() {
            Some(ai) => (ai.provider.as_str(), ai.model.as_str()),
            None => (DEFAULT_PROVIDER, DEFAULT_MODEL),
        }
    }

    /// Records the provider and model this project's prompts go to, on first use
    /// (ADR 0021 §4). A project that already records one keeps it.
    pub fn record_ai(&mut self, provider: &str, model: &str) -> Result<(), ProjectError> {
        self.project.record_ai(provider, model)
    }

    /// A tool that refused its arguments is refused the same way a patch that will not apply
    /// is: a result, not a failure (ADR 0006 §2). Nothing distinguishes the two for a caller,
    /// which is the point — both are things it can fix by calling differently.
    /// A tool builds its patch from a **fork** of the id source; this decides whether the fork
    /// is kept.
    ///
    /// It is installed for the duration of the call, so the entry `record` mints comes after
    /// the ids the tool used and the sequence is exactly what it would have been. It is put
    /// back if the call kept nothing — a dry run, or a refusal — so previewing burns no id and
    /// the apply that follows mints exactly what the preview showed (ADR 0006 §3, which §9
    /// needs, since a person approves a patch before it is applied).
    fn from_tool(
        &mut self,
        tool: &str,
        built: Result<Vec<Op>, Vec<Violation>>,
        dry_run: bool,
        ids: Box<dyn IdSource + Send>,
    ) -> Result<ToolResult, ProjectError> {
        let Ok(ops) = built else {
            return Ok(refused(built.expect_err("just matched")));
        };
        let previous = std::mem::replace(&mut self.ids, ids);
        let result = self.run(tool, &ops, dry_run);
        if dry_run || !matches!(&result, Ok(outcome) if outcome.valid) {
            self.ids = previous;
        }
        result
    }

    /// Every mutating tool ends here: prepare, and either describe it or record it.
    ///
    /// Private because the typed tools of the next steps are the surface; what they share is
    /// this function, not a trait.
    fn run(&mut self, tool: &str, ops: &[Op], dry_run: bool) -> Result<ToolResult, ProjectError> {
        // Every mutating tool passes through here, which is what ADR 0002 §4 means by "the
        // tool API's job, on input". Normalising only in `apply_patch` left `set_param(-0.0)`
        // to be refused by the validator's `negative_zero` — a rule that should never fire on
        // input that came through a tool.
        let mut ops = ops.to_vec();
        normalise_input(&mut ops);
        let ops = &ops[..];

        // The session's author is the default for every call that arrives with no other word
        // — the binaries' `--author`, the window's `Human` — and no request message gains an
        // author field, because a field on the wire is a field a caller can lie in. The three
        // optional ids reach the log through the proposal, which is the only place a model's
        // calls enter (ADR 0021 §1, §2; ADR 0020 §1).
        let prepared = match &self.made {
            // A proposal's patch carries provenance `prepare` already decided on the fork, so
            // this is the one caller whose two §4.3 exemptions come apart: versions guarded,
            // provenance kept (ADR 0019 §2, ADR 0021 §2).
            Made::Proposal(_) => self.project.prepare_proposal(ops),
            // An edited patch goes through the ordinary `prepare`, which is the point of the
            // arm: the entities the model minted carry its provenance in the text a person
            // was handed, and the walk replaces a *new* entity's with the call's — which is
            // this person's (ADR 0021 §1, ADR 0019 §3).
            Made::Call(made) | Made::Edit(made) => self.project.prepare(ops, made),
            Made::Session => self
                .project
                .prepare(ops, &crate::project::authorship(self.author, &*self.clock)),
        };
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(violations) => return Ok(refused(violations)),
        };

        // A call that changes nothing records nothing. An entry with no operations is a claim
        // that something happened, and `merge_branch` already answers this case that way.
        if prepared.ops().is_empty() {
            return Ok(described("no change".to_string()));
        }

        // Built from `prepared` before anything is written, so a dry run and the commit that
        // follows it cannot describe different patches.
        let patch = ops_text(prepared.ops()).into_bytes();
        let summary = summarise(prepared.ops());

        if dry_run {
            return Ok(ToolResult {
                valid: true,
                errors: vec![],
                patch,
                summary,
                entry_id: String::new(),
            });
        }

        // **A proposal's call keeps its document and writes nothing** (ADR 0019 §1). This is
        // the one branch here that is not a commit, and it is what makes the second call of
        // "add a track, then a clip on it" possible: the track is in the fork, under the id the
        // first call's result returned, and the project has not been touched. The empty
        // `entry_id` is true rather than a placeholder — a person approves the whole, once,
        // when the turn ends (decision 2).
        if matches!(self.made, Made::Call(_)) {
            self.project.keep(prepared);
            return Ok(ToolResult {
                valid: true,
                errors: vec![],
                patch,
                summary,
                entry_id: String::new(),
            });
        }

        let entry_id = self.commit(prepared, tool)?;
        Ok(ToolResult { valid: true, errors: vec![], patch, summary, entry_id })
    }

    fn commit(&mut self, prepared: Prepared, tool: &str) -> Result<String, ProjectError> {
        let Some(head) = self.project.history().head_id().map(str::to_string) else {
            return Err(ProjectError {
                path: self.project.root().display().to_string(),
                rule: "head_unset",
                message: "HEAD names no entry".to_string(),
            });
        };
        self.append(prepared, tool, vec![head])
    }

    /// The one place this session appends to the log.
    ///
    /// It used to be the one place the undo cursor was cleared as well. There is no cursor to
    /// clear: an entry that is not an undo or a redo discards what had been undone by being in
    /// the log, which `undo_stack` reads (ADR 0005 §4, amended 2026-09-15).
    /// Runs `doing` with `made` in place, and puts the previous one back **whatever happens**.
    ///
    /// A `mem::replace` … restore pair is only a pair while nothing unwinds between the two
    /// halves, and something can: a panic anywhere inside the commit pipeline left `made` at
    /// `Made::Proposal(…)` for the rest of the session's life. The Tauri host recovers a
    /// poisoned mutex rather than propagating it (`app/src-tauri/src/main.rs`) — which is the
    /// right call for one bad call, and is what turned this into a live hole: **every later
    /// human commit was then recorded as the model's**, which is exactly the forgery ADR 0021
    /// §1 closed. Found by the M3 review, 2026-09-24.
    ///
    /// `ponytail:` `catch_unwind` rather than a drop guard, because a guard that restored a
    /// field of `Session` would have to hold `&mut Session` and nothing could then use it. The
    /// panic is re-raised unchanged — this decides nothing about the failure, only that the
    /// session does not keep the model's identity afterwards.
    fn making<T>(&mut self, made: Made, doing: impl FnOnce(&mut Self) -> T) -> T {
        let previously = std::mem::replace(&mut self.made, made);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| doing(self)));
        self.made = previously;
        match outcome {
            Ok(answered) => answered,
            Err(panicked) => std::panic::resume_unwind(panicked),
        }
    }

    fn append(
        &mut self,
        prepared: Prepared,
        tool: &str,
        parents: Vec<String>,
    ) -> Result<String, ProjectError> {
        let made = match &self.made {
            Made::Session => crate::project::authorship(self.author, &*self.clock),
            // A proposal's entry names the model and the prompt as well (ADR 0021 §2); an
            // edited one names the person and the prompt (ADR 0019 §3).
            Made::Call(made) | Made::Proposal(made) | Made::Edit(made) => made.clone(),
        };
        self.project.record(prepared, tool, parents, &made, &mut *self.ids)
    }
}

/// The model's calls, applied to a fork of the document, writing nothing (ADR 0019 §1).
///
/// **A proposal is a session on a copy of the project that records nothing.** That sentence is
/// the whole design and it is why this type is forty lines rather than four hundred: the tool
/// functions, the validator, `prepare`, the version rule and the one dispatch
/// [`crate::call::call`] are all the ones a person's edit goes through, run against
/// `Project::clone` instead of the project. Nothing about what is *accepted* differs by author
/// (docs/plan.md, M3 trap 18); what differs is where it lands, and the carrier decides that by
/// calling this instead of the session (ADR 0019 §1).
///
/// Two things the fork keeps that a dry run does not, and they are the two that matter:
///
/// 1. **The ids**, because the fork is never put back. So the track `add_track` minted is
///    still there, under the id its result returned, when `add_clip` names it — which is what
///    every model in the spike did with the ids a real apply returned, and what no dry run can
///    offer (ADR 0012 §4; M3 trap 8).
/// 2. **The document**, because a valid call swaps the prepared song in. So the view the model
///    reads between calls is the document as its own calls have left it, which is ADR 0018 §2's
///    "re-read after every applied call" with "applied" meaning applied to the proposal.
///
/// What it does not keep is anything on disk. `Project::write` is paid once, at approval, which
/// is ADR 0017 §1's second reason arriving unchanged and the answer to the cost a branch per
/// proposal would have had (docs/plan.md, M3 trap 9).
pub struct Proposal {
    /// The session the calls run on: this project, cloned, with a fork of the id source and a
    /// copy of the clock. Its author is `Model` and its `made` is replaced per call.
    fork: Box<Session>,
    /// The project as it stood when the turn began.
    ///
    /// The patch is computed against **this** rather than against wherever the project has got
    /// to since, and that is the whole of how a pending proposal meets a concurrent edit: the
    /// patch then carries version claims of `before + 1`, which are *stale* if a person has
    /// edited an entity it touched, and a stale claim is what ADR 0005 §3's guard refuses
    /// (ADR 0019 §2; ADR 0012 §4). Preparing against the moved document instead would compute
    /// claims that fit, and merge over their edit in silence.
    ///
    /// `ponytail:` a whole `Project` clone, log included, because `prepare` is a method on one
    /// and this is one line rather than a constructor for a project positioned at a document
    /// with no history. It is paid once per turn, beside a `Song` clone that is paid per call.
    /// If a long log ever makes opening a turn drag, that constructor is the thing to add.
    base: Project,
    /// The SHA-256 of the prompt's text (ADR 0021 §2). Every entity the turn mints carries it,
    /// and so does the entry.
    prompt_id: String,
    /// The tools this turn has called, in order, accepted ones only.
    ///
    /// Not in the entry and never on its way there: `history.proto` gains no field for them and
    /// a list of names would not explain itself without its arguments, which are the ops
    /// (ADR 0019 §4). It is what the turn reports to a person while the proposal is still
    /// pending — "three calls" — and the conversation is where the calls themselves live.
    calls: Vec<String>,
}

impl Proposal {
    /// Forks `from`'s project, ids and clock. Writes nothing and mints nothing.
    fn forked(from: &Session, prompt_id: String) -> Self {
        let fork = Session {
            project: from.project.clone(),
            ids: from.ids.fork(),
            clock: from.clock.fork(),
            // The proposal's own author, never the session's: `app` keeps `Author::Human` and
            // the model's calls never enter through it (ADR 0021 §1, Consequences).
            author: Author::Model,
            // A proposal renders nothing and plays nothing: neither `render_export` nor
            // `render_preview` is offered, and an engine the model cannot hear is the reason
            // (ADR 0022 §1, CLAUDE.md #6).
            engine: None,
            // The sandbox **is** inherited, where the engine is not, and the difference is
            // what the model is offered: `compile_generator` is, from M4 PR 6, and a compile
            // on a proposal spawns the sandbox and writes the fork's clip, which a person
            // then applies whole (ADR 0026 §3). It writes no file and plays no sound, so
            // CLAUDE.md #6 has nothing to say about it.
            sandbox: from.sandbox.clone(),
            preview: None,
            // One proposal at a time, structurally: a fork has no fork (ADR 0019 §3).
            proposal: None,
            made: Made::Session,
        };
        Proposal {
            base: from.project.clone(),
            fork: Box::new(fork),
            prompt_id,
            calls: Vec::new(),
        }
    }

    /// The document as the proposal's calls have left it. What the view is computed from, and
    /// what crosses back on every `CallResult` (ADR 0018 §2, ADR 0020 §3).
    pub fn song(&self) -> &Song {
        self.fork.project.song()
    }

    /// The prompt whose turn this is (ADR 0021 §2).
    pub fn prompt_id(&self) -> &str {
        &self.prompt_id
    }

    /// The tools this turn has called, accepted ones only.
    pub fn calls(&self) -> &[String] {
        &self.calls
    }

    /// Runs one of the model's calls against the fork (ADR 0019 §1).
    ///
    /// `model_id` is the model the response that made this call named; `tool_call_id` is the id
    /// the provider assigned it. Both are stamped on whatever the call mints, so an entity in
    /// the log leads back to the line of the conversation that created it (ADR 0021 §2).
    ///
    /// **A tool the model was not offered is refused here**, and this is the only gate: it is a
    /// statement about what is *offered* to one author and not about what is accepted, which is
    /// unchanged for every caller (ADR 0022 §1; M3 trap 18). It is also what keeps the fork
    /// harmless — `create_branch`, `switch_branch` and `add_asset` write to the project root
    /// straight out, which on a clone of a live project would be the real one.
    ///
    /// A model that sends `dry_run: true` from habit gets a dry run **of the fork** — prepared
    /// and described, with nothing swapped in — so the models that preview first get the same
    /// answers as the ones that do not (ADR 0022 §1).
    pub fn call(
        &mut self,
        name: &str,
        arguments: &Map<String, Value>,
        model_id: &str,
        tool_call_id: &str,
    ) -> Result<Answer, CallError> {
        if !OFFERED.contains(&name) {
            return Err(CallError {
                kind: ErrorKind::BadRequest,
                message: format!(
                    "`{name}` is not one of the tools you were offered; call one of: {}",
                    OFFERED.join(", ")
                ),
            });
        }
        self.fork.made = Made::Call(Provenance {
            author: Author::Model as i32,
            model_id: Some(model_id.to_string()),
            prompt_id: Some(self.prompt_id.clone()),
            tool_call_id: Some(tool_call_id.to_string()),
            created_at: Some(self.fork.clock.now()),
        });
        let answered = crate::call::call(&mut self.fork, name, arguments);
        self.fork.made = Made::Session;
        if matches!(&answered, Ok(answer) if !answer.refused) {
            self.calls.push(name.to_string());
        }
        answered
    }

    /// The operations that take the document as it stood to the document the model's calls
    /// produced (ADR 0019 §2).
    ///
    /// The fork's per-call version bumps are scaffolding and never reach the log: a track the
    /// model edited three times stands at `before + 3` on the fork and at `before + 1` in the
    /// entry, because what a person approves is one change. [`restore_versions`] puts the
    /// numbers back where they started, and the single `prepare` these ops then go through
    /// computes exactly what one call producing this document would have recorded.
    fn ops(&self) -> Vec<Op> {
        let before = serde_json::to_value(self.base.song()).expect("a Song serialises");
        let mut proposed = serde_json::to_value(self.song()).expect("a Song serialises");
        restore_versions(&before, &mut proposed);
        diff(&before, &proposed)
    }

    /// Every path this proposal's operations touch whose value in the project **is no longer
    /// what it was** when the fork was taken (ADR 0019 §2, amended 2026-09-24).
    ///
    /// ADR 0005 §3's version guard has a blind spot and this is where it is closed. A
    /// proposal's patch claims `before + 1`; after exactly one intervening entry the entity
    /// *holds* `before + 1`, and a claim equal to the number core computes reads as disputing
    /// nothing — so one edit to an entity the proposal touched merged over that edit in
    /// silence. `core/tests/proposal.rs` proved it, and ADR 0019's Consequences wrote it down,
    /// while §5 went on saying any such edit was refused.
    ///
    /// **This does not change the version rule**, which is ADR 0005's to change and is a held
    /// drag's as much as a proposal's. It adds the thing a proposal has and a drag does not:
    /// [`Proposal::base`], the document the patch was computed against. Comparing the ops'
    /// own paths in that document with the same paths now answers the question the version
    /// number was standing in for — *did what I am about to overwrite move?* — exactly, at
    /// the granularity the patch writes at, and it names what moved rather than a number.
    ///
    /// An edit **elsewhere** is still not a dispute and still merges: a path no operation
    /// touches is not compared. A path neither document has — the id of a track the proposal
    /// is adding — compares equal, which is the right answer and not a special case.
    fn moved_under(&self, now: &Song) -> Vec<String> {
        let was = serde_json::to_value(self.base.song()).expect("a Song serialises");
        let now = serde_json::to_value(now).expect("a Song serialises");
        let mut moved: Vec<String> = Vec::new();
        for op in self.ops() {
            let path = op.path();
            if was.pointer(path) != now.pointer(path) && !moved.iter().any(|seen| seen == path) {
                moved.push(path.to_string());
            }
        }
        moved
    }

    /// **The patch a person approves** (ADR 0019 §2): those operations as one `prepare` records
    /// them, version bumps included.
    ///
    /// Read-only and computable after every call, which is what lets the panel draw the
    /// proposal as it grows (PR 9). It is also exactly what Apply sends, so what a person
    /// approved and what is committed are one value and not two computations of it.
    pub fn patch(&self) -> Result<Prepared, Vec<Violation>> {
        self.base.prepare_proposal(&self.ops())
    }
}

/// Whichever of two id sources is further along.
///
/// Ids sort in the order they are minted (ADR 0001 §5), so "further along" is a comparison of
/// what each would mint **next** — read from a fork of each, which burns nothing on either.
///
/// The two sources here share an origin: a proposal forks the session's at the start of a turn
/// and keeps it (ADR 0019 §1), while the session goes on minting for whatever a person does
/// meanwhile. Both are monotone, so the one that is ahead has produced a superset of the
/// other's numbers and taking it is what keeps every id in the project distinct.
///
/// `ponytail:` under [`crate::id::UlidSource`] the comparison is exact within a millisecond and
/// arbitrary across one, because a new millisecond takes fresh entropy — and across one it does
/// not matter, since two sources that both re-seeded are 80 bits apart. Where it has to be
/// exact is where it is: [`crate::id::SeededIds`], which is a counter and is what every golden
/// in this repository is minted by.
fn ahead(
    one: Box<dyn IdSource + Send>,
    other: Box<dyn IdSource + Send>,
) -> Box<dyn IdSource + Send> {
    if one.fork().next_id() >= other.fork().next_id() {
        one
    } else {
        other
    }
}

/// A refused call: no patch, no entry, and every reason it was refused (§5, §6).
fn refused(violations: Vec<Violation>) -> ToolResult {
    ToolResult {
        valid: false,
        errors: violations.into_iter().map(wire).collect(),
        patch: Vec::new(),
        summary: String::new(),
        entry_id: String::new(),
    }
}

/// A call that succeeded and produced no patch: the branch tools, which move a ref rather than
/// the document. An empty `patch` here says "nothing changed in the song", which is true.
fn described(summary: String) -> ToolResult {
    ToolResult {
        valid: true,
        errors: vec![],
        patch: Vec::new(),
        summary,
        entry_id: String::new(),
    }
}

/// A refused render: no engine ran, and every reason it was refused (ADR 0007 §6).
fn render_refused(violations: Vec<Violation>) -> RenderResponse {
    RenderResponse {
        valid: false,
        errors: violations.into_iter().map(wire).collect(),
        summary: String::new(),
        result: None,
    }
}

/// One deterministic line describing a plan.
///
/// From the plan rather than from the request, so it says what will be rendered and carries no
/// path: `output_path` is machine-specific, and a summary is compared byte for byte by the
/// determinism suite (§11).
fn describe(plan: &escribass_proto::render::RenderPlan) -> String {
    let tracks = plan.tracks.len();
    let clips: usize = plan.tracks.iter().map(|t| t.clips.len()).sum();
    format!(
        "{} ticks, {tracks} track{}, {clips} clip{}",
        plan.length_ticks,
        if tracks == 1 { "" } else { "s" },
        if clips == 1 { "" } else { "s" },
    )
}

/// `Violation` as it crosses the wire. Not a fifth error type — the transport of the one
/// `core` already returns everywhere (ADR 0006 §2).
fn wire(violation: Violation) -> escribass_proto::tools::Violation {
    escribass_proto::tools::Violation {
        path: violation.path,
        rule: violation.rule.to_string(),
        message: violation.message,
    }
}

/// A `HistoryError` reaching a caller. Same three fields, so no translation is needed beyond
/// the type.
fn history_error(e: &HistoryError) -> ProjectError {
    ProjectError { path: e.path.clone(), rule: e.rule, message: e.message.clone() }
}

/// One deterministic line describing a patch (§5's `summary`).
///
/// Generated from the diff rather than written per tool: two identical calls must produce
/// identical bytes (§11), and a sentence composed by a tool is a second description of what
/// the ops already say. M3 can enrich this; it cannot make it non-deterministic.
fn summarise(ops: &[Op]) -> String {
    const SHOWN: usize = 3;
    if ops.is_empty() {
        return "no change".to_string();
    }
    let paths: Vec<&str> = ops.iter().map(Op::path).take(SHOWN).collect();
    let more = ops.len().saturating_sub(SHOWN);
    format!(
        "{} op{}: {}{}",
        ops.len(),
        if ops.len() == 1 { "" } else { "s" },
        paths.join(", "),
        if more > 0 { format!(", and {more} more") } else { String::new() },
    )
}

/// Normalises values entering through the tool API (ADR 0002 §4).
///
/// `-0.0` is the one that matters: it is a distinct double that serialises as `-0.0`, so a
/// song carrying one is not byte-identical to the same song carrying `0.0`, and §11's
/// determinism claim quietly weakens. ADR 0002 §4 assigns the fix to this boundary — the
/// validator rejects what gets past it (`negative_zero`), and this is what keeps that rule
/// from ever firing on ordinary input.
fn normalise_input(ops: &mut [Op]) {
    for op in ops {
        match op {
            Op::Add { value, .. } | Op::Replace { value, .. } | Op::Test { value, .. } => {
                normalise(value)
            }
            Op::Remove { .. } | Op::Copy { .. } | Op::Move { .. } => {}
        }
    }
}

fn normalise(value: &mut Value) {
    match value {
        Value::Number(n) => {
            if n.as_f64() == Some(0.0) && n.as_f64().is_some_and(|f| f.is_sign_negative()) {
                *value = Value::from(0.0);
            }
        }
        Value::Array(items) => items.iter_mut().for_each(normalise),
        Value::Object(map) => map.values_mut().for_each(normalise),
        _ => {}
    }
}

/// A new song: the smallest document that passes the validator (§4.4).
///
/// A master track because §4.4 requires exactly one; a tempo event at tick 0 and a time
/// signature because the maps may not be empty and need an origin; a render target because its
/// enum may not be left unspecified. Nothing else — a new project is empty, not opinionated.
///
/// Ids and the timestamp come from the caller's sources, never from `SystemTime::now` or an
/// unseeded generator (§11, CLAUDE.md #3), which is what lets two scripted sessions create
/// byte-identical projects. So does `author`: a project a model created should not record a
/// person as having written its first entry, which is what hardcoding `Author::Human` here did
/// while both binaries documented `--author` as covering everything they write.
pub fn new_song(ids: &mut dyn IdSource, clock: &dyn Clock, author: Author) -> Song {
    use escribass_schema::song::*;
    use escribass_schema::SCHEMA_VERSION;

    let made = || Provenance {
        author: author as i32,
        model_id: None,
        prompt_id: None,
        tool_call_id: None,
        created_at: Some(clock.now()),
    };

    let song_id = ids.next_id();
    let master_id = ids.next_id();
    let tempo_id = ids.next_id();
    let signature_id = ids.next_id();

    Song {
        id: song_id,
        provenance: Some(made()),
        version: 1,
        schema_version: SCHEMA_VERSION,
        tempo_map: Some(TempoMap {
            events: [(
                tempo_id.clone(),
                TempoEvent { id: tempo_id, tick: 0, bpm: 120.0 },
            )]
            .into_iter()
            .collect(),
        }),
        time_signature_map: Some(TimeSignatureMap {
            events: [(
                signature_id.clone(),
                TimeSignatureEvent { id: signature_id, tick: 0, numerator: 4, denominator: 4 },
            )]
            .into_iter()
            .collect(),
        }),
        sections: Default::default(),
        markers: Default::default(),
        tracks: [(
            master_id.clone(),
            Track {
                id: master_id,
                provenance: Some(made()),
                version: 1,
                name: "Master".to_string(),
                kind: TrackKind::Master as i32,
                index: 0,
                instrument: None,
                fx_chain: Default::default(),
                routing: Some(Routing::default()),
                mix: Some(Mix { gain_db: 0.0, pan: 0.0, mute: false, solo: false }),
                allow_overlap: false,
            },
        )]
        .into_iter()
        .collect(),
        clips: Default::default(),
        automation: Default::default(),
        generators: Default::default(),
        render_target: Some(RenderTarget {
            kind: RenderKind::Master as i32,
            sample_rate: 48_000,
            bit_depth: 24,
            dither: false,
        }),
    }
}
