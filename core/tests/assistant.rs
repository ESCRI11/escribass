//! Starting the sidecar, talking to it, and reading what became of it (ADR 0020 §4, §5).
//!
//! The sidecar this suite drives is a **shell script beside an `Assistant` server in this
//! process**, not `escribass-ai` — `core/tests/engine.rs`'s choice, for the same two reasons.
//! What is tested here is `core`'s side of the boundary: the line it reads, the prompt it
//! sends, the events it collects, which failures are an operator's, and whether a dead sidecar
//! is reported with its status. And the split is what makes the unhappy paths reachable at all:
//! the script is the process `core` spawns and waits on, so an exit code, a silent stdout, an
//! address nothing is listening on and a child that dies mid-session are each one line of
//! shell, where the real sidecar would have to be broken to produce them.
//!
//! What the **real** sidecar does is `ai/tests/test_sidecar.py`, which drives the real process
//! over a real socket — and the two halves meet in
//! [`the_real_sidecar_answers_the_transcript`], which is `#[ignore]`d because it needs `ai/`'s
//! Python environment, the way `tests/renders.rs`'s device test needs an audio output. It is
//! run by hand, and what it prints is in the pull request.

mod common;
use common::{fake_assistant, manifest};

use escribass_core::{
    Assistant, FixedClock, Health, Project, SeededIds, Session, TurnEnd, REFUSALS_PER_TURN,
};
use escribass_proto::assistant::{
    assistant_command, assistant_event, AssistantEvent, Done, ReplyText, ToolCall,
};
use escribass_schema::song::Author;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const AT: i64 = 1_788_307_200_000;
/// The track `add_track` mints first in a fresh session over the fixture, under `SeededIds`
/// default: `Project::create` takes counter 1 for the root entry, so the first id a tool mints
/// is counter 2. Named literally for `tests/AGENTS.md`'s reason — a change in mint order should
/// fail loudly rather than quietly still passing.
const MINTED: &str = "01M1FPMP000000000000000002";

/// A project on disk and a session over it, as the window has one.
fn opened(dir: &Scratch) -> Session {
    let song = serde_json::from_str(
        &std::fs::read_to_string(FIXTURE).expect("the song fixture is readable"),
    )
    .expect("the fixture is a song");
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project =
        Project::create(dir.at("project"), &song, &mut ids, &clock, Author::Human, manifest())
            .expect("the project is created");
    Session::new(project, Box::new(ids), Box::new(clock), Author::Human)
}

fn call_event(id: &str, name: &str, args: &str) -> AssistantEvent {
    AssistantEvent {
        event: Some(assistant_event::Event::Call(ToolCall {
            call_id: id.to_string(),
            name: name.to_string(),
            args_json: args.to_string(),
            model_id: "deepseek/deepseek-v4.1-flash".to_string(),
        })),
    }
}

fn done_event(text: &str) -> AssistantEvent {
    AssistantEvent {
        event: Some(assistant_event::Event::Done(Done {
            text: text.to_string(),
            model_id: "deepseek/deepseek-v4.1-flash".to_string(),
        })),
    }
}

/// The `CallResult`s the host sent back, once the sidecar has seen `count` of them.
///
/// Bounded, and it decides only whether this test can proceed: the host returned from the turn
/// the moment it read `done`, and what it sent afterwards is still in flight.
fn answered(served: &common::AskedFor, count: usize) -> Vec<escribass_proto::tools::ToolResult> {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let results: Vec<escribass_proto::tools::ToolResult> = served
            .commands()
            .into_iter()
            .filter_map(|command| match command.command {
                Some(assistant_command::Command::Result(answered)) => answered.result,
                _ => None,
            })
            .collect();
        if results.len() >= count {
            return results;
        }
        assert!(Instant::now() < deadline, "only {} of {count} results arrived", results.len());
        std::thread::sleep(Duration::from_millis(10));
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-assistant-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self(path)
    }

    fn at(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A sidecar that is a script. `common::fake_engine` writes it: a child that prints an address
/// on its stdout is a child that prints an address, whichever service it then pretends to
/// serve, and one script is one thing to keep in step with what `core` actually spawns.
#[cfg(unix)]
fn fake_sidecar(dir: &Scratch, body: &str) -> PathBuf {
    common::fake_engine(&dir.0, body)
}

#[cfg(unix)]
fn told(script: &std::path::Path, arguments: &[&str]) -> Assistant {
    let mut command = vec![script.display().to_string()];
    command.extend(arguments.iter().map(|a| a.to_string()));
    Assistant::new(command)
}

// ---- the happy path, and what crosses (ADR 0020 §3, §4) ----

#[cfg(unix)]
#[test]
fn it_reads_the_one_line_and_answers_on_the_socket_the_sidecar_named() {
    // The whole protocol in one claim: the command is run as it was given, the first line of
    // its stdout is the address, one `Prompt` crosses there, and the events come back in order.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(
        &dir.0,
        vec![
            AssistantEvent {
                event: Some(assistant_event::Event::Text(ReplyText {
                    text: "raising it".to_string(),
                })),
            },
            done_event("raising it"),
        ],
        None,
    );
    // The script stays alive after naming its socket, because a sidecar serves a session
    // rather than a call: `cat` returns when `core` closes the stdin it holds.
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar =
        told(&script, &["--transcript", "one-answer.json"]).start().expect("it starts");

    assert_eq!(sidecar.address(), format!("unix:{}", served.socket.display()));
    assert_eq!(sidecar.health(), Health::Running);
    // The arguments are the command's, passed through untouched: `core` is told a command and
    // does not inspect it (ADR 0020 §4).
    assert_eq!(
        std::fs::read_to_string(dir.at("argument")).expect("the script recorded its argument"),
        "--transcript\n"
    );

    let turn = sidecar
        .turn(&mut session, "raise the bass in bar 17", &[])
        .expect("the turn answers");

    assert_eq!(turn.end, TurnEnd::Answered);
    assert_eq!(turn.recorded.reply, "raising it");
    assert_eq!(turn.model_id, "deepseek/deepseek-v4.1-flash");
    let asked = served.commands();
    assert_eq!(asked.len(), 1, "one prompt per turn: {asked:?}");
    // The stream carries the prompt the caller wrote, and no key, no project path and no
    // session travel with it (ADR 0020 §4).
    let Some(assistant_command::Command::Prompt(prompt)) = asked[0].command.clone() else {
        panic!("the first message on the stream is the prompt: {asked:?}");
    };
    assert_eq!(prompt.text, "raise the bass in bar 17");
    // The document the view is computed from, and the twelve tools the model is offered, with
    // `dry_run` taken out of every one (ADR 0018 §1, ADR 0022 §1).
    assert_eq!(prompt.song.as_ref().map(|song| song.id.as_str()), Some("01M1FPMP00SNG0000000000001"));
    assert_eq!(prompt.tools.len(), escribass_core::OFFERED.len());
    assert!(
        prompt.tools.iter().all(|tool| !tool.input_schema.contains("dry_run")),
        "a schema still advertises `dry_run`"
    );
    // The model the project records, which this prompt is what writes (ADR 0021 §4).
    assert_eq!(prompt.model_id, escribass_core::DEFAULT_MODEL);
    assert_eq!(sidecar.health(), Health::Running, "a turn does not end the sidecar");
    // And nothing was applied: a proposal is pending and the log is where it was (ADR 0019 §2).
    assert!(session.proposal().is_some(), "the turn left no proposal");
    assert_eq!(session.project().history().entries().len(), 1);
}

// ---- a sidecar that will not start is an operator's (ADR 0006 §2) ----

#[cfg(unix)]
#[test]
fn a_command_that_is_not_there_is_an_operator_error() {
    let dir = Scratch::new();
    let failed = told(&dir.at("no-such-sidecar"), &[]).start().expect_err("nothing to start");
    assert_eq!(failed.rule, "assistant_missing", "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn a_sidecar_that_will_not_start_is_reported_with_its_exit_status_and_its_last_words() {
    // ADR 0020 §5: the verdict is the child's exit status, read — and the sentence a person
    // acts on is the one the child wrote on its way out.
    let dir = Scratch::new();
    let script = fake_sidecar(&dir, "echo 'no transcript at that path' >&2\nexit 2");
    let failed = told(&script, &[]).start().expect_err("it did not start");
    assert_eq!(failed.rule, "assistant_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 2"), "{}", failed.message);
    assert!(failed.message.contains("no transcript at that path"), "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn a_sidecar_that_exits_nought_without_naming_a_socket_is_unreadable() {
    // The engine's own M1 PR 13 defect, one process over: a child that exited 0 having done
    // nothing is not a crash to look up in a log, and calling it `assistant_failed` would send
    // a person looking for one.
    let dir = Scratch::new();
    let script = fake_sidecar(&dir, "exit 0");
    let failed = told(&script, &[]).start().expect_err("it named no socket");
    assert_eq!(failed.rule, "assistant_unreadable", "{}", failed.message);
}

// ---- the health dot (ADR 0020 §5) ----

#[cfg(unix)]
#[test]
fn the_health_dot_is_the_childs_exit_status_read_and_kept() {
    // The claim ADR 0020 §5 makes in place of `Preview::drop`'s gap: a sidecar that dies is
    // reported *with its status*, and asking twice says the same thing — the window asks on a
    // timer, and the tail of a pipe can only be taken once.
    let dir = Scratch::new();
    let leave = dir.at("leave");
    let script = fake_sidecar(
        &dir,
        &format!(
            "echo unix:{}\necho 'the provider is unreachable' >&2\n\
             while [ ! -f '{}' ]; do sleep 0.02; done\nexit 5",
            dir.at("nothing-listens-here.sock").display(),
            leave.display(),
        ),
    );
    let mut sidecar = told(&script, &[]).start().expect("it starts");
    assert_eq!(sidecar.health(), Health::Running);

    std::fs::write(&leave, b"").expect("the scratch directory is writable");
    let gone = wait_until_gone(&mut sidecar);
    assert!(gone.contains("exited 5"), "{gone}");
    assert!(gone.contains("the provider is unreachable"), "{gone}");
    assert_eq!(sidecar.health(), Health::Gone(gone), "the status is read once and kept");
}

#[cfg(unix)]
#[test]
fn a_turn_that_fails_leaves_a_living_sidecar_alone() {
    // Where this differs from the engine, and why it has its own test: an engine that fails a
    // call is on its way out and is killed; a sidecar serves a session, so one bad turn must
    // not end it. The address below is a path nothing listens on.
    let dir = Scratch::new();
    let script = fake_sidecar(
        &dir,
        &format!("echo unix:{}\ncat > /dev/null", dir.at("nobody.sock").display()),
    );
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let mut session = opened(&dir);
    let failed =
        sidecar.turn(&mut session, "anything", &[]).expect_err("nothing is listening");
    assert_eq!(failed.rule, "assistant_failed", "{}", failed.message);
    assert_eq!(sidecar.health(), Health::Running, "a failed turn killed the sidecar");
    // The fork goes with it: a half-built proposal nobody saw is not a thing to leave pending
    // (ADR 0022 §3).
    assert!(session.proposal().is_none(), "a failed turn left a proposal behind");
}

#[cfg(unix)]
#[test]
fn stopping_it_closes_its_stdin_and_says_how_it_went() {
    // How a session ends: the host lets go of the pipe, the sidecar stops serving and exits,
    // and the status is read on that path too rather than discarded (ADR 0020 §5).
    let dir = Scratch::new();
    let script = fake_sidecar(
        &dir,
        &format!(
            "echo unix:{}\ncat > /dev/null\nexit 0",
            dir.at("assistant.sock").display()
        ),
    );
    let sidecar = told(&script, &[]).start().expect("it starts");
    let said = sidecar.stop();
    assert!(said.contains("exited 0"), "{said}");
}

// ---- the loop (ADR 0019 §1, ADR 0022 §3) ----

#[cfg(unix)]
#[test]
fn the_models_calls_land_on_the_proposal_and_nothing_is_applied() {
    // The turn in one claim: two calls go out, two results come back carrying the document as
    // the proposal has it, and the project is untouched until a person applies (ADR 0019 §1).
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(
        &dir.0,
        vec![
            call_event(
                "call_1",
                "add_track",
                r#"{"name": "Lead", "kind": 1,
                    "ref": {"plugin": {"plugin_id": "Surge Synth Team/Surge XT",
                                       "version": "1.3.4"}}}"#,
            ),
            call_event(
                "call_2",
                "add_clip",
                &format!(
                    r#"{{"track_id": "{MINTED}", "start_tick": 0, "length_ticks": 960,
                        "note_clip": {{"notes": {{}}}}}}"#
                ),
            ),
            done_event("added a Lead track with an empty clip"),
        ],
        None,
    );
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let turn = sidecar.turn(&mut session, "add a lead", &[]).expect("the turn answers");

    assert_eq!(turn.end, TurnEnd::Answered);
    assert_eq!(turn.recorded.calls.len(), 2, "{:?}", turn.recorded.calls);
    assert!(
        turn.recorded.calls.iter().all(|c| c.result.as_ref().is_some_and(|r| r.valid)),
        "a call was refused: {:?}",
        turn.recorded.calls
    );
    // **The second call named the first call's track and was accepted.** That is the whole of
    // what a fork buys over a dry run: under a dry run the track would have been minted into a
    // document that was thrown away, and this call would be `track_unknown` (M3 trap 8).
    let proposal = session.proposal().expect("the turn left a proposal");
    assert!(proposal.song().tracks.contains_key(MINTED));
    assert_eq!(proposal.song().clips.values().filter(|c| c.track_id == MINTED).count(), 1);
    assert_eq!(proposal.calls(), ["add_track", "add_clip"]);

    // Nothing is applied. The project has its root entry and nothing else, and its document is
    // the one it opened with (ADR 0019 §2; the PR row's "what it does not do").
    assert_eq!(session.project().history().entries().len(), 1);
    assert!(!session.project().song().tracks.contains_key(MINTED));

    // And the sidecar was answered with the proposal's document, not the project's, so the
    // view the model reads between calls is the one its own calls produced (ADR 0018 §2).
    let results = answered(&served, 2);
    assert!(results.iter().all(|r| r.valid), "{results:?}");
    let songs: Vec<bool> = served
        .commands()
        .into_iter()
        .filter_map(|command| match command.command {
            Some(assistant_command::Command::Result(answered)) => {
                Some(answered.song.is_some_and(|song| song.tracks.contains_key(MINTED)))
            }
            _ => None,
        })
        .collect();
    assert_eq!(songs, [true, true], "the proposal's document did not cross back");
}

#[cfg(unix)]
#[test]
fn three_refused_calls_end_the_turn_and_the_fourth_is_never_made() {
    // ADR 0022 §3's first kind, and its budget. A refusal goes back to the model **whole** —
    // every violation, because a model fixing one problem at a time wastes its retries — and
    // three of them in one turn end it. Counted per turn rather than per call, because a loop
    // cannot tell a retry of one call from a new call.
    //
    // Watched failing first: with the budget spelled `> REFUSALS_PER_TURN` this test found the
    // fourth call executed and four results fed back.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let refused = || call_event("c", "add_clip", r#"{"track_id": "01M1FPMP00NOSUCHTRACK00000"}"#);
    let served = fake_assistant(
        &dir.0,
        vec![refused(), refused(), refused(), refused(), done_event("gave up")],
        None,
    );
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let turn = sidecar.turn(&mut session, "put a clip on a track I made up", &[]).expect("a turn");

    assert_eq!(turn.end, TurnEnd::Refused);
    assert_eq!(turn.recorded.calls.len(), REFUSALS_PER_TURN);
    // Whole: the rules the validator gave, not a count and not the first one.
    let rules: Vec<&str> = turn.recorded.calls[0]
        .result
        .as_ref()
        .expect("a refusal is a result")
        .errors
        .iter()
        .map(|e| e.rule.as_str())
        .collect();
    assert!(rules.contains(&"track_unknown"), "{rules:?}");
    // Fed back, and provably: the sidecar sends its next call only once the previous call's
    // result has arrived, so the third refusal exists because the first two were fed back.
    // The last one is not asserted — the host closed the stream the moment the budget ran out,
    // and what is still in flight at a close is not a claim worth making.
    let results = answered(&served, REFUSALS_PER_TURN - 1);
    assert!(results.iter().all(|r| !r.valid), "{results:?}");

    // The proposal stays: a person is told which calls were refused and why, and whatever the
    // accepted calls put in it — here, nothing — is still theirs to reject (ADR 0022 §3).
    assert!(session.proposal().is_some());
    assert_eq!(session.project().history().entries().len(), 1);
}

#[cfg(unix)]
#[test]
fn a_provider_failure_is_neither_a_refusal_nor_the_projects() {
    // ADR 0022 §3's third kind, arriving as `ai` reports it after its own bounded retries:
    // `UNAVAILABLE` on the closed stream. The host must not show it as a refusal — the model's
    // call was not wrong — and must not show it as a corrupt project (M3 trap 2).
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(
        &dir.0,
        vec![],
        Some(tonic::Status::unavailable(
            "the provider failed after 3 retries: 429 temporarily rate-limited upstream",
        )),
    );
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let failed = sidecar.turn(&mut session, "anything", &[]).expect_err("the provider failed");
    assert_eq!(failed.rule, "provider_failed", "{}", failed.message);
    assert!(failed.message.contains("rate-limited"), "{}", failed.message);
    // The sidecar is alive and was not blamed for it, and the turn left nothing pending.
    assert_eq!(sidecar.health(), Health::Running);
    assert!(session.proposal().is_none());
}

#[cfg(unix)]
#[test]
fn a_model_that_ran_out_of_room_is_not_a_provider_failure() {
    // The fourth thing ADR 0022 §3 names so it is not mistaken for one of the three: the
    // spike's `finish_reason: length` after 8,192 tokens of reasoning is the model exhausting
    // its output, and a retry with backoff reproduces it.
    //
    // Watched failing first: mapping `ResourceExhausted` to `provider_failed` fails this test,
    // which is what keeps the fourth thing from collapsing into the third.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(
        &dir.0,
        vec![],
        Some(tonic::Status::resource_exhausted("the model produced neither a call nor text")),
    );
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let failed = sidecar.turn(&mut session, "anything", &[]).expect_err("the model gave up");
    assert_eq!(failed.rule, "turn_unfinished", "{}", failed.message);
}

#[cfg(unix)]
#[test]
fn a_project_that_will_not_write_ends_the_turn_before_the_model_sees_it() {
    // ADR 0022 §3's second kind, with **zero** retries: the project refuses, the prompt never
    // goes out, and nothing is fed back to a model that has not been asked anything.
    //
    // The failure is real rather than injected: the first prompt in a project records the model
    // it was sent to in `lock.json` (ADR 0021 §4), and a directory a person cannot write is a
    // project that will not write.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(&dir.0, vec![done_event("never asked")], None);
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    let project = dir.at("project");
    let permissions = std::fs::metadata(&project).expect("the project is there").permissions();
    let mut locked = permissions.clone();
    std::os::unix::fs::PermissionsExt::set_mode(&mut locked, 0o555);
    std::fs::set_permissions(&project, locked).expect("the mode is settable");

    let failed = sidecar.turn(&mut session, "raise the bass", &[]).expect_err("it cannot write");
    std::fs::set_permissions(&project, permissions).expect("put it back for the cleanup");

    assert_eq!(failed.rule, "unwritable", "{}", failed.message);
    // Zero retries and nothing asked: the model never saw it (ADR 0022 §3).
    assert!(served.commands().is_empty(), "the prompt went out anyway: {:?}", served.commands());
    assert!(session.proposal().is_none(), "a failed turn left a proposal behind");
}

#[cfg(unix)]
#[test]
fn a_second_prompt_waits_for_the_pending_proposal() {
    // ADR 0019 §3: one proposal per session at a time. A new prompt is refused rather than
    // stacking a second fork on the first or dropping what a person has not read.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let served = fake_assistant(&dir.0, vec![done_event("done")], None);
    let script = fake_sidecar(&dir, &format!("{}\ncat > /dev/null", served.address()));
    let mut sidecar = told(&script, &[]).start().expect("it starts");

    sidecar.turn(&mut session, "first", &[]).expect("the first turn answers");
    let refused = sidecar.turn(&mut session, "second", &[]).expect_err("one at a time");
    assert_eq!(refused.rule, "proposal_pending", "{}", refused.message);

    // Rejecting it is what unblocks the next prompt, and nothing is undone because nothing was
    // written (ADR 0019 §3).
    assert!(session.reject());
    sidecar.turn(&mut session, "second", &[]).expect("the next turn answers");
}

// ---- the two halves, meeting (ADR 0020 §4) ----

/// The real sidecar, started the way ADR 0020 §4 says `core` starts it.
///
/// **`#[ignore]`d**, for `tests/renders.rs`'s reason one directory over: it needs something the
/// machine may not have — here `uv` and a synced `ai/` environment — and a test that noticed
/// and returned would be the quiet skip the determinism suite exists to prevent. Run it with
///
/// ```text
/// cd ai && uv sync --locked
/// cargo test -p escribass-core --test assistant -- --ignored --nocapture
/// ```
#[cfg(unix)]
#[test]
#[ignore = "needs uv and ai/'s environment: cd ai && uv sync --locked"]
fn the_real_sidecar_answers_the_transcript() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let ai = root.join("ai");
    let transcript = ai.join("tests/transcripts/one-answer.json");
    let mut sidecar = Assistant::new(vec![
        "uv".to_string(),
        "run".to_string(),
        "--project".to_string(),
        ai.display().to_string(),
        "escribass-ai".to_string(),
        "--transcript".to_string(),
        transcript.display().to_string(),
    ])
    .start()
    .expect("the real sidecar starts");

    println!("the sidecar named {}", sidecar.address());
    assert!(sidecar.address().starts_with("unix:"));

    let dir = Scratch::new();
    let mut session = opened(&dir);
    let turn = sidecar
        .turn(&mut session, "What can you change about this song?", &[])
        .expect("the real sidecar answers");
    println!("{:#?}", turn.recorded);

    let recorded = std::fs::read_to_string(&transcript).expect("the transcript is readable");
    assert!(
        recorded.contains(&turn.recorded.reply),
        "the answer is not the transcript's: {}",
        turn.recorded.reply
    );
    assert_eq!(turn.end, TurnEnd::Answered);
    assert!(turn.recorded.calls.is_empty(), "this transcript proposes nothing");
    assert_eq!(session.project().history().entries().len(), 1, "nothing was applied");

    println!("{}", sidecar.stop());
}

#[cfg(unix)]
fn wait_until_gone(sidecar: &mut escribass_core::Sidecar) -> String {
    // Bounded, and it decides only whether this test can proceed: the child is leaving on a
    // file the test has already written.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Health::Gone(said) = sidecar.health() {
            return said;
        }
        assert!(Instant::now() < deadline, "the sidecar never left");
        std::thread::sleep(Duration::from_millis(10));
    }
}
