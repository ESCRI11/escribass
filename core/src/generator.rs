//! Handing one source to one sandbox process, over gRPC (ADR 0024 §1, §2) — the engine's
//! spawn-and-dial, a **third** child over, and the four places it differs.
//!
//! The whole protocol is [`crate::engine`]'s: start a child from a command `core` was
//! **told**, read **one line** from its stdout — `unix:<path>`, the address of the socket it
//! is now serving `Generate` on, printed only once the server is listening — dial it, make one
//! `Compile` call, and let it go (ADR 0013 §3). That one line is the address and the readiness
//! at once, so there is nothing to poll, no port to guess and no sleep to tune; and the path is
//! the child's to invent, because a parent-chosen one needs entropy in `core` (CLAUDE.md #3)
//! or a pid.
//!
//! **`core` is told a command, not a path, and does not inspect it** (ADR 0024 §1). On a build
//! tree it is `uv run --no-sync --project compilers/generative escribass-generative`; in a
//! bundle it will be something else. Told and never searched, for ADR 0008 §2's reason one
//! child over: a search path that silently finds another interpreter compiles a project
//! against a toolchain nobody chose, and the symptom is a golden that moved with no pull
//! request to blame. Absent, the one call that needs it refuses as an operator error
//! (`generator_missing`) rather than being skipped.
//!
//! **The type is called [`Sandbox`] and the flag is called `--generator`**, and the two names
//! are deliberate. A person compiles a *generator*, which is `escribass_schema::song::Generator`
//! — the entity in the document — so that is what the flag and every rule id say. What this
//! file owns is the *process* that entity is compiled in, and two `Generator`s in one module is
//! how a reader loses the thread.
//!
//! **Where this differs from the engine, and why.**
//!
//! 1. **Its stdin is a pipe this process holds open for the length of the call.** The engine's
//!    is `/dev/null`; this child watches its stdin for end of file and stops serving when it
//!    arrives (ADR 0024 §2, amended — the net for a `core` that died before it called). A
//!    `/dev/null` stdin is *already* at end of file, so a child started that way would stop
//!    before it was dialled. Letting go of the pipe is also what tells a child nobody is
//!    waiting any more, which is why it is dropped on every path out of [`Sandbox::compile`].
//! 2. **`PYTHONHASHSEED=0` is set on it** (ADR 0024 §3's second lock, §4 amended). The child
//!    owns the lock rather than trusting us — one that is started without the variable
//!    re-executes itself with it — so what setting it here buys is not safety but the
//!    interpreter start that re-exec would cost on every compile.
//! 3. **The whole answer is bounded by a wall clock, and running out of it has its own rule
//!    id.** The engine's two clocks decide only whether it failed; this one also decides which
//!    of two things a person is sent to look at. `generator_timeout` is a child that never
//!    answered; `generator_failed` is one that would not start, named no socket, or exited
//!    without answering. **Both are the operator's** — amended 2026-10-01 in M4 PR 6, where
//!    offering the tool to a model is what showed the first of them was not an author's: see
//!    [`NotCompiled`]. One death in that family *is* the author's, and it is told apart by
//!    three facts rather than by one: a child that named its socket, was dialled, and then
//!    died by `SIGKILL` on its own has reached its own hard CPU limit, which no diagnostic can
//!    come back from, and answers `generator_error` with no line (amended 2026-10-02, M4 PR 8).
//! 4. **Nothing about the child survives the call.** A fresh process per compile (ADR 0024
//!    §1), so a compile cannot depend on the compile before it and a limit that kills the
//!    child costs a restart and nothing else. The child is told **nothing about the project**:
//!    no path, no `lock.json`, no `song.json`, not even the clip's id. It is handed what it
//!    compiles (ADR 0026 §1) and hands back notes.
//! 5. **The socket's directory is swept when the child was killed**, because a killed process
//!    runs no `finally` — see [`swept`].
//!
//! Nothing here reads a clock to decide *what* is compiled (CLAUDE.md #3, which from M4 names
//! `compilers`). The request is a pure function of the document, the notes are a function of
//! the request, the socket path is the child's own to invent, and the one clock this file
//! reads decides only which of two answers a caller is given — see [`ANSWERING`].

use crate::engine::{drain, ended, listening, tail};
use crate::project::{asset_hash, ProjectError};
use crate::validate::Violation;
use escribass_proto::generate::compile_response::Result as Answer;
use escribass_proto::generate::generate_client::GenerateClient;
use escribass_proto::generate::{CompileRequest, CompileResponse};
use escribass_schema::song::{clip::Content, Generator, Section, Song, TempoEvent,
    TimeSignatureEvent};
use std::process::{Child, Command, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;
use tonic::transport::Endpoint;

/// How long the sandbox has to name its socket before it is given up on.
///
/// A clock in the failure path alone, as the engine's is: it decides *whether* a compile
/// happened and never a note. A minute rather than a second because what is being waited for
/// is a CPython start under `uv` and the import of `grpclib`, `betterproto2` and `pydantic`.
const NAMING_ITS_SOCKET: Duration = Duration::from_secs(60);

/// How long the sandbox has to answer the one call, before the call ends the caller's turn
/// with `generator_timeout` (ADR 0024 §7, amended twice).
///
/// **This is the imposed failure M0.4's rule asks for**: a hang is not a failure unless one is
/// imposed, and the DSL can express a loop that never ends. The child imposes its own CPU and
/// memory limits and catches the first of them, so a runaway generator comes back
/// as `generator_error` naming the line and the limit it exceeded; this is what is left if
/// answering itself never happens — which is a wedged child and not a source, and is why this
/// is an operator error from M4 PR 6 (see [`NotCompiled`]).
///
/// A minute, so it sits far above everything the child's own limits allow — five CPU seconds
/// soft, five more before the hard one, plus an interpreter start, measured at 6.05 s for the
/// default budget in M4 PR 4. A wall clock below that would turn the child's own diagnostic
/// into this refusal and lose the line number with it.
pub const ANSWERING: Duration = Duration::from_secs(60);

/// `SIGKILL`, the one signal that means something here (see [`Sandbox::compile`]).
///
/// The number rather than a dependency on `libc` for one constant (CLAUDE.md #4). It is 9 on
/// every Linux ABI and this repository claims one platform (ADR 0009 §1).
const SIGKILL: i32 = 9;

/// The same death, reported by a launcher standing in front of the child.
///
/// `core` is told a **command** and never a binary (see the module note), and on a build tree
/// that command is `uv run --no-sync --project compilers/generative escribass-generative`. So
/// the process the kernel kills is `uv`'s child, `uv` sees it die and exits `128 + 9` — the
/// convention every shell and supervisor uses — and `core` reads an exit *code* where a bundle
/// shipping the binary directly would give it a *signal*. One event, two shapes.
const KILLED_BY_THE_KERNEL: i32 = 128 + SIGKILL;

/// Removes the directory the socket was in, once the child that owned it is gone.
///
/// The child removes it itself in a `finally` — and a `finally` does not run when the process
/// is killed, which is three of the paths it has: its own hard CPU limit, [`ANSWERING`]'s kill
/// above, and a test's. So `/tmp/escribass-generative-*` accumulated one directory per killed
/// compile (found by review, 2026-10-02). `core` dialled the path, so `core` can finish the job.
///
/// **`remove_dir`, not `remove_dir_all`**, and that is the whole safety argument: this is a
/// path a *child* named, and an empty-directory removal cannot take anything with it. If the
/// child already tidied up, both calls are no-ops; if it named something that is not a socket
/// in a directory of its own, the second call fails and nothing happens. Both results are
/// ignored, because a compile that failed must not fail differently over a leftover file.
///
/// **Called on the two paths where the child was killed, and on neither of the others.** A
/// child that answered exited by itself and ran its own `finally` — [`ended`] has already
/// waited for it, so there is no race to lose — and one that named no socket named no directory
/// either.
fn swept(address: &str) {
    let Some(socket) = address.strip_prefix("unix:") else { return };
    let socket = std::path::Path::new(socket);
    let _ = std::fs::remove_file(socket);
    if let Some(directory) = socket.parent() {
        let _ = std::fs::remove_dir(directory);
    }
}

/// Where the generative compiler is, as a command this process was told (`--generator`).
///
/// [`Sandbox::answering`] is a field rather than the constant because a test that cannot
/// shrink it cannot watch `generator_timeout` happen at all, and a bound nothing has ever been
/// seen to fire is a bound nobody should believe (docs/plan.md, M4 trap 1).
#[derive(Debug, Clone)]
pub struct Sandbox {
    pub command: Vec<String>,
    pub answering: Duration,
}

/// What the child answered, once it is known to *be* an answer.
///
/// `result` is not an `Option` here, and that is the whole point of the type: proto3 has no
/// required fields, so a `CompileResponse` with no arm set is a well-formed message, and one
/// read as "zero notes" is M1's `RenderResult::decode(&[]) == Ok` arriving one boundary over
/// (`proto/generate.proto`). [`Sandbox::compile`] refuses that shape, so by the time a caller
/// has one of these the child has said something.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub dsl_version: String,
    pub python_version: String,
    pub result: Answer,
}

/// Why a compile produced no answer — and there is only one side of ADR 0006 §2's line left
/// here, which is the amendment of 2026-10-01 (ADR 0024 §7, M4 PR 6).
///
/// Until M4 PR 6 this was an enum: `TimedOut` was caller-fixable, on the reasoning that what
/// loops for ever is the source. Offering `compile_generator` to a model is what showed that
/// wrong. **No source can reach this wall.** The child imposes its own CPU and memory limits,
/// catches the first of them and answers a `Diagnostic` carrying the line — measured at 2.04 s
/// and 6.05 s and 0.28 s in M4 PR 4 — and [`ANSWERING`] is set a minute out *so that* it sits
/// above all of them. A child that has neither answered nor died after that is wedged, and the
/// message says so and carries no line, no column and nothing an author could edit. Feeding
/// that to a model and charging it one of the three refusals a turn allows is M3 trap 2: a
/// retry budget spent on a wall. So it is an operator error like every other child that will
/// not speak, with its own rule id — `generator_timeout`, which is still not
/// `generator_failed`, because "it never answered" and "it died" are different things to go
/// and look at.
pub type NotCompiled = ProjectError;

impl Sandbox {
    pub fn new(command: Vec<String>) -> Self {
        Self { command, answering: ANSWERING }
    }

    /// Compiles one request in a process of its own, and returns what the child answered.
    pub fn compile(&self, request: &CompileRequest) -> Result<Compiled, NotCompiled> {
        let Some((program, arguments)) = self.command.split_first() else {
            return Err(self.broke("generator_failed", "the generator command is empty".into()));
        };
        let mut child = Command::new(program)
            .args(arguments)
            // ADR 0024 §4, amended: the child owns this lock and re-executes itself when it is
            // missing, so what this line buys is the interpreter start that re-exec costs —
            // not the lock. The rest of the environment is inherited, as the engine's is.
            .env("PYTHONHASHSEED", "0")
            // **A pipe of our own, and held**: see the module note. `/dev/null` is already at
            // end of file, and this child stops serving when its stdin reaches one.
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            // Not `generator_missing`: that one is the *flag* being absent, which is a
            // different thing a person does about differently (ADR 0024 §7).
            .map_err(|e| {
                self.broke("generator_failed", format!("cannot start the generator: {e}"))
            })?;

        // Dropped on every path below, which is what tells the child nobody is waiting.
        let stdin = child.stdin.take();
        // Drained on a thread of its own, for the engine's reason: a full stderr pipe stops
        // the child mid-call, and the call is what this process is blocked on.
        let said = child.stderr.take().map(drain);

        let address = match listening(&mut child, NAMING_ITS_SOCKET, "generator") {
            Ok(address) => address,
            Err(why) => {
                drop(stdin);
                return Err(self.verdict(&mut child, said, &why));
            }
        };

        let answered = compile_one(&address, request, self.answering);
        // Before the waiting below, never after: a child that is still waiting for its call
        // leaves when this is dropped, and one that has answered has already stopped.
        drop(stdin);

        match answered {
            Err(Call::TimedOut) => {
                // Killed rather than given the moment [`ended`] gives a child that is on its
                // way out: this one is not on its way anywhere, which is the whole report.
                let _ = child.kill();
                let _ = child.wait();
                drop(tail(said));
                swept(&address);
                Err(self.broke(
                    "generator_timeout",
                    format!(
                        "the generator did not answer within {} seconds and was stopped. \
                         Nothing an author can write reaches this bound: the child's own CPU \
                         and memory limits fire inside it and come back as `generator_error` \
                         with the line, so a compiler that is silent for this long is wedged \
                         (ADR 0024 §7, amended 2026-10-01)",
                        self.answering.as_secs()
                    ),
                ))
            }
            Err(Call::Failed) => {
                // The transport's view of the failure is not read, for the engine's reason:
                // the useful report is the one the child already wrote, which is its exit
                // status and the last of its stderr (ADR 0013 §3).
                let gone = ended(&mut child, said, "the generator did not answer the call");
                swept(&address);
                // **The one failure in this family that is the author's.** A child that named
                // its socket, was dialled, and then died by `SIGKILL` of its own accord has
                // reached its own hard CPU limit: the soft one is a Python signal handler and
                // a built-in call does not return to the interpreter for it to run, so
                // `x = sum(range(10**10))` under a one-second budget produces no diagnostic
                // and no answer — measured at exit −9 after 6.9 s (ADR 0024 §7, amended
                // 2026-10-02, where review found it reported as `generator_failed`).
                //
                // Told apart by all three facts and not by the signal alone: the child got as
                // far as naming a socket and being called, and both fields are `None` for the
                // `SIGKILL` [`ended`] itself sends to a child that would not leave.
                if gone.by_signal == Some(SIGKILL)
                    || gone.exit_code == Some(KILLED_BY_THE_KERNEL)
                {
                    return Err(self.broke("generator_error", format!(
                        "the generator exceeded the compiler's CPU limit inside a single \
                         built-in call and was stopped by the kernel, so it never regained \
                         control to say where: there is no line. A call that does not return \
                         to the interpreter — `sum(range(10**10))`, a `Fraction` of a \
                         thousand-digit power — cannot be interrupted by the limit's own \
                         handler, and the hard limit a few seconds above it is what stops it. \
                         Do less work per call (ADR 0024 §4, §7); {}",
                        gone.message,
                    )));
                }
                Err(self.broke("generator_failed", gone.message))
            }
            Ok(answer) => {
                // **Reaped, not judged.** The answer is complete — a child killed mid-send
                // would have failed the call above — and ADR 0024 §2 says `core` may let go of
                // the child without waiting, so its exit status is the verdict on nothing.
                // What this is for is that a host which compiles all afternoon accumulates
                // neither zombies nor servers on abandoned sockets, and `ended` is the bounded
                // `wait` that does it: a moment, then a kill.
                let gone = ended(&mut child, said, "the generator answered");
                let Some(result) = answer.result else {
                    return Err(self.broke(
                        "generator_failed",
                        format!(
                            "the generator answered with a CompileResponse carrying neither \
                             notes nor a diagnostic; a generator that emits nothing answers \
                             with an empty note list, so no arm at all is a child that \
                             returned without compiling (proto/generate.proto); {}",
                            gone.message
                        ),
                    ));
                };
                if answer.dsl_version.is_empty() {
                    return Err(self.broke(
                        "generator_failed",
                        format!(
                            "the generator answered without stating its dsl_version, which it \
                             states on every answer and `core` compares before anything is \
                             written (ADR 0027 §1, §2); {}",
                            gone.message
                        ),
                    ));
                }
                Ok(Compiled {
                    dsl_version: answer.dsl_version,
                    python_version: answer.python_version,
                    result,
                })
            }
        }
    }

    /// What became of a child that did not answer, as the caller's error.
    ///
    /// One rule id for both of [`ended`]'s outcomes, where the engine and the sidecar have
    /// two. ADR 0024 §7 names one — `generator_failed` — and the distinction the second id
    /// buys elsewhere is already bought here by the return type: there is no path on which a
    /// child that said nothing becomes an `Ok`, which was the defect the engine's
    /// `engine_unreadable` was split out to make visible (M1 PR 13). Which of the two it was
    /// is in the message, where a person reads it.
    fn verdict(
        &self,
        child: &mut Child,
        said: Option<JoinHandle<Vec<u8>>>,
        what: &str,
    ) -> NotCompiled {
        self.broke("generator_failed", ended(child, said, what).message)
    }

    fn broke(&self, rule: &'static str, message: String) -> NotCompiled {
        ProjectError { path: self.command.join(" "), rule, message }
    }
}

/// How the one call ended, when it did not end with an answer.
enum Call {
    /// The wall clock, [`Sandbox::answering`].
    TimedOut,
    /// Anything else, and deliberately carrying nothing: what the transport saw is never the
    /// verdict, so there is nothing here to be tempted into reporting. [`Sandbox::verdict`]
    /// asks the child what became of it instead (ADR 0013 §3).
    Failed,
}

/// One `Compile` call, on a runtime and a thread of its own.
///
/// Both are required rather than tidy, for `crate::engine`'s reason: this is a synchronous
/// tool like any other, and when the tool API is reached over gRPC it is already running on
/// that server's runtime, where `block_on` panics.
///
/// Neither message is capped. The request carries the song's whole tempo map, signature map
/// and section list, and the answer carries every note a generator wrote — a four-minute
/// generator at a sixteenth-note grid is thousands — so a limit here would be a refusal the
/// transport invented rather than one the validator states (`crate::engine`, same two lines).
///
/// `ponytail:` a channel per call, built and dropped around it. There is one call, to a server
/// that exits after serving it, so there is nothing a pool could hold on to.
fn compile_one(
    address: &str,
    request: &CompileRequest,
    within: Duration,
) -> Result<CompileResponse, Call> {
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| Call::Failed)?;
                runtime.block_on(async {
                    let channel = Endpoint::from_shared(address.to_string())
                        .map_err(|_| Call::Failed)?
                        .connect()
                        .await
                        .map_err(|_| Call::Failed)?;
                    let mut client = GenerateClient::new(channel)
                        .max_encoding_message_size(usize::MAX)
                        .max_decoding_message_size(usize::MAX);
                    match tokio::time::timeout(within, client.compile(request.clone())).await {
                        Err(_) => Err(Call::TimedOut),
                        Ok(Ok(answer)) => Ok(answer.into_inner()),
                        Ok(Err(_)) => Err(Call::Failed),
                    }
                })
            })
            .join()
            .expect("the thread that calls the generator")
    })
}

// ---------------------------------------------------------------------------
// What crosses, and what names it (ADR 0024 §6, ADR 0026 §1)
// ---------------------------------------------------------------------------

/// The request one generator compiles from, and the clip its notes replace.
///
/// **Pure, and the whole of what a compile reads.** Everything the sandbox is handed is here
/// and nothing a compile reads is elsewhere, because this message is also the *definition* of
/// `Generator.compiled_hash` (ADR 0024 §6): a field in here that a compile does not read
/// would stale a generator on an event that cannot change a note, and a fact a compile reads
/// that is not in here would leave one fresh after its notes have changed.
///
/// Refuses `target_not_note_clip` — valid and uncompilable, the shape ADR 0007 §6 gave *valid
/// and unrenderable* — for a generator whose target is a track or an audio clip. The validator
/// accepts either target because the schema allows either (ADR 0002 §3); what a track target
/// should *mean* is not decided against no consumer, and is a ledger row until something asks
/// (ADR 0024 §5).
pub fn to_compile(song: &Song, generator: &Generator) -> Result<(String, CompileRequest), Violation> {
    let at = format!("/generators/{}/target", generator.id);
    let not_a_note_clip = |what: String| Violation {
        path: at.clone(),
        rule: "target_not_note_clip",
        message: format!(
            "{what}; in M4 a compile writes a note clip's notes, and what a track target \
             should mean is not decided against no consumer (ADR 0024 §5)"
        ),
    };
    let clip_id = match &generator.target {
        Some(escribass_schema::song::generator::Target::ClipId(id)) => id.clone(),
        Some(escribass_schema::song::generator::Target::TrackId(id)) => {
            return Err(not_a_note_clip(format!("`{id}` is a track, not a note clip")))
        }
        None => return Err(not_a_note_clip("this generator targets nothing".to_string())),
    };
    // The validator has already refused a target naming nothing (`clip_unknown`), so a clip
    // that is not here is a document no commit produced — reported rather than unwrapped.
    let Some(clip) = song.clips.get(&clip_id) else {
        return Err(not_a_note_clip(format!("`{clip_id}` is not a clip in this song")));
    };
    if !matches!(clip.content, Some(Content::NoteClip(_))) {
        return Err(not_a_note_clip(format!("`{clip_id}` is an audio clip; it has no notes")));
    }

    // Tick order, as a plan's repeated fields are (ADR 0007 §2). Both maps iterate in id
    // order, and the sort is stable, so a tie — which a valid song has none of, since
    // `set_tempo` upserts by tick — breaks the way `core/src/render.rs` already breaks it.
    // Sections break theirs by name, which is the one of the three that has one.
    let mut tempo: Vec<&TempoEvent> =
        song.tempo_map.iter().flat_map(|map| map.events.values()).collect();
    tempo.sort_by_key(|e| e.tick);
    let mut signature: Vec<&TimeSignatureEvent> =
        song.time_signature_map.iter().flat_map(|map| map.events.values()).collect();
    signature.sort_by_key(|e| e.tick);
    let mut sections: Vec<&Section> = song.sections.values().collect();
    sections.sort_by(|a, b| (a.start_tick, &a.name).cmp(&(b.start_tick, &b.name)));

    Ok((
        clip_id,
        CompileRequest {
            kind: generator.kind,
            source: generator.source.clone(),
            seed: generator.seed,
            params: generator.params.clone(),
            // §4.3 blanked, as a plan blanks it: an id the child cannot use would be an input
            // to the hash that no compile reads, so renaming a tempo event's key would stale
            // every generator in the song.
            tempo: tempo
                .into_iter()
                .map(|e| TempoEvent { id: String::new(), ..e.clone() })
                .collect(),
            signature: signature
                .into_iter()
                .map(|e| TimeSignatureEvent { id: String::new(), ..e.clone() })
                .collect(),
            sections: sections
                .into_iter()
                .map(|s| Section {
                    id: String::new(),
                    provenance: None,
                    version: 0,
                    ..s.clone()
                })
                .collect(),
            clip_start_tick: clip.start_tick,
            clip_length_ticks: clip.length_ticks,
        },
    ))
}

/// What `Generator.compiled_hash` is: the SHA-256 of the canonical JSON of the request the
/// compile was made from, and of nothing else (ADR 0024 §6).
///
/// **One hasher, in `core`.** The same `sha2` that names an asset in `assets/` and a prompt in
/// a provenance, because a hash computed in Python and compared against one computed in Rust
/// disagree the first time one canonicalises a key order differently — M0.1's serialisation
/// boundary with a status word as the symptom (docs/plan.md, M4 trap 3). The sandbox is never
/// asked what it compiled; `core` knows, because `core` built the question.
///
/// Written by the compile, recomputed by whoever asks: **unequal is stale, empty is never
/// compiled.** `toolchain_version` is deliberately not in the request and so not in here — a
/// toolchain is a pin, compared at compile, and a hash that went stale on the same event would
/// say the same thing twice (ADR 0027 §2).
pub fn compiled_hash(request: &CompileRequest) -> String {
    let text = crate::to_canonical_json(request)
        // A `CompileRequest` holds one double, `TempoEvent.bpm`, and the validator refuses a
        // non-finite one before a song can hold it — so there is no document this can fail
        // on. An empty hash would read as "never compiled", which is the one answer that must
        // not be reachable by accident, so this does not swallow it.
        .expect("a request built from a validated song is representable");
    asset_hash(text.as_bytes())
}
