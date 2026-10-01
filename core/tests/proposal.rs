//! The proposal: the model's calls applied to a fork, and one entry a person approves
//! (ADR 0019).
//!
//! Every project here is built by `Project::create` and changed only through `Session` and
//! `core::call`, so CLAUDE.md #2 holds: no test writes `song.json` or hand-builds an entry.
//!
//! The suite opens with the premise rather than assuming it. ADR 0019 exists because **dry runs
//! do not compose**, and the first test is that failure reproduced: two dry runs, the second
//! naming the id the first returned, refused `track_unknown`. Everything after it is what a
//! fork buys.

mod common;
use common::manifest;

use escribass_core::{call, FixedClock, Project, SeededIds, Session, IMPLEMENTED, OFFERED};
use escribass_proto::tools::{AddClipRequest, AddTrackRequest, TransposeRequest};
use escribass_schema::song::{Author, Song};
use serde_json::{json, Map, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../tests/fixtures/song/minimal.json");
const AT: i64 = 1_788_307_200_000;
const CLIP: &str = "01M1FPMP00CPCHRS0000000006";
const BASS: &str = "01M1FPMP00TRACKBASS0000002";
/// The first id a tool mints in a fresh session over the fixture: `Project::create` takes
/// counter 1 for the root entry. Named literally, as a determinism script's ids are, so a
/// change in mint order fails loudly rather than quietly still passing (`tests/AGENTS.md`).
const MINTED: &str = "01M1FPMP000000000000000002";
const SURGE: &str = r#"{"plugin": {"plugin_id": "Surge Synth Team/Surge XT", "version": "1.3.4"}}"#;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-proposal-{}-{}.escri",
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

fn fixture_song() -> Song {
    serde_json::from_str(&std::fs::read_to_string(FIXTURE).expect("the fixture is readable"))
        .expect("the fixture is a song")
}

fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project =
        Project::create(&dir.0, &fixture_song(), &mut ids, &clock, Author::Human, manifest())
            .expect("the project is created");
    let session = Session::new(project, Box::new(ids), Box::new(clock), Author::Human);
    (dir, session)
}

fn args(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("arguments are an object, not {other}"),
    }
}

/// One of the model's calls, with the two ids ADR 0021 §2 stamps on whatever it mints.
fn proposed(session: &mut Session, name: &str, arguments: Value, call_id: &str) -> Value {
    let proposal = session.proposal_mut().expect("a proposal is open");
    match proposal.call(name, &args(arguments), "deepseek/deepseek-v4.1-flash", call_id) {
        Ok(answer) => answer.structured,
        Err(e) => json!({"error": e.message, "kind": format!("{:?}", e.kind)}),
    }
}

fn add_lead() -> Value {
    json!({"name": "Lead", "kind": 1, "ref": serde_json::from_str::<Value>(SURGE).unwrap()})
}

fn clip_on(track: &str) -> Value {
    json!({"track_id": track, "start_tick": 0, "length_ticks": 960, "note_clip": {"notes": {}}})
}

// ---------------------------------------------------------------------------
// The premise (docs/plan.md, M3 trap 8)
// ---------------------------------------------------------------------------

#[test]
fn dry_runs_do_not_compose_which_is_why_a_proposal_exists() {
    // The measured failure ADR 0019 was written from, reproduced here so the rest of this file
    // is a fix for something rather than a design nobody checked. A dry run mints real ids from
    // a fork it then puts back: the track the first call previewed is in a document that was
    // thrown away, and the second call naming it is refused.
    //
    // The spike met this live — V4.1 Flash applied `add_track`, dry-ran `add_clip` on an id an
    // earlier dry run had shown, and was told `track_unknown`, twice.
    let (_dir, mut session) = opened();

    let previewed = session
        .add_track(&AddTrackRequest {
            name: "Lead".to_string(),
            kind: 1,
            r#ref: Some(serde_json::from_str(SURGE).unwrap()),
            dry_run: true,
        })
        .expect("a dry run answers");
    assert!(previewed.valid);
    let patch: Value = serde_json::from_slice(&previewed.patch).expect("the patch is JSON");
    assert!(
        patch.to_string().contains(MINTED),
        "the dry run minted a different id: {patch}"
    );

    let refused = session
        .add_clip(&AddClipRequest {
            track_id: MINTED.to_string(),
            start_tick: 0,
            length_ticks: 960,
            content: None,
            dry_run: true,
        })
        .expect("a dry run answers");
    assert!(!refused.valid);
    assert_eq!(
        refused.errors.iter().map(|e| e.rule.as_str()).collect::<Vec<_>>(),
        ["track_unknown"],
        "dry runs composed, and ADR 0019 has no premise"
    );
}

// ---------------------------------------------------------------------------
// The fork (ADR 0019 §1)
// ---------------------------------------------------------------------------

#[test]
fn a_second_call_names_the_first_calls_track_and_is_accepted() {
    // The same two calls, through a proposal. The fork is never put back, so the track is
    // still there under the id the first call's result returned — which is what every model in
    // the spike did with the ids a real apply returned.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");

    let first = proposed(&mut session, "add_track", add_lead(), "call_1");
    assert_eq!(first["valid"], json!(true), "{first}");
    assert_eq!(first["entry_id"], json!(""), "a proposal's call records nothing");
    // The id the model reads off this result, taken from the answer rather than from a
    // constant: this is the id it will name next.
    let minted = first["patch"].to_string();
    assert!(minted.contains(MINTED), "{minted}");

    let second = proposed(&mut session, "add_clip", clip_on(MINTED), "call_2");
    assert_eq!(second["valid"], json!(true), "{second}");

    let proposal = session.proposal().expect("still pending");
    assert!(proposal.song().tracks.contains_key(MINTED));
    assert_eq!(proposal.song().clips.values().filter(|c| c.track_id == MINTED).count(), 1);
    assert_eq!(proposal.calls(), ["add_track", "add_clip"]);

    // And the project is exactly as it was: no entry, no track, nothing written (ADR 0019 §1).
    assert_eq!(session.project().history().entries().len(), 1);
    assert!(!session.project().song().tracks.contains_key(MINTED));
    let on_disk: Song = escribass_core::from_canonical_json(
        &std::fs::read_to_string(session.project().root().join("song.json")).expect("song.json"),
    )
    .expect("song.json is a song");
    assert_eq!(&on_disk, session.project().song());
}

#[test]
fn a_dry_run_inside_a_proposal_is_a_dry_run_of_the_fork() {
    // `dry_run` is out of the schemas the model sees (ADR 0022 §1), and the field stays on the
    // wire: a model that sends it from habit gets a dry run of the fork — prepared and
    // described, with nothing swapped in — so the models that preview first get the same
    // answers as the ones that do not (ADR 0019 §1).
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");

    let mut previewing = add_lead();
    previewing["dry_run"] = json!(true);
    let previewed = proposed(&mut session, "add_track", previewing, "call_1");
    assert_eq!(previewed["valid"], json!(true), "{previewed}");
    assert!(!session.proposal().expect("pending").song().tracks.contains_key(MINTED));
    // The fork was not advanced, so the id is still there for the call that applies it.
    let applied = proposed(&mut session, "add_track", add_lead(), "call_2");
    assert!(applied["patch"].to_string().contains(MINTED), "{applied}");
}

#[test]
fn a_refused_call_changes_nothing_and_burns_no_id() {
    // The refusals script's claim, one layer over: a call the validator refuses leaves the
    // fork's document and the fork's id source exactly where they were.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");

    let refused = proposed(&mut session, "add_clip", clip_on("01M1FPMP00NOSUCHTRACK00000"), "c1");
    assert_eq!(refused["valid"], json!(false), "{refused}");
    assert_eq!(refused["errors"][0]["rule"], json!("track_unknown"));
    assert!(session.proposal().expect("pending").calls().is_empty());

    let accepted = proposed(&mut session, "add_track", add_lead(), "c2");
    assert!(accepted["patch"].to_string().contains(MINTED), "a refusal burned an id: {accepted}");
}

#[test]
fn a_tool_the_model_was_not_offered_is_refused_and_writes_nothing() {
    // The one gate, and it is about what is **offered** to one author, never about what is
    // accepted (ADR 0022 §1; M3 trap 18). It is also what keeps the fork harmless: `undo`,
    // `create_branch` and `add_asset` write to the project root straight out, and the fork's
    // root is the real project's.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");

    for withheld in ["undo", "create_branch", "add_asset", "get_song", "set_param"] {
        let answered = proposed(&mut session, withheld, json!({}), "c1");
        assert_eq!(answered["kind"], json!("BadRequest"), "`{withheld}`: {answered}");
        assert!(
            answered["error"].as_str().is_some_and(|said| said.contains("not one of the tools")),
            "`{withheld}`: {answered}"
        );
    }
    assert_eq!(session.project().history().entries().len(), 1);
    assert_eq!(session.project().history().refs().refs.len(), 1);
}

#[test]
fn one_proposal_at_a_time_and_rejecting_writes_nothing() {
    let (_dir, mut session) = opened();
    session.propose("first").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");

    let refused = session.propose("second").expect_err("one at a time");
    assert_eq!(refused.rule, "proposal_pending", "{}", refused.message);

    assert!(session.reject());
    assert!(!session.reject(), "there is nothing left to reject");
    // Nothing written means nothing to undo and nothing in the log (ADR 0019 §3).
    assert_eq!(session.project().history().entries().len(), 1);
    assert!(!session.project().song().tracks.contains_key(MINTED));
    session.propose("second").expect("the next prompt goes through");
}

// ---------------------------------------------------------------------------
// The one patch (ADR 0019 §2)
// ---------------------------------------------------------------------------

#[test]
fn applying_records_one_entry_whatever_the_number_of_calls() {
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "call_1");
    proposed(&mut session, "add_clip", clip_on(MINTED), "call_2");
    proposed(&mut session, "set_tempo", json!({"tick": 3840, "bpm": 132.0}), "call_3");

    let applied = session
        .apply_proposal("deepseek/deepseek-v4.1-flash")
        .expect("the project writes");
    assert!(applied.valid, "{applied:?}");
    assert_ne!(applied.entry_id, "");

    // **One entry**, whatever the number of calls (ADR 0017 §1, ADR 0019 §2).
    assert_eq!(session.project().history().entries().len(), 2);
    let entry = session.project().history().get(&applied.entry_id).expect("the entry");
    assert_eq!(entry.tool, "proposal");
    assert!(session.proposal().is_none(), "an applied proposal is still pending");

    // The ids the model was shown are the ids in the log: the fork's, not a fresh mint
    // (ADR 0012 §4, one process over).
    assert!(session.project().song().tracks.contains_key(MINTED));
    assert_eq!(session.project().song().tempo_map.as_ref().unwrap().events.len(), 2);
    let ops = escribass_core::ops_text(&escribass_core::ops_of(entry).expect("ops"));
    assert!(ops.contains(MINTED), "the entry does not name the previewed track");
}

#[test]
fn applying_takes_the_session_past_every_id_the_proposal_minted() {
    // `from_tool`'s rule, for a turn instead of a call: the fork keeps its ids across calls
    // (ADR 0019 §1), so when the proposal applies the session has to adopt them or it mints
    // over what the proposal just created.
    //
    // Watched failing first, and found by reading a golden rather than by reasoning: the
    // end-to-end entry id and the track the proposal minted were the same 26 characters, and
    // the session's next call would have minted the instrument's.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");
    let applied = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it applies");

    // `add_track` minted a track and an instrument, so the entry is the third id.
    assert_eq!(applied.entry_id, "01M1FPMP000000000000000004");
    assert!(session.project().song().tracks.contains_key(MINTED));

    // And the next call the window makes mints past all of it.
    let next = session
        .add_track(&AddTrackRequest {
            name: "Pad".to_string(),
            kind: 1,
            r#ref: Some(serde_json::from_str(SURGE).unwrap()),
            dry_run: false,
        })
        .expect("a person's own call applies");
    let patch: Value = serde_json::from_slice(&next.patch).expect("a patch is JSON");
    assert!(patch.to_string().contains("01M1FPMP000000000000000005"), "{patch}");
    assert_eq!(session.project().song().tracks.len(), 5);
}

#[test]
fn a_rejected_proposal_burns_no_id_at_all() {
    // The other side: nothing was written, so nothing was minted. A fork's ids go back with
    // the fork (ADR 0019 §3), exactly as a dry run's do (ADR 0006 §3).
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");
    proposed(&mut session, "add_clip", clip_on(MINTED), "c2");
    assert!(session.reject());

    let next = session
        .add_track(&AddTrackRequest {
            name: "Pad".to_string(),
            kind: 1,
            r#ref: Some(serde_json::from_str(SURGE).unwrap()),
            dry_run: false,
        })
        .expect("a person's own call applies");
    let patch: Value = serde_json::from_slice(&next.patch).expect("a patch is JSON");
    assert!(patch.to_string().contains(MINTED), "a rejected proposal burned an id: {patch}");
}

#[test]
fn the_entry_names_the_model_and_the_prompt_and_each_entity_names_its_call() {
    // ADR 0021 §2, the whole of it in one project. The entry carries the turn — author, the
    // model that answered, the prompt — and **no** `tool_call_id`, since a composed proposal
    // has several. Each entity carries the call that minted it, so a row in the log leads back
    // to a line of the conversation.
    let (_dir, mut session) = opened();
    session.propose("the-prompts-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "call_1");
    proposed(&mut session, "add_clip", clip_on(MINTED), "call_2");
    let applied = session.apply_proposal("openai/gpt-5-mini").expect("it applies");

    let entry = session.project().history().get(&applied.entry_id).expect("the entry").clone();
    let made = entry.provenance.expect("an entry carries provenance");
    assert_eq!(made.author, Author::Model as i32);
    assert_eq!(made.model_id.as_deref(), Some("openai/gpt-5-mini"));
    assert_eq!(made.prompt_id.as_deref(), Some("the-prompts-hash"));
    assert_eq!(made.tool_call_id, None, "a composed proposal has several calls");

    let song = session.project().song();
    let track = song.tracks[MINTED].provenance.clone().expect("a track carries provenance");
    assert_eq!(track.author, Author::Model as i32);
    assert_eq!(track.model_id.as_deref(), Some("deepseek/deepseek-v4.1-flash"));
    assert_eq!(track.prompt_id.as_deref(), Some("the-prompts-hash"));
    assert_eq!(track.tool_call_id.as_deref(), Some("call_1"));
    let clip = song
        .clips
        .values()
        .find(|clip| clip.track_id == MINTED)
        .and_then(|clip| clip.provenance.clone())
        .expect("a clip carries provenance");
    assert_eq!(clip.tool_call_id.as_deref(), Some("call_2"), "the second call minted the clip");

    // And the entities the proposal did not create keep theirs: an edit does not change who
    // made a thing (ADR 0021 §1).
    let bass = song.tracks[BASS].provenance.clone().expect("provenance");
    assert_eq!(bass.author, Author::Human as i32);
    assert_eq!(bass.tool_call_id, None);
}

#[test]
fn a_track_the_model_edited_three_times_is_one_version_on() {
    // The fork's per-call bumps are scaffolding and never reach the log: what a person
    // approved is one change, so the entry says `before + 1` (ADR 0019 §2). Under
    // `prepare_merge`'s `max(ours, theirs) + 1` this track would be at `before + 4`.
    let (_dir, mut session) = opened();
    let before = session.project().song().clips[CLIP].version;
    session.propose("prompt-hash").expect("a proposal opens");
    for (position, call_id) in ["c1", "c2", "c3"].iter().enumerate() {
        let answered = proposed(
            &mut session,
            "transpose",
            json!({"clip_id": CLIP, "semitones": 1}),
            call_id,
        );
        assert_eq!(answered["valid"], json!(true), "{answered}");
        assert_eq!(
            session.proposal().expect("pending").song().clips[CLIP].version,
            before + position as u32 + 1,
            "the fork bumps per call, which is what has to be undone"
        );
    }

    session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it applies");
    assert_eq!(session.project().song().clips[CLIP].version, before + 1);
}

#[test]
fn a_document_that_moved_under_the_proposal_is_refused() {
    // ADR 0019 §2's promise, and from 2026-09-24 it is kept by the thing that can keep it:
    // `Proposal::base` is the document the patch was computed against, so a path the patch
    // writes to whose value has since moved is refused by comparison, not by inference from a
    // version number. Two edits here rather than one only because this test predates the
    // amendment; one is refused too, by the test below.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "transpose", json!({"clip_id": CLIP, "semitones": 1}), "c1");

    for _ in 0..2 {
        session
            .transpose(&TransposeRequest {
                clip_id: CLIP.to_string(),
                semitones: 1,
                note_ids: vec![],
                dry_run: false,
            })
            .expect("a person's own edit applies");
    }

    let refused = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it answers");
    assert!(!refused.valid, "{refused:?}");
    let rules: Vec<&str> = refused.errors.iter().map(|e| e.rule.as_str()).collect();
    assert!(rules.contains(&"document_moved"), "{rules:?}");
    // Named, so a person can be told *what* moved rather than a number (ADR 0019 §2).
    assert!(
        refused.errors.iter().any(|e| e.path.contains(CLIP)),
        "the refusal does not name the clip: {refused:?}"
    );
    // Still pending: a refusal is something a person acts on, not a reason to lose their turn.
    assert!(session.proposal().is_some());
}

#[test]
fn exactly_one_intervening_edit_is_refused_and_the_refusal_names_what_moved() {
    // **This test used to be called `exactly_one_intervening_edit_is_the_guards_blind_spot`,
    // and it asserted the opposite.** It was right about the mechanism and it was written down
    // honestly: ADR 0005 §3 reads a claim equal to the number core computes as disputing
    // nothing, a proposal's patch claims `before + 1`, and after exactly one intervening entry
    // the entity *holds* `before + 1` — so the proposal merged over a person's edit in
    // silence, and the person's five semitones were gone with nothing said.
    //
    // What changed on 2026-09-24 is not the version rule, which is still ADR 0005's and is
    // still a held drag's. It is that a proposal knows something a drag does not: the document
    // its patch was computed against. `Proposal::moved_under` compares the paths the patch
    // writes with what those paths hold now, which answers the question the version number was
    // standing in for. A proposal sits pending for as long as a person takes to read it, so
    // this was the likeliest concurrent edit there is (M3 review, 2026-09-24).
    //
    // Watched failing first: before `moved_under` this ran green with the apply *valid* and
    // the person's pitch of 43 overwritten by the model's 39.
    let (_dir, mut session) = opened();
    let before = session.project().song().clips[CLIP].version;
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "transpose", json!({"clip_id": CLIP, "semitones": 1}), "c1");

    session
        .transpose(&TransposeRequest {
            clip_id: CLIP.to_string(),
            semitones: 5,
            note_ids: vec![],
            dry_run: false,
        })
        .expect("a person's own edit applies");
    assert_eq!(session.project().song().clips[CLIP].version, before + 1);

    let refused = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it answers");
    assert!(!refused.valid, "one intervening edit merged in silence: {refused:?}");
    assert!(
        refused.errors.iter().all(|e| e.rule == "document_moved"),
        "{refused:?}"
    );
    // **What moved**, and at the granularity the patch writes at: the notes the person
    // transposed, each named by its own path, so the panel can say which and not "a number".
    assert!(
        refused.errors.iter().all(|e| e.path.contains(CLIP)),
        "the refusal does not name what moved: {refused:?}"
    );
    // And the person's five semitones are still there: nothing was written over.
    let content = session.project().song().clips[CLIP].content.clone();
    let Some(escribass_schema::song::clip::Content::NoteClip(notes)) = content else {
        panic!("the fixture's clip is a note clip");
    };
    let pitches: Vec<i32> = notes.notes.values().map(|note| note.pitch).collect();
    assert!(pitches.contains(&43), "the person's own edit was overwritten: {pitches:?}");
    assert!(!pitches.contains(&39), "the proposal's pitch landed: {pitches:?}");
    // Still pending, as every refusal leaves it: Reject and ask again is the way out
    // (ADR 0019 §2).
    assert!(session.proposal().is_some());
}

#[test]
fn a_second_edit_anywhere_refuses_the_proposal_at_the_song() {
    // The other end of the same rule, and the reason "an edit elsewhere merges cleanly" holds
    // for one edit and not for two: **every** change bumps the `Song`'s own `version`, so a
    // proposal's claim on `/version` goes stale after two entries whether or not it touched
    // anything the person did. The refusal then names `/version` — the song — which is the
    // least useful thing it could name, and is why ADR 0019 §2's way out is Reject and ask
    // again rather than a merge.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");

    for semitones in [1, -1] {
        session
            .transpose(&TransposeRequest {
                clip_id: CLIP.to_string(),
                semitones,
                note_ids: vec![],
                dry_run: false,
            })
            .expect("a person's own edit applies");
    }

    let refused = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it answers");
    assert!(!refused.valid, "{refused:?}");
    assert_eq!(
        refused.errors.iter().map(|e| e.path.as_str()).collect::<Vec<_>>(),
        ["/version"]
    );
    assert!(session.proposal().is_some(), "a refused apply threw the turn away");
}

#[test]
fn an_edit_elsewhere_merges_cleanly() {
    // The other half of the same rule: a person editing something the proposal did not touch
    // is not a dispute, and the proposal applies over it (ADR 0019 §2) — for **one** edit. See
    // `a_second_edit_anywhere_refuses_the_proposal_at_the_song` for where that stops.
    let (_dir, mut session) = opened();
    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");

    session
        .transpose(&TransposeRequest {
            clip_id: CLIP.to_string(),
            semitones: 2,
            note_ids: vec![],
            dry_run: false,
        })
        .expect("a person's own edit applies");

    let applied = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it applies");
    assert!(applied.valid, "{applied:?}");
    assert!(session.project().song().tracks.contains_key(MINTED));
}

// ---------------------------------------------------------------------------
// The list (ADR 0022 §1)
// ---------------------------------------------------------------------------

#[test]
fn offered_is_a_subset_of_implemented_in_its_order() {
    assert_eq!(OFFERED.len(), 12, "ADR 0022 §1 offers twelve");
    let kept: Vec<&&str> = IMPLEMENTED.iter().filter(|name| OFFERED.contains(name)).collect();
    assert_eq!(
        kept.into_iter().copied().collect::<Vec<&str>>(),
        OFFERED.to_vec(),
        "`OFFERED` is `IMPLEMENTED` filtered, in its order"
    );
    // Every withheld tool is withheld on purpose, and the count says so: fifteen from M4 PR
    // 5, which implements `define_generator` and `compile_generator` and offers neither.
    // ADR 0026 §3 makes them the thirteenth and fourteenth offered, and M4 PR 6 is where
    // that lands — with the view line a model needs in order to know what there is to
    // compile, which is the half that would be missing if this number moved here.
    assert_eq!(IMPLEMENTED.len() - OFFERED.len(), 15);
    for later in ["define_generator", "compile_generator"] {
        assert!(IMPLEMENTED.contains(&later), "`{later}` is implemented from M4 PR 5");
        assert!(!OFFERED.contains(&later), "`{later}` is offered in M4 PR 6, not here");
    }
    assert!(OFFERED.contains(&"apply_patch"), "no typed tool sets a mix (ADR 0022 §1)");
    for withheld in ["get_song", "set_param", "undo", "render_preview", "merge_branch"] {
        assert!(!OFFERED.contains(&withheld), "`{withheld}` is offered");
    }
}

#[test]
fn the_schemas_the_model_sees_are_the_descriptors_own_without_dry_run() {
    let offered = escribass_core::offered_schemas(escribass_proto::DESCRIPTOR).expect("schemas");
    assert_eq!(
        offered.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>(),
        OFFERED.to_vec()
    );
    let every = escribass_core::tool_schemas(escribass_proto::DESCRIPTOR).expect("schemas");
    for tool in &offered {
        assert!(
            !tool.input_schema["properties"].as_object().expect("properties").contains_key("dry_run"),
            "`{}` still advertises `dry_run`",
            tool.name
        );
        // Everything else is the descriptor's own, untouched: no second description, and no
        // field quietly dropped beside `dry_run` (ADR 0006 §6, M3 trap 11).
        let whole = every.iter().find(|t| t.name == tool.name).expect("the same tool");
        let mut without = whole.input_schema.clone();
        without["properties"].as_object_mut().expect("properties").remove("dry_run");
        assert_eq!(tool.input_schema, without, "`{}` differs by more than `dry_run`", tool.name);
        assert_eq!(tool.description, whole.description);
    }
    // And the one thing a filter could get wrong quietly: `dry_run` was actually there.
    let before = every.iter().find(|t| t.name == "add_track").expect("add_track");
    assert!(before.input_schema["properties"]["dry_run"].is_object());
}

// ---------------------------------------------------------------------------
// The cost (docs/plan.md, M3 trap 9)
// ---------------------------------------------------------------------------

#[test]
fn a_proposal_of_many_calls_drives_one_write() {
    // Trap 9 re-measured, as ADR 0019's Consequences ask. `Project::write` is O(history) — 17 ms
    // at 21 entries and 30.6 ms at 300, measured in M2 PR 5 — and the loop is the first author
    // with no person between calls. A branch per proposal would have paid that per model call;
    // a fork pays it once, at approval.
    //
    // The numbers are printed rather than asserted: a threshold on a wall clock is a flaky
    // test, and what this asserts is the shape — **one** entry and one write for six calls.
    let (_dir, mut session) = opened();
    for step in 0..20 {
        session
            .transpose(&TransposeRequest {
                clip_id: CLIP.to_string(),
                semitones: if step % 2 == 0 { 1 } else { -1 },
                note_ids: vec![],
                dry_run: false,
            })
            .expect("a person's edit applies");
    }
    let before = session.project().history().entries().len();

    session.propose("prompt-hash").expect("a proposal opens");
    let calling = std::time::Instant::now();
    proposed(&mut session, "add_track", add_lead(), "c1");
    proposed(&mut session, "add_clip", clip_on(MINTED), "c2");
    for (position, call_id) in ["c3", "c4", "c5", "c6"].iter().enumerate() {
        let answered = proposed(
            &mut session,
            "transpose",
            json!({"clip_id": CLIP, "semitones": if position % 2 == 0 { 1 } else { -1 }}),
            call_id,
        );
        assert_eq!(answered["valid"], json!(true), "{answered}");
    }
    let calls = calling.elapsed();

    let applying = std::time::Instant::now();
    let applied = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it applies");
    let apply = applying.elapsed();
    assert!(applied.valid, "{applied:?}");

    println!(
        "trap 9, re-measured at {before} entries: six proposed calls in {:.1} ms \
         (no write at all), one apply in {:.1} ms (one `Project::write`)",
        calls.as_secs_f64() * 1000.0,
        apply.as_secs_f64() * 1000.0,
    );
    assert_eq!(session.project().history().entries().len(), before + 1);
}

// ---------------------------------------------------------------------------
// What the project records (ADR 0021 §4)
// ---------------------------------------------------------------------------

#[test]
fn the_model_is_recorded_on_first_use_and_never_rewritten() {
    let (dir, mut session) = opened();
    let lock = || {
        serde_json::from_str::<Value>(
            &std::fs::read_to_string(dir.0.join("lock.json")).expect("lock.json"),
        )
        .expect("lock.json is JSON")
    };
    // A project written before a prompt was sent carries no `ai` block at all, which is what
    // keeps every lock.json this repository has already blessed exactly where it is.
    assert_eq!(lock().get("ai"), None, "a fresh project records a model");
    assert_eq!(session.ai(), (escribass_core::DEFAULT_PROVIDER, escribass_core::DEFAULT_MODEL));

    session.record_ai("openrouter", "deepseek/deepseek-v4.1-flash").expect("it writes");
    assert_eq!(lock()["ai"]["model"], json!("deepseek/deepseek-v4.1-flash"));
    assert_eq!(lock()["ai"]["provider"], json!("openrouter"));

    // Never rewritten by a tool: changing it is editing the text (§2.6, ADR 0010 §3).
    session.record_ai("openrouter", "openai/gpt-5-mini").expect("it answers");
    assert_eq!(lock()["ai"]["model"], json!("deepseek/deepseek-v4.1-flash"));

    // And it survives a reopen, which is where the loop reads it (ADR 0021 §4).
    let reopened = Project::open(&dir.0, manifest()).expect("it reopens");
    assert_eq!(reopened.ai().map(|ai| ai.model.as_str()), Some("deepseek/deepseek-v4.1-flash"));
}

#[test]
fn the_default_model_is_the_one_lock_baseline_records() {
    // A constant in code and a row in a JSON file are two places to say one thing, and this is
    // what keeps them from drifting apart (ADR 0021 §4).
    let baseline: Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../lock.baseline.json"))
            .expect("lock.baseline.json is readable"),
    )
    .expect("lock.baseline.json is JSON");
    assert_eq!(baseline["ai"]["model"], json!(escribass_core::DEFAULT_MODEL));
    assert_eq!(baseline["ai"]["provider"], json!(escribass_core::DEFAULT_PROVIDER));
}

/// `call` is the dispatch a proposal runs through; naming it here keeps the import honest.
#[allow(dead_code)]
fn _dispatch_is_the_one_that_exists(session: &mut Session) {
    let _ = call(session, "get_song", &Map::new());
}

// ---------------------------------------------------------------------------
// The three controls (ADR 0019 §3; docs/specs.md §9)
// ---------------------------------------------------------------------------

/// Every byte under the project directory, so "records nothing" can be asserted rather than
/// sampled.
///
/// A count of entries is the check that cannot fail here (docs/plan.md, M3 trap 1): a Reject
/// that quietly wrote `song.json`, moved a ref or touched `lock.json` leaves the entry count
/// exactly where it was. What Reject promises is that the project did not move at all, so
/// that is what is compared.
fn everything_under(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    fn walk(
        at: &std::path::Path,
        under: &str,
        into: &mut std::collections::BTreeMap<String, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(at).expect("the project directory is readable") {
            let entry = entry.expect("a directory entry");
            let name = format!("{under}{}", entry.file_name().to_string_lossy());
            if entry.path().is_dir() {
                walk(&entry.path(), &format!("{name}/"), into);
            } else {
                into.insert(name, std::fs::read(entry.path()).expect("a file is readable"));
            }
        }
    }
    let mut found = std::collections::BTreeMap::new();
    walk(root, "", &mut found);
    found
}

#[test]
fn rejecting_leaves_the_project_byte_for_byte_where_it_was() {
    // ADR 0019 §3: Reject drops the fork and **records nothing**. Nothing was written, so
    // there is nothing to undo — and the way to watch that is to compare the bytes, since a
    // Reject that wrote an entry would pass every test that only counted entries.
    let (dir, mut session) = opened();
    let before = everything_under(&dir.0);

    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");
    proposed(&mut session, "add_clip", clip_on(MINTED), "c2");
    proposed(&mut session, "set_tempo", json!({"tick": 3840, "bpm": 132.0}), "c3");
    assert_eq!(session.proposal().expect("pending").calls().len(), 3);

    assert!(session.reject());
    let after = everything_under(&dir.0);
    assert_eq!(
        before.keys().collect::<Vec<_>>(),
        after.keys().collect::<Vec<_>>(),
        "a rejected proposal added or removed a file"
    );
    for (name, bytes) in &before {
        assert_eq!(bytes, &after[name], "a rejected proposal changed `{name}`");
    }
}

#[test]
fn editing_applies_the_persons_patch_as_the_persons_with_the_prompt_kept() {
    // ADR 0019 §3, and ADR 0021 §1's worked example. The person edits the proposal's patch
    // text; what lands is an `apply_patch` whose entry says `AUTHOR_HUMAN` and **no**
    // `model_id`, with the `prompt_id` kept so the row still leads to the conversation — and
    // whose new entities carry the person's provenance rather than the model's, because they
    // are new and `prepare` decides a new entity's (ADR 0021 §1).
    let (_dir, mut session) = opened();
    session.propose("the-prompts-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "call_1");

    let pending = session.proposal().expect("pending").patch().expect("the patch is computable");
    let text = escribass_core::ops_text(pending.ops());
    // The person's own change to the model's proposal, which is the whole point of the control.
    let edited = text.replace("\"Lead\"", "\"Lead (mine)\"");
    assert_ne!(edited, text, "the patch does not name the track the model proposed");

    let applied = session.edit_proposal(edited.as_bytes()).expect("the project writes");
    assert!(applied.valid, "{applied:?}");
    assert!(session.proposal().is_none(), "an applied edit left the proposal pending");

    let entry = session.project().history().get(&applied.entry_id).expect("the entry").clone();
    assert_eq!(entry.tool, "apply_patch", "an edit is the person's own apply_patch");
    let made = entry.provenance.expect("an entry carries provenance");
    assert_eq!(made.author, Author::Human as i32);
    assert_eq!(made.model_id, None, "a person who changed the bytes owns the bytes");
    assert_eq!(made.prompt_id.as_deref(), Some("the-prompts-hash"));
    assert_eq!(made.tool_call_id, None);

    let track = session.project().song().tracks[MINTED].clone();
    assert_eq!(track.name, "Lead (mine)");
    let stamped = track.provenance.expect("a track carries provenance");
    assert_eq!(stamped.author, Author::Human as i32, "the entity kept the model's author");
    assert_eq!(stamped.model_id, None);
    assert_eq!(stamped.prompt_id.as_deref(), Some("the-prompts-hash"));
    assert_eq!(stamped.tool_call_id, None, "the entity kept the model's call id");
}

#[test]
fn editing_takes_the_session_past_every_id_the_proposal_minted() {
    // `apply_proposal`'s rule, for the control beside it and for the same reason: the text a
    // person edits names the ids the **fork** minted, so applying it puts them in the project
    // and a session that had not moved past them would mint one of them again
    // (`entry_exists`, or two entities under one ULID).
    let (_dir, mut session) = opened();
    session.propose("the-prompts-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "call_1");
    let pending = session.proposal().expect("pending").patch().expect("computable");
    let text = escribass_core::ops_text(pending.ops());

    let applied = session.edit_proposal(text.as_bytes()).expect("the project writes");
    assert!(applied.valid, "{applied:?}");
    // `add_track` minted a track and an instrument, so the entry is the third id.
    assert_eq!(applied.entry_id, "01M1FPMP000000000000000004");
    assert!(session.project().song().tracks.contains_key(MINTED));

    let next = session
        .add_track(&AddTrackRequest {
            name: "Pad".to_string(),
            kind: 1,
            r#ref: Some(serde_json::from_str(SURGE).unwrap()),
            dry_run: false,
        })
        .expect("a person's own call applies");
    let patch: Value = serde_json::from_slice(&next.patch).expect("a patch is JSON");
    assert!(
        patch.to_string().contains("01M1FPMP000000000000000005"),
        "the session minted over what the edit applied: {patch}"
    );
    assert_eq!(session.project().song().tracks.len(), 5);
}

#[test]
fn an_edit_that_will_not_apply_is_refused_and_the_proposal_stays_pending() {
    // ADR 0017 §3's refusal path, which ADR 0019 §3 asks for by name: a patch a person
    // mistyped is shown refused with its rule, and the thing they were editing is still there
    // to edit. Three ways in, because they leave by three different doors in `core`.
    let (_dir, mut session) = opened();
    session.propose("the-prompts-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "call_1");

    // 1. Not RFC 6902 at all.
    let refused = session.edit_proposal(b"{ this is not a patch").expect("a refusal is an answer");
    assert!(!refused.valid);
    assert_eq!(refused.errors[0].rule, "patch_unreadable");
    assert!(session.proposal().is_some(), "a mistyped patch threw the turn away");

    // 2. A patch the document will not take.
    let refused = session
        .edit_proposal(br#"[{"op": "replace", "path": "/tracks/nope/name", "value": "x"}]"#)
        .expect("a refusal is an answer");
    assert!(!refused.valid, "{refused:?}");
    assert!(session.proposal().is_some(), "a patch that would not apply threw the turn away");

    // 3. A patch that applies to nothing. There is no entry, so there is nothing to say
    //    happened, and the proposal in front of the person is still the thing to decide.
    let empty = session.edit_proposal(b"[]").expect("an empty patch is an answer");
    assert_eq!(empty.entry_id, "");
    assert!(session.proposal().is_some(), "an empty edit threw the turn away");

    // Nothing of the three wrote anything.
    assert_eq!(session.project().history().entries().len(), 1);
    assert!(!session.project().song().tracks.contains_key(MINTED));

    // And the proposal is still applicable afterwards, which is what "still pending" means.
    let applied = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("it applies");
    assert!(applied.valid, "{applied:?}");
}

#[test]
fn there_is_nothing_to_apply_or_edit_without_a_proposal() {
    // Both controls answer the same way, as a refusal rather than as an operator error: it is
    // a thing a caller fixes by calling when there is one (ADR 0006 §2).
    let (_dir, mut session) = opened();
    let applied = session.apply_proposal("deepseek/deepseek-v4.1-flash").expect("an answer");
    assert_eq!(applied.errors[0].rule, "no_proposal");
    let edited = session.edit_proposal(b"[]").expect("an answer");
    assert_eq!(edited.errors[0].rule, "no_proposal");
    assert!(!session.reject());
}

// ---------------------------------------------------------------------------
// A panic in the commit pipeline (M3 review, 2026-09-24)
// ---------------------------------------------------------------------------

/// An id source that panics once `remaining` mints have been served, sharing that budget with
/// every fork of itself.
///
/// The smallest injectable way to make the commit pipeline unwind: `IdSource` is a constructor
/// parameter precisely so a test can decide what it mints (ADR 0001 §5), and the budget is
/// shared because [`Session::apply_proposal`] mints from a *fork* it chooses, not from the
/// source the session was built with.
struct PanicsWhenSpent {
    inner: Box<dyn escribass_core::IdSource + Send>,
    remaining: std::sync::Arc<AtomicUsize>,
    served: std::sync::Arc<AtomicUsize>,
}

impl escribass_core::IdSource for PanicsWhenSpent {
    fn next_id(&mut self) -> String {
        if self.remaining.load(Ordering::Relaxed) == 0 {
            panic!("the id source gave up");
        }
        self.remaining.fetch_sub(1, Ordering::Relaxed);
        self.served.fetch_add(1, Ordering::Relaxed);
        self.inner.next_id()
    }

    fn fork(&self) -> Box<dyn escribass_core::IdSource + Send> {
        Box::new(PanicsWhenSpent {
            inner: self.inner.fork(),
            remaining: std::sync::Arc::clone(&self.remaining),
            served: std::sync::Arc::clone(&self.served),
        })
    }
}

#[test]
fn a_panic_while_applying_does_not_leave_the_session_recording_as_the_model() {
    // **The forgery hole ADR 0021 §1 closed, reopened by a `mem::replace` nothing unwound**
    // (M3 review, 2026-09-24). `apply_proposal` swapped `Session::made` to the model's
    // provenance, committed, and swapped it back — and a panic between the two halves left it
    // swapped for the rest of the session's life. The Tauri host *recovers* a poisoned mutex
    // rather than propagating it, which is right for one bad call and is what made this live:
    // every later commit a person made was then recorded as the model's.
    //
    // Watched failing first: with the restore written as a plain assignment after `run`, the
    // person's transpose below came back `AUTHOR_MODEL` with `deepseek/deepseek-v4.1-flash`
    // on it.
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let project =
        Project::create(&dir.0, &fixture_song(), &mut ids, &clock, Author::Human, manifest())
            .expect("the project is created");
    let remaining = std::sync::Arc::new(AtomicUsize::new(usize::MAX));
    let served = std::sync::Arc::new(AtomicUsize::new(0));
    let mut session = Session::new(
        project,
        Box::new(PanicsWhenSpent {
            inner: Box::new(ids),
            remaining: std::sync::Arc::clone(&remaining),
            served: std::sync::Arc::clone(&served),
        }),
        Box::new(clock),
        Author::Human,
    );

    session.propose("prompt-hash").expect("a proposal opens");
    proposed(&mut session, "add_track", add_lead(), "c1");

    // Two mints are `ahead`'s, one from a fork of each source; the third is the entry id
    // `Project::record` takes, which is inside the commit — so the budget runs out exactly
    // there. `served` is asserted below so that a change in `ahead` fails this loudly rather
    // than moving the panic somewhere the hole is not.
    served.store(0, Ordering::Relaxed);
    remaining.store(2, Ordering::Relaxed);
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.apply_proposal("deepseek/deepseek-v4.1-flash")
    }));
    assert!(panicked.is_err(), "the id source did not panic");
    assert_eq!(served.load(Ordering::Relaxed), 2, "the panic was not inside the commit");
    remaining.store(usize::MAX, Ordering::Relaxed);

    // The session is used again, as the host uses it after recovering the poisoned mutex.
    let applied = session
        .transpose(&TransposeRequest {
            clip_id: CLIP.to_string(),
            semitones: 1,
            note_ids: vec![],
            dry_run: false,
        })
        .expect("a person's own edit applies");
    let entry = session
        .project()
        .history()
        .get(&applied.entry_id)
        .expect("the entry")
        .clone();
    let made = entry.provenance.expect("an entry carries provenance");
    assert_eq!(made.author, Author::Human as i32, "a person's commit was recorded as the model's");
    assert_eq!(made.model_id, None, "a person's commit names a model");
    assert_eq!(made.prompt_id, None, "a person's commit names a prompt");
}
