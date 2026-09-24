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
use common::fake_assistant;

use escribass_core::{Assistant, Health};
use escribass_proto::assistant::{assistant_event, AssistantEvent, Done, Prompt, ReplyText};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

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

fn text(event: &AssistantEvent) -> Option<&str> {
    match event.event.as_ref()? {
        assistant_event::Event::Text(said) => Some(&said.text),
        _ => None,
    }
}

// ---- the happy path, and what crosses (ADR 0020 §3, §4) ----

#[cfg(unix)]
#[test]
fn it_reads_the_one_line_and_answers_on_the_socket_the_sidecar_named() {
    // The whole protocol in one claim: the command is run as it was given, the first line of
    // its stdout is the address, one `Prompt` crosses there, and the events come back in order.
    let dir = Scratch::new();
    let served = fake_assistant(
        &dir.0,
        vec![
            AssistantEvent {
                event: Some(assistant_event::Event::Text(ReplyText {
                    text: "raising it".to_string(),
                })),
            },
            AssistantEvent {
                event: Some(assistant_event::Event::Done(Done {
                    text: "raising it".to_string(),
                    model_id: "deepseek/deepseek-v4.1-flash".to_string(),
                })),
            },
        ],
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

    let events = sidecar
        .answer(Prompt { text: "raise the bass in bar 17".to_string(), ..Prompt::default() })
        .expect("the turn answers");

    assert_eq!(events.len(), 2, "{events:?}");
    assert_eq!(text(&events[0]), Some("raising it"));
    let asked = served.commands();
    assert_eq!(asked.len(), 1, "one prompt per turn: {asked:?}");
    // The stream carries the prompt the caller wrote, and no key, no project path and no
    // session travel with it (ADR 0020 §4).
    assert!(
        format!("{asked:?}").contains("raise the bass in bar 17"),
        "the prompt did not cross: {asked:?}"
    );
    assert_eq!(sidecar.health(), Health::Running, "a turn does not end the sidecar");
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

    let failed = sidecar.answer(Prompt::default()).expect_err("nothing is listening");
    assert_eq!(failed.rule, "assistant_failed", "{}", failed.message);
    assert_eq!(sidecar.health(), Health::Running, "a failed turn killed the sidecar");
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

    let events = sidecar
        .answer(Prompt {
            text: "What can you change about this song?".to_string(),
            model_id: "deepseek/deepseek-v4.1-flash".to_string(),
            ..Prompt::default()
        })
        .expect("the real sidecar answers");
    println!("{events:#?}");

    let recorded = std::fs::read_to_string(&transcript).expect("the transcript is readable");
    let said = text(&events[0]).expect("the first event is the reply");
    assert!(
        recorded.contains(said),
        "the answer is not the transcript's: {said}"
    );
    assert!(
        matches!(
            events.last().and_then(|e| e.event.as_ref()),
            Some(assistant_event::Event::Done(_))
        ),
        "a turn ends with done: {events:?}"
    );
    assert!(
        !events.iter().any(|e| matches!(e.event, Some(assistant_event::Event::Call(_)))),
        "this build proposes nothing"
    );

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
