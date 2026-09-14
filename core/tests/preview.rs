//! `render_preview`: the live process, the stream to it, and which failures are whose
//! (ADR 0013 §2, §3; ADR 0006 §2).
//!
//! **What this suite can test is everything a preview does that is not sound.** CI has no sound
//! card (docs/plan.md, M2 trap 13), so the engine behind these tests is the one `engine.rs` uses —
//! a shell script `core` spawns, which names a socket — and on that socket a model of the
//! engine's side of the stream ([`common::fake_preview`]). What that covers is `core`'s whole
//! half: which process is started and in which mode, that later commands go down the same stream
//! to the same process, the plan a play carries, the answer each command gets and how it is told
//! from the transport's own chatter, what is refused before anything is sent, what an engine with
//! no device or a broken stream reports, and that the process leaves when its stream closes.
//!
//! **What it cannot cover is the engine's half**, which needs a device: the edit a plan builds
//! in a live Tracktion engine, the transport moving, and audio leaving the process. That half is
//! untested *loudly* — `tests/renders.rs` carries it as an ignored test that says why every time
//! the render suite runs, and the engine job asserts that a runner with no sound card gets an exit
//! code and a sentence rather than a silent preview.

mod common;
use common::{fake_engine, fake_preview, fake_server, Heard, Playing, MANIFEST};

use escribass_core::{new_song, Engine, FixedClock, Project, SeededIds, Session};
use escribass_proto::render::preview_command::Command as Sent;
use escribass_proto::render::{PreviewLoop, PreviewSeek, PreviewState, PreviewStop, RenderResult};
use escribass_proto::tools::render_preview_request::Command;
use escribass_proto::tools::{
    AddClipRequest, AddEffectRequest, AddTrackRequest, PreviewFrom, RenderExportRequest,
    RenderPreviewRequest,
};
use escribass_schema::song::device_ref::Kind;
use escribass_schema::song::{Author, DeviceRef, Note, NoteClip, PluginRef, SourceRef, TrackKind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-preview-{}-{}",
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

/// A project with a Surge XT track and one clip of one note, built through the tool API
/// (CLAUDE.md #2), so a play has a plan with something in it.
fn opened(dir: &Scratch) -> Session {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(dir.at("p.escri"), &song, &mut ids, &clock, Author::Model, common::manifest())
            .expect("a project");
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);
    let track = session.add_track(&AddTrackRequest {
        name: "Lead".to_string(),
        kind: TrackKind::Instrument as i32,
        r#ref: Some(surge()),
        dry_run: false,
    });
    let track = added(&track.expect("a track"));
    let note = Note { pitch: 60, start_tick: 0, length_ticks: 480, velocity: 100, ..Default::default() };
    session
        .add_clip(&AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 960,
            content: Some(escribass_proto::tools::add_clip_request::Content::NoteClip(NoteClip {
                notes: [("a".to_string(), note)].into_iter().collect(),
            })),
            dry_run: false,
        })
        .expect("a clip");
    session
}

fn surge() -> DeviceRef {
    DeviceRef {
        kind: Some(Kind::Plugin(PluginRef {
            plugin_id: "Surge Synth Team/Surge XT".to_string(),
            version: "1.3.4".to_string(),
        })),
    }
}

/// An engine script that serves a preview on `heard`'s socket and lives as long as the stream,
/// then leaves with `code` — and, asked to render instead, names `render` when there is one.
/// Every mode it is started in is appended to `modes`, so a test can count processes.
#[cfg(unix)]
fn engine(dir: &Scratch, heard: &Heard, code: i32, render: Option<&Path>) -> PathBuf {
    let render = render.map_or("exit 9".to_string(), |socket| format!("echo unix:{}", socket.display()));
    fake_engine(
        &dir.0,
        &format!(
            "echo \"$2\" >> '{modes}'\n\
             if [ \"$2\" = --render ]; then {render}; exit 0; fi\n\
             echo unix:{socket}\n\
             while [ ! -e '{closed}' ]; do sleep 0.01; done\n\
             echo 'playback stopped' >&2\n\
             echo left > '{left}'\n\
             exit {code}",
            modes = dir.at("modes").display(),
            socket = heard.socket.display(),
            closed = dir.at("closed").display(),
            left = dir.at("left").display(),
        ),
    )
}

fn modes(dir: &Scratch) -> Vec<String> {
    std::fs::read_to_string(dir.at("modes")).unwrap_or_default().lines().map(str::to_string).collect()
}

fn play(start_tick: Option<i32>) -> RenderPreviewRequest {
    RenderPreviewRequest { command: Some(Command::Play(PreviewFrom { start_tick })), dry_run: false }
}

fn seek(tick: i32) -> RenderPreviewRequest {
    RenderPreviewRequest { command: Some(Command::Seek(PreviewSeek { tick })), dry_run: false }
}

fn stop() -> RenderPreviewRequest {
    RenderPreviewRequest { command: Some(Command::Stop(PreviewStop {})), dry_run: false }
}

fn dry(mut request: RenderPreviewRequest) -> RenderPreviewRequest {
    request.dry_run = true;
    request
}

// ---- the process and the stream (ADR 0013 §2, §3) ----

#[cfg(unix)]
#[test]
fn a_play_starts_a_preview_process_and_hands_it_the_plan_compile_produces() {
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing::default());
    session.set_engine(Engine::new(engine(&dir, &heard, 0, None), MANIFEST));

    let answer = session.render_preview(&play(Some(480))).expect("the preview starts");
    assert!(answer.valid, "{:?}", answer.errors);
    let event = answer.event.expect("the engine's answer");
    assert_eq!((event.tick, event.state(), event.applied), (480, PreviewState::Playing, 1));
    assert_eq!(answer.summary, "play from tick 480: 960 ticks, 1 track, 1 clip");

    // The mode is which service the process serves (ADR 0013 §3), and a preview asks for
    // `Preview` — which only holds if `core` spawns it that way.
    assert_eq!(modes(&dir), ["--preview"]);
    assert_eq!(std::fs::read_to_string(dir.at("argument")).unwrap().trim(), MANIFEST);

    // What crossed is the plan `compile` makes of the document, whole — never a diff, never a
    // plan the caller wrote (ADR 0013 §2, ADR 0007 §1).
    let commands = heard.commands();
    let Some(Sent::Play(sent)) = &commands[0].command else { panic!("{commands:?}") };
    let compiled = escribass_core::compile(session.project().song(), &session.project().assets().unwrap())
        .expect("the song compiles");
    assert_eq!(sent.plan.as_ref(), Some(&compiled));
    assert_eq!(sent.start_tick, 480);
}

#[cfg(unix)]
#[test]
fn every_later_command_goes_down_the_same_stream_and_is_answered_by_its_own_event() {
    // The transport writes events of its own while it plays, and one written while a command
    // is on the wire arrives after it was sent and before it was applied. `chatter` is that
    // event, before every answer: a caller that took the next event to arrive would answer a
    // seek to 960 with tick 77777, and this is what says it does not.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing { chatter: Some(77_777), ..Default::default() });
    session.set_engine(Engine::new(engine(&dir, &heard, 0, None), MANIFEST));

    let played = session.render_preview(&play(Some(0))).unwrap().event.unwrap();
    let sought = session.render_preview(&seek(960)).unwrap().event.unwrap();
    let looped = session
        .render_preview(&RenderPreviewRequest {
            command: Some(Command::Loop(PreviewLoop { start_tick: 0, end_tick: 1920 })),
            dry_run: false,
        })
        .unwrap();
    let stopped = session.render_preview(&stop()).unwrap().event.unwrap();

    assert_eq!((played.applied, played.tick), (1, 0));
    assert_eq!((sought.applied, sought.tick), (2, 960));
    assert_eq!(looped.event.unwrap().applied, 3);
    assert_eq!(looped.summary, "loop ticks 0 to 1920");
    assert_eq!((stopped.applied, stopped.state()), (4, PreviewState::Stopped));

    // One process for all four: a stop keeps it, and its plugins, alive (render.proto,
    // `PreviewStop`).
    assert_eq!(modes(&dir), ["--preview"]);
    let kinds: Vec<&str> = heard
        .commands()
        .iter()
        .map(|c| match c.command {
            Some(Sent::Play(_)) => "play",
            Some(Sent::Seek(_)) => "seek",
            Some(Sent::Loop(_)) => "loop",
            Some(Sent::Stop(_)) => "stop",
            None => "none",
        })
        .collect();
    assert_eq!(kinds, ["play", "seek", "loop", "stop"]);
}

#[cfg(unix)]
#[test]
fn a_play_with_no_tick_starts_from_where_the_transport_last_said_it_was() {
    // How an edit made while playing is heard: the plan is compiled again and sent from where
    // playback has reached, which only the transport knows (ADR 0013 §2). A dry run is how a
    // caller reads that without moving it.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing { moves_to: Some(3000), ..Default::default() });
    session.set_engine(Engine::new(engine(&dir, &heard, 0, None), MANIFEST));

    session.render_preview(&play(Some(0))).expect("the preview starts");
    // The move is written after the answer, so give it the moment it takes to arrive: a dry
    // run reports what has been heard, and never waits for more.
    let mut reached = 0;
    for _ in 0..500 {
        let answer = session.render_preview(&dry(stop())).expect("a dry run");
        reached = answer.event.expect("a live transport has a position").tick;
        if reached == 3000 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert_eq!(reached, 3000);
    assert_eq!(heard.commands().len(), 1, "a dry run sent something");

    let answer = session.render_preview(&play(None)).expect("the replacement plays");
    assert_eq!(answer.summary, "play from where it is: 960 ticks, 1 track, 1 clip");
    let Some(Sent::Play(sent)) = &heard.commands()[1].command else { panic!() };
    assert_eq!(sent.start_tick, 3000);
}

#[cfg(unix)]
#[test]
fn closing_the_session_closes_the_stream_and_the_engine_leaves_on_its_own() {
    // The stream's lifetime is the preview's (ADR 0013 §2): nothing sends a "quit", and the
    // engine is not killed. The script writes `left` only if it runs to its own end, which a
    // killed one would not.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing::default());
    session.set_engine(Engine::new(engine(&dir, &heard, 0, None), MANIFEST));
    session.render_preview(&play(Some(0))).expect("the preview starts");
    assert!(!dir.at("left").exists());

    drop(session);
    assert!(dir.at("closed").exists(), "the stream is still open");
    assert!(dir.at("left").exists(), "the engine was killed rather than left");
}

#[cfg(unix)]
#[test]
fn an_export_while_previewing_is_a_fresh_process_of_its_own() {
    // docs/plan.md, M2 trap 6: the one saving that cannot be taken. A plugin's smoothers ramp
    // from their previous value, so an export sharing the playing process would depend on what
    // was played (ADR 0008 §2). It gets a process spawned in `--render` mode, and the preview
    // is still there afterwards, in its own.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing::default());
    let answer = RenderResult { pcm_sha256: "ab".repeat(32), commits: Default::default() };
    let served = fake_server(&dir.0, Ok(answer));
    session.set_engine(Engine::new(engine(&dir, &heard, 0, Some(&served.socket)), MANIFEST));

    session.render_preview(&play(Some(0))).expect("the preview starts");
    let exported = session
        .render_export(&RenderExportRequest {
            output_path: dir.at("out.wav").display().to_string(),
            dry_run: false,
        })
        .expect("the export runs");
    assert!(exported.result.is_some());
    assert!(served.called());
    let stopped = session.render_preview(&stop()).expect("the preview is still live");
    assert_eq!(stopped.event.unwrap().applied, 2, "the same stream, still counting");

    assert_eq!(modes(&dir), ["--preview", "--render"]);
    assert_eq!(heard.commands().len(), 2, "the export sent nothing down the preview's stream");
}

// ---- what a caller can fix: `valid = false`, and nothing sent (ADR 0006 §2) ----

#[cfg(unix)]
#[test]
fn a_dry_run_compiles_and_starts_nothing() {
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing::default());
    session.set_engine(Engine::new(engine(&dir, &heard, 0, None), MANIFEST));

    let answer = session.render_preview(&dry(play(Some(0)))).expect("a dry run");
    assert!(answer.valid);
    assert_eq!(answer.summary, "play from tick 0: 960 ticks, 1 track, 1 clip");
    assert_eq!(answer.event, None, "nothing is playing, so there is nowhere it is");
    assert!(modes(&dir).is_empty(), "a dry run started a process");
}

#[test]
fn a_command_for_a_transport_that_is_not_there_is_refused_and_sends_nothing() {
    // The first command on a stream carries the plan (ADR 0013 §2). A seek, loop or stop with
    // nothing playing is the caller's to fix by playing first — and the engine here does not
    // exist, so anything that tried to start one would fail as an operator error instead.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));

    for request in [seek(0), stop(), dry(stop())] {
        let answer = session.render_preview(&request).expect("a refusal, not an operator error");
        assert!(!answer.valid);
        let rules: Vec<&str> = answer.errors.iter().map(|e| e.rule.as_str()).collect();
        assert_eq!(rules, ["preview_idle"], "{request:?}");
        assert_eq!(answer.event, None);
    }

    // Every reason at once, sorted by path, as `render_export` gives them (ADR 0006 §1).
    let answer = session.render_preview(&seek(-1)).unwrap();
    let rules: Vec<&str> = answer.errors.iter().map(|e| e.rule.as_str()).collect();
    assert_eq!(rules, ["preview_idle", "tick_negative"]);

    let answer = session.render_preview(&RenderPreviewRequest { command: None, dry_run: false }).unwrap();
    assert_eq!(answer.errors[0].rule, "oneof_unset");
}

#[test]
fn what_compile_refuses_is_refused_before_any_engine_starts() {
    let dir = Scratch::new();
    let mut session = opened(&dir);
    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));
    let track = session.project().song().tracks.keys().find(|id| {
        session.project().song().tracks[*id].kind() == TrackKind::Instrument
    });
    session
        .add_effect(&AddEffectRequest {
            track_id: track.expect("the instrument track").clone(),
            r#ref: Some(DeviceRef { kind: Some(Kind::Faust(SourceRef { source_hash: "3c1de4f98b".to_string() })) }),
            index: None,
            dry_run: false,
        })
        .expect("an effect the validator accepts and this engine cannot render");

    let answer = session.render_preview(&play(Some(-5))).expect("a refusal");
    let rules: Vec<&str> = answer.errors.iter().map(|e| e.rule.as_str()).collect();
    assert_eq!(rules, ["tick_negative", "render_unsupported"], "{:?}", answer.errors);
}

// ---- what only an operator can fix: `Err` (ADR 0006 §2) ----

#[test]
fn a_session_told_no_engine_says_so_and_a_dry_run_still_answers() {
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let failed = session.render_preview(&play(Some(0))).expect_err("no engine");
    assert_eq!(failed.rule, "engine_unset", "{}", failed.message);
    assert!(session.render_preview(&dry(play(Some(0)))).unwrap().valid);

    session.set_engine(Engine::new(dir.at("no-such-engine"), MANIFEST));
    assert_eq!(session.render_preview(&play(Some(0))).unwrap_err().rule, "engine_missing");
}

#[cfg(unix)]
#[test]
fn a_machine_with_no_audio_device_is_an_operator_error_carrying_what_the_engine_said() {
    // docs/plan.md, M2 trap 13, from `core`'s side. The engine opens the device before it
    // serves anything, and with none it exits 6 and says why — and that sentence is what a
    // person reads, not "connection refused".
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let script = fake_engine(
        &dir.0,
        "echo 'escribass_engine: no audio output device opened, so there is nothing to play a \
         preview on. ALSA lists 0 output devices.' >&2\nexit 6",
    );
    session.set_engine(Engine::new(script, MANIFEST));

    let failed = session.render_preview(&play(Some(0))).expect_err("no device is not a refusal");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 6"), "{}", failed.message);
    assert!(failed.message.contains("no audio output device"), "{}", failed.message);
    // Nothing is left pretending to play.
    assert_eq!(session.render_preview(&stop()).unwrap().errors[0].rule, "preview_idle");
}

#[cfg(unix)]
#[test]
fn a_stream_the_engine_ends_is_its_failure_and_the_next_play_starts_afresh() {
    // A plan the engine cannot build ends the stream with a status and the process with a
    // non-zero exit (ADR 0013 §2, ADR 0008 §1). The report is the exit and the stderr, and the
    // session forgets the dead preview rather than sending into it.
    let dir = Scratch::new();
    let mut session = opened(&dir);
    let heard = fake_preview(&dir.0, Playing { refuses: Some(2), ..Default::default() });
    session.set_engine(Engine::new(engine(&dir, &heard, 2, None), MANIFEST));

    session.render_preview(&play(Some(0))).expect("the first command is applied");
    let failed = session.render_preview(&seek(480)).expect_err("the engine refused it");
    assert_eq!(failed.rule, "engine_failed", "{}", failed.message);
    assert!(failed.message.contains("exited 2"), "{}", failed.message);
    assert!(failed.message.contains("playback stopped"), "{}", failed.message);
    assert_eq!(session.render_preview(&seek(480)).unwrap().errors[0].rule, "preview_idle");
}

/// The id of the entity a tool's patch added, read from the patch it returned.
fn added(result: &escribass_proto::tools::ToolResult) -> String {
    let ops: serde_json::Value =
        serde_json::from_slice(&result.patch).expect("the patch is RFC 6902 text");
    let path = ops[0]["path"].as_str().expect("an op has a path");
    path.rsplit('/').next().expect("a path has a last segment").to_string()
}
