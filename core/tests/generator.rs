//! `define_generator` and `compile_generator`: the two tools, the child between them, and
//! the seven ways a compile does not produce notes (ADR 0024 §5, §7; ADR 0026 §2; ADR 0027 §2).
//!
//! The compiler this suite drives is a **shell script beside a `Generate` server in this
//! process**, not `escribass-generative`, for `core/tests/engine.rs`'s reasons one child over.
//! What is tested here is `core`'s side of the boundary — the request it builds, the hash it
//! takes of it, the ids it mints, the provenance it stamps, the block it writes, and which
//! failures are a refusal and which are an operator's — and the unhappy paths are a line of
//! shell each, where a real compiler would have to be broken to produce them. What a real
//! compiler does with a real source is `compilers/generative/tests/` (43 tests, no `core`),
//! and what the two of them do together is `tests/determinism/generators` (both transports,
//! twice, against committed bytes).
//!
//! `#[cfg(unix)]` throughout for `engine.rs`'s reason: this repository claims Linux x86-64
//! and nothing else (ADR 0009 §1), so a shell and a Unix socket are a fair assumption where a
//! golden already is.
#![cfg(unix)]

mod common;
use common::{fake_compiler, fake_generator, hash_seed, manifest, Compiling};

use escribass_core::{
    new_song, FixedClock, Project, Sandbox, SeededIds, Session, Toolchain, Toolchains,
};
use escribass_proto::generate::{compile_response, CompileResponse, Diagnostic, Notes};
use escribass_proto::tools::{
    define_generator_request::Target, AddClipRequest, AddTrackRequest, ApplyPatchRequest,
    CompileGeneratorRequest, DefineGeneratorRequest, ToolResult,
};
use escribass_schema::song::{
    clip::Content, device_ref::Kind, Author, DeviceRef, GeneratorKind, Note, NoteClip, PluginRef,
    Song, TrackKind,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

const AT: i64 = 1_788_307_200_000;
/// The DSL version and the interpreter the fake compiler states, which is what a project
/// pinned by it records (ADR 0027 §1). The real child states `1` and `3.12.12`.
const DSL: &str = "1";
const PYTHON: &str = "3.12.12";

/// One bar of 4/4 at 960 PPQ.
const BAR: i32 = 3840;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-generator-{}-{}",
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

/// A project with one instrument track and one empty note clip a bar long.
fn opened(dir: &Scratch) -> (Session, String, String) {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project =
        Project::create(dir.at("p.escri"), &song, &mut ids, &clock, Author::Model, manifest())
            .expect("a project");
    let mut session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);

    let track = session
        .add_track(&AddTrackRequest {
            name: "Drums".to_string(),
            kind: TrackKind::Instrument as i32,
            r#ref: Some(DeviceRef {
                kind: Some(Kind::Plugin(PluginRef {
                    plugin_id: "Surge Synth Team/Surge XT".to_string(),
                    version: "1.3.4".to_string(),
                })),
            }),
            dry_run: false,
        })
        .expect("a track");
    let track = minted(&track);
    let clip = session
        .add_clip(&AddClipRequest {
            track_id: track.clone(),
            start_tick: 0,
            length_ticks: BAR,
            content: None,
            dry_run: false,
        })
        .expect("a clip");
    (session, track, minted(&clip))
}

/// The id of the one entity a tool minted, read off its summary the way a script does.
fn minted(result: &ToolResult) -> String {
    assert!(result.valid, "{:?}", result.errors);
    result
        .summary
        .split('/')
        .nth(2)
        .expect("`1 op: /<kind>/<id>` names what was minted")
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .expect("a ULID is alphanumeric")
        .to_string()
}

fn define(session: &mut Session, source: &str, target: Target) -> String {
    let defined = session
        .define_generator(&DefineGeneratorRequest {
            kind: GeneratorKind::Python as i32,
            source: source.to_string(),
            // Above 2^53 on purpose: a seed that passed through a double on the way would
            // still compile, to different notes, silently (docs/plan.md, M4 trap 10).
            seed: 9_007_199_254_740_993,
            params: BTreeMap::new(),
            target: Some(target),
            dry_run: false,
        })
        .expect("a generator");
    minted(&defined)
}

fn compile(session: &mut Session, id: &str, dry_run: bool) -> ToolResult {
    session
        .compile_generator(&CompileGeneratorRequest {
            generator_id: id.to_string(),
            dry_run,
        })
        .expect("a compile that is not an operator's problem")
}

/// Two notes, as the child returns them: §4.3 blank, ticks relative to the clip's start.
fn answered(notes: &[(i32, i32)]) -> CompileResponse {
    CompileResponse {
        dsl_version: DSL.to_string(),
        python_version: PYTHON.to_string(),
        result: Some(compile_response::Result::Notes(Notes {
            notes: notes
                .iter()
                .map(|(pitch, start)| Note {
                    pitch: *pitch,
                    start_tick: *start,
                    length_ticks: 240,
                    velocity: 100,
                    ..Note::default()
                })
                .collect(),
        })),
    }
}

/// A sandbox that answers `answer`, and the handle that says what it was asked.
fn compiler(dir: &Scratch, answer: Option<CompileResponse>) -> (Compiling, Sandbox) {
    let compiling = fake_compiler(&dir.0, answer);
    let command = fake_generator(&dir.0, &compiling.address());
    (compiling, Sandbox::new(command))
}

fn notes_of(song: &Song, clip: &str) -> NoteClip {
    match song.clips[clip].content.as_ref().expect("content") {
        Content::NoteClip(notes) => notes.clone(),
        Content::AudioClip(_) => panic!("not a note clip"),
    }
}

fn lock(session: &Session) -> serde_json::Value {
    let text = std::fs::read_to_string(session.project().root().join("lock.json"))
        .expect("a project has a lock");
    serde_json::from_str(&text).expect("lock.json is JSON")
}

fn rules(result: &ToolResult) -> Vec<&str> {
    result.errors.iter().map(|e| e.rule.as_str()).collect()
}

fn entries(session: &Session) -> usize {
    session.project().history().entries().len()
}

// ---- what a compile writes (ADR 0024 §5) ----

#[test]
fn a_compile_replaces_the_clips_notes_and_writes_the_two_fields_on_the_generator() {
    // The whole of decision 5 in one claim: one entry under `compile_generator`, the clip's
    // notes replaced whole with ids `core` minted, `compiled_hash` set to the hash of the
    // request, and `toolchain_version` set to what the child reported.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0), (38, 960)])));
    session.set_sandbox(sandbox);

    let before = entries(&session);
    let result = compile(&mut session, &id, false);
    assert!(result.valid, "{:?}", result.errors);
    assert_eq!(entries(&session), before + 1, "one entry, whatever the number of notes");
    let entry = session.project().history().get(&result.entry_id).expect("the entry").clone();
    assert_eq!(entry.tool, "compile_generator");

    let song = session.project().song();
    let written = notes_of(song, &clip);
    assert_eq!(written.notes.len(), 2);
    let pitches: Vec<i32> = written.notes.values().map(|n| n.pitch).collect();
    assert_eq!(pitches, vec![36, 38]);
    // Minted by `core`, never by the sandbox: the child answered with §4.3 blank and every
    // note here carries a ULID this session's seeded source produced (ADR 0001 §5, trap 9).
    for (key, note) in &written.notes {
        assert_eq!(&note.id, key, "a note is keyed by its own id");
        assert_eq!(note.id.len(), 26, "that is not a ULID: {}", note.id);
    }

    let generator = &song.generators[&id];
    assert_eq!(generator.toolchain_version, DSL);
    assert_eq!(generator.compiled_hash.len(), 64, "a SHA-256 in lowercase hex");
    // The hash is of the request the compile was made from, and of nothing else — so it is
    // recomputable from the document by anybody holding the same `CompileRequest`
    // (ADR 0024 §6). This is that computation, run against what the child was actually sent.
    assert_eq!(
        generator.compiled_hash,
        escribass_core::generator::compiled_hash(&compiling.request()),
    );
}

#[test]
fn a_generator_that_emits_nothing_writes_its_hash_and_no_notes() {
    // Zero notes is an answer, and a different one from no answer at all
    // (`proto/generate.proto`). The clip keeps no notes, the generator records that it was
    // compiled, and the entry exists — which is what distinguishes this from the child that
    // returned without compiling, two tests below.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "pass", Target::ClipId(clip.clone()));
    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[])));
    session.set_sandbox(sandbox);

    let result = compile(&mut session, &id, false);
    assert!(result.valid, "{:?}", result.errors);
    assert!(!result.entry_id.is_empty(), "a compile that wrote a hash recorded an entry");
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
    assert!(!session.project().song().generators[&id].compiled_hash.is_empty());
}

#[test]
fn define_generator_compiles_nothing_and_leaves_both_fields_empty() {
    // ADR 0026 §2: two calls, not one. A generator a `define` has just added has been
    // compiled by nothing, so it says nothing about a toolchain and nothing about a hash —
    // which §4.4's narrowed rule is what permits (ADR 0027 §1).
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);

    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));
    assert!(!compiling.called(), "`define_generator` spawned a compiler");
    let generator = &session.project().song().generators[&id];
    assert_eq!(generator.toolchain_version, "");
    assert_eq!(generator.compiled_hash, "");
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
}

// ---- provenance (ADR 0021 §1, ADR 0024 §5) ----

#[test]
fn a_compiled_note_carries_the_provenance_of_the_call_that_compiled_it() {
    // `song.proto` has said since M0.1 that compiled output carries "this generator's compile
    // call in their provenance", and ADR 0024 §5 says `prepare` is what decides it — on this
    // path as on every other (ADR 0021 §1). So a note the sandbox returned with §4.3 blank
    // comes out of the pipeline stamped with *this session's* author and clock, and the
    // sandbox cannot forge one: it has no field to write it in.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));
    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0), (38, 960)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);

    for note in notes_of(session.project().song(), &clip).notes.values() {
        let made = note.provenance.clone().expect("a note carries provenance");
        assert_eq!(made.author, Author::Model as i32, "this session's author, not the child's");
        assert_eq!(made.created_at.expect("a timestamp").seconds, AT / 1000);
        // The three model ids are a proposal's and this is not one (ADR 0021 §2).
        assert_eq!(made.model_id, None);
        assert_eq!(made.prompt_id, None);
        assert_eq!(made.tool_call_id, None);
    }
}

#[test]
fn a_models_compile_writes_the_generators_version_and_the_block_waits_for_a_commit() {
    // **The consequence ADR 0027 §1 named in M4 PR 5 and this pull request accepts**, pinned
    // here rather than left incidental. `compile_generator` joined `OFFERED` in M4 PR 6
    // (ADR 0026 §3), so a compile *inside a proposal* is reachable: it spawns the sandbox,
    // writes the fork's clip, and `Generator.toolchain_version` travels in the patch a person
    // applies. `lock.json`'s `toolchains` block does **not**, because `record_toolchain` fires
    // only on a compile that commits an entry, and a proposal commits nothing until a person
    // applies — at which point the one entry is `proposal`, not `compile_generator`.
    //
    // So a project whose compiles have all been a model's is unpinned until a person compiles
    // once, and the two can never disagree in the meantime: `toolchain_mismatch` compares the
    // generator's own version as well as the block, and a project with no block still has
    // that comparison (ADR 0027 §2).
    //
    // Watched failing first by dropping the `entry_id` condition from `record_toolchain`'s
    // guard: a fork is a clone of the project and shares its root, so the block lands in the
    // **real** `lock.json` and leaves a pin no entry in the log explains.
    //
    // The second half is the forgery check M3's review earned (`Session.made` not restored),
    // asked of this path: a call inside a turn must not leave the session answering as the
    // model afterwards.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0), (38, 960)])));
    session.set_sandbox(sandbox);
    assert!(lock(&session).get("toolchains").is_none(), "nothing has compiled yet");

    let entries_before = entries(&session);
    session.propose("prompt-sha").expect("a turn starts");
    let proposal = session.proposal_mut().expect("a proposal");
    let answer = proposal
        .call(
            "compile_generator",
            &serde_json::from_str(&format!("{{\"generator_id\": \"{id}\"}}"))
                .expect("arguments"),
            "a-model",
            "call-7",
        )
        .expect("`compile_generator` is offered from M4 PR 6");
    assert!(!answer.refused, "{:?}", answer.text);
    assert!(compiling.called(), "the fork inherits the sandbox (`Proposal::forked`)");

    // On the fork: the notes, the hash and the version. Not on the project, and not in the
    // log — a proposal records nothing (ADR 0019 §1).
    let forked = session.proposal().expect("still pending").song().generators[&id].clone();
    assert_eq!(forked.toolchain_version, DSL);
    assert!(!forked.compiled_hash.is_empty());
    assert_eq!(entries(&session), entries_before, "a proposal records nothing");
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
    assert!(lock(&session).get("toolchains").is_none(), "a fork's compile pinned the project");

    // Applied. One entry, under `proposal`, carrying the model's work — and **still no
    // block**, which is the accepted consequence: the version is in the patch and the block
    // is not.
    let applied = session.apply_proposal("a-model").expect("the project writes");
    assert!(applied.valid, "{:?}", applied.errors);
    let committed = &session.project().song().generators[&id];
    assert_eq!(committed.toolchain_version, DSL, "the version travelled in the patch");
    assert_eq!(committed.compiled_hash, forked.compiled_hash);
    assert_eq!(notes_of(session.project().song(), &clip).notes.len(), 2);
    assert_eq!(
        session.project().history().get(&applied.entry_id).expect("the entry").tool,
        "proposal",
        "a proposal applies as one entry under its own tool name (ADR 0019 §4)"
    );
    assert!(
        lock(&session).get("toolchains").is_none(),
        "the block is written by a compile that commits, and a proposal's did not"
    );

    // **The next committing compile writes it**, from the same answer, and the generator's own
    // version is what the mismatch check compares against in the meantime.
    let second = compile(&mut session, &id, false);
    assert!(second.valid, "{:?}", second.errors);
    assert_eq!(lock(&session)["toolchains"]["generator"]["dsl"], serde_json::json!(DSL));
    assert_eq!(lock(&session)["toolchains"]["generator"]["python"], serde_json::json!(PYTHON));

    // And the session is still a person's: the entry that compile wrote names no model.
    let entry = session.project().history().get(&second.entry_id).expect("the entry").clone();
    let made = entry.provenance.expect("an entry carries provenance");
    assert_eq!(made.model_id, None, "the session's next commit is not the model's");
}

// ---- the dry run, and the hash (ADR 0024 §6) ----

#[test]
fn a_dry_run_that_is_up_to_date_answers_without_spawning_anything() {
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);
    assert!(compiling.called());

    // A second compiler, so "was it called" is a question about *this* dry run.
    let second = Scratch::new();
    let (quiet, sandbox) = compiler(&second, Some(answered(&[(99, 0)])));
    session.set_sandbox(sandbox);
    let status = compile(&mut session, &id, true);
    assert!(status.valid);
    assert_eq!(status.summary, "up to date");
    assert!(status.patch.is_empty(), "up to date is an empty patch");
    assert!(!quiet.called(), "an up-to-date dry run spawned a compiler");
    assert_eq!(hash_seed(&second.0), None, "and so did not start the child at all");
}

#[test]
fn a_dry_run_that_is_stale_compiles_answers_with_the_diff_and_writes_nothing() {
    // The other half of ADR 0024 §6, and what the code view's *Compile* is: a dry run whose
    // inputs no longer hash to `compiled_hash` spawns the sandbox and answers with the patch
    // the commit would write — with the **same note ids**, because a dry run mints from a
    // fork that is put back (ADR 0006 §3).
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0), (38, 960)])));
    session.set_sandbox(sandbox);

    let before = entries(&session);
    let previewed = compile(&mut session, &id, true);
    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(compiling.called(), "a stale dry run compiles");
    assert!(!previewed.patch.is_empty());
    assert!(previewed.entry_id.is_empty(), "a dry run records nothing");
    assert_eq!(entries(&session), before);
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
    assert!(lock(&session).get("toolchains").is_none(), "a dry run pins nothing");

    let applied = compile(&mut session, &id, false);
    assert_eq!(
        String::from_utf8(applied.patch.clone()).expect("utf-8"),
        String::from_utf8(previewed.patch).expect("utf-8"),
        "the apply wrote a different patch from the one the dry run showed"
    );
}

#[test]
fn what_stales_a_generator_is_what_crosses_to_the_sandbox_and_nothing_else() {
    // ADR 0024 §6's one sentence, as two measurements: a tempo change crosses, so it stales a
    // generator; a track's name does not cross, so it does not. Defining the hash over the
    // request rather than over a list of fields is what makes those two facts the same fact.
    let dir = Scratch::new();
    let (mut session, track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);

    let renamed = session
        .apply_patch(&ApplyPatchRequest {
            patch: format!("[{{\"op\": \"replace\", \"path\": \"/tracks/{track}/name\", \"value\": \"Kit\"}}]")
                .into_bytes(),
            dry_run: false,
        })
        .expect("the rename applies");
    assert!(renamed.valid, "{:?}", renamed.errors);
    assert_eq!(compile(&mut session, &id, true).summary, "up to date", "a rename staled it");

    session
        .set_tempo(&escribass_proto::tools::SetTempoRequest {
            bpm: 140.0,
            tick: 960,
            dry_run: false,
        })
        .expect("the tempo change applies");
    let after = compile(&mut session, &id, true);
    assert!(!after.patch.is_empty(), "a tempo change left it fresh: {}", after.summary);
}

// ---- the refusals the caller fixes (ADR 0024 §7) ----

#[test]
fn compiling_a_generator_that_is_not_there_is_the_callers_mistake() {
    let dir = Scratch::new();
    let (mut session, _track, _clip) = opened(&dir);
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[])));
    session.set_sandbox(sandbox);
    let refused = compile(&mut session, "01M1FPMP00000000000000000X", false);
    assert!(!refused.valid);
    assert_eq!(rules(&refused), vec!["generator_unknown"]);
    assert!(!compiling.called(), "an unknown generator spawned a compiler");
}

#[test]
fn a_target_that_is_not_a_note_clip_is_valid_and_uncompilable() {
    // ADR 0024 §5: the validator accepts either target because the schema allows either, and
    // a compile refuses everything that is not a note clip — the shape ADR 0007 §6 gave
    // *valid and unrenderable*. Both arms, because a track target and an audio clip are
    // different mistakes with the same answer.
    let dir = Scratch::new();
    let (mut session, track, _clip) = opened(&dir);
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[])));
    session.set_sandbox(sandbox);

    let on_a_track = define(&mut session, "note(36, 0, 240)", Target::TrackId(track.clone()));
    let refused = compile(&mut session, &on_a_track, false);
    assert_eq!(rules(&refused), vec!["target_not_note_clip"]);
    assert_eq!(refused.errors[0].path, format!("/generators/{on_a_track}/target"));

    let asset = session
        .add_asset(&escribass_proto::tools::AddAssetRequest {
            content: b"RIFF".to_vec(),
            dry_run: false,
        })
        .expect("an asset");
    let audio = session
        .add_clip(&AddClipRequest {
            track_id: track,
            // Past the note clip: two clips on one track may not overlap unless the track
            // says so, and what this test is about is the target, not the arrangement.
            start_tick: BAR,
            length_ticks: BAR,
            content: Some(escribass_proto::tools::add_clip_request::Content::AudioClip(
                escribass_schema::song::AudioClip {
                    asset_hash: asset.asset_hash,
                    ..escribass_schema::song::AudioClip::default()
                },
            )),
            dry_run: false,
        })
        .expect("an audio clip");
    let on_audio = define(&mut session, "note(36, 0, 240)", Target::ClipId(minted(&audio)));
    assert_eq!(rules(&compile(&mut session, &on_audio, false)), vec!["target_not_note_clip"]);

    assert!(!compiling.called(), "an uncompilable target spawned a compiler");
}

#[test]
fn a_diagnostic_is_a_refusal_carrying_the_childs_own_line_and_text() {
    // ADR 0026 §3: the one refusal in this system whose message is the whole fix, fed back
    // whole. `core` adds the rule and the path and rewrites nothing, because the same text
    // the user reads is what the model retries against (Plate 3).
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "import math", Target::ClipId(clip.clone()));
    let said = "`Import` is outside the generator DSL (ADR 0024 §3)";
    let (_compiling, sandbox) = compiler(
        &dir,
        Some(CompileResponse {
            dsl_version: DSL.to_string(),
            python_version: PYTHON.to_string(),
            result: Some(compile_response::Result::Diagnostic(Diagnostic {
                line: 1,
                column: 1,
                message: said.to_string(),
            })),
        }),
    );
    session.set_sandbox(sandbox);

    let before = entries(&session);
    let refused = compile(&mut session, &id, false);
    assert!(!refused.valid);
    assert_eq!(rules(&refused), vec!["generator_error"]);
    assert_eq!(refused.errors[0].path, format!("/generators/{id}/source"));
    assert_eq!(refused.errors[0].message, format!("1:1: {said}"));
    // Nothing at all: no notes, no hash, no entry, no block.
    assert_eq!(entries(&session), before);
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
    assert_eq!(session.project().song().generators[&id].compiled_hash, "");
    assert!(lock(&session).get("toolchains").is_none());
}

#[test]
fn a_diagnostic_with_no_column_names_the_line_alone() {
    // `proto/generate.proto`: 0 means the child has no position for it — an exception raised
    // at run time has a line and no column — and a caller renders the line rather than
    // pointing at character zero.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 0 // 0)", Target::ClipId(clip));
    let (_compiling, sandbox) = compiler(
        &dir,
        Some(CompileResponse {
            dsl_version: DSL.to_string(),
            python_version: PYTHON.to_string(),
            result: Some(compile_response::Result::Diagnostic(Diagnostic {
                line: 1,
                column: 0,
                message: "ZeroDivisionError: integer division or modulo by zero".to_string(),
            })),
        }),
    );
    session.set_sandbox(sandbox);
    let refused = compile(&mut session, &id, false);
    assert!(refused.errors[0].message.starts_with("1: ZeroDivisionError"),
        "{}", refused.errors[0].message);
}

#[test]
fn a_sandbox_that_never_answers_ends_the_turn_rather_than_spending_a_models_retries() {
    // M0.4's rule — a hang is not a failure unless one is imposed — and **ADR 0024 §7,
    // amended a second time on 2026-10-01, in M4 PR 6**. Until then `generator_timeout` was
    // caller-fixable, on the reasoning that what loops for ever is the source. Offering
    // `compile_generator` to a model is what showed that wrong, and this test's own name was
    // the evidence: it drives *a sandbox that never answers*, because no source can reach
    // this bound. The child catches its own CPU and memory limits well inside it and answers
    // a `Diagnostic` with the line (2.04 s, 6.05 s and 0.28 s, measured in M4 PR 4), and
    // `ANSWERING` is a minute precisely so that it sits above all of them. What arrives here
    // has no line, no column and nothing an author could edit — so feeding it to a model and
    // charging it one of three refusals is a retry budget spent on a wall (M3 trap 2).
    //
    // Watched failing first by returning the call's own `Err` arm instead of the timeout:
    // with no bound on it this test does not fail, it **hangs**, which is the shape of the
    // defect rather than a different one.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "while True:\n    pass", Target::ClipId(clip.clone()));
    let (compiling, mut sandbox) = compiler(&dir, None);
    // Shrunk from the minute a compile is really given, for the reason the field exists: a
    // bound nothing has ever been seen to fire is a bound nobody should believe.
    sandbox.answering = Duration::from_millis(400);
    session.set_sandbox(sandbox);

    let before = entries(&session);
    let broken = session
        .compile_generator(&CompileGeneratorRequest { generator_id: id.clone(), dry_run: false })
        .expect_err("a child that never answers is the operator's");
    assert_eq!(broken.rule, "generator_timeout");
    // Not `generator_failed`: the two are different things to go and look at, and the id is
    // the whole of that distinction (`core/src/generator.rs`, `verdict`).
    assert!(broken.message.contains("did not answer within"), "{}", broken.message);
    assert!(compiling.called(), "the call did reach the sandbox");
    assert_eq!(entries(&session), before);
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
}

#[test]
fn the_cpu_limit_arrives_as_a_diagnostic_and_so_as_generator_error() {
    // ADR 0024 §4 and §7, amended in M4 PR 4: the wire has one refusal arm, the child catches
    // its own `SIGXCPU` and answers an ordinary diagnostic with the line the generator was
    // on, and `core` names that `generator_error`. The two refusals are genuinely different
    // things — one carries a line a person can go to, the other says only that nothing came
    // back — and nothing in `core` may collapse them.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "while True:\n    pass", Target::ClipId(clip));
    let (_compiling, sandbox) = compiler(
        &dir,
        Some(CompileResponse {
            dsl_version: DSL.to_string(),
            python_version: PYTHON.to_string(),
            result: Some(compile_response::Result::Diagnostic(Diagnostic {
                line: 2,
                column: 0,
                message: "the generator reached the compiler's CPU limit of 1 second \
                          (ADR 0024 §4)"
                    .to_string(),
            })),
        }),
    );
    session.set_sandbox(sandbox);

    let refused = compile(&mut session, &id, false);
    assert_eq!(rules(&refused), vec!["generator_error"], "the CPU limit is not the wall clock");
    assert!(refused.errors[0].message.contains("CPU limit"), "{}", refused.errors[0].message);
    assert!(refused.errors[0].message.starts_with("2: "), "it names the line it was on");
}

// ---- the operator errors (ADR 0024 §7, ADR 0027 §2) ----

#[test]
fn a_session_told_no_generator_says_so_rather_than_skipping_the_compile() {
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let broken = session
        .compile_generator(&CompileGeneratorRequest { generator_id: id, dry_run: false })
        .expect_err("no compiler was named");
    assert_eq!(broken.rule, "generator_missing");
    assert!(broken.message.contains("--generator"), "{}", broken.message);
}

#[test]
fn a_child_that_will_not_start_a_child_that_names_no_socket_and_one_that_never_serves() {
    // The three shapes ADR 0024 §7 puts on the operator's side, each an `Err` and each
    // carrying what the child did. The third is the one M1 met: a process that exits **0**
    // having written nothing, which is a compile that never happened — and the return type is
    // what makes it impossible to report as a success, whatever the exit code was.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));

    let mut broken = |command: Vec<String>, expect: &str| {
        session.set_sandbox(Sandbox::new(command));
        let e = session
            .compile_generator(&CompileGeneratorRequest {
                generator_id: id.clone(),
                dry_run: false,
            })
            .expect_err("this child cannot answer");
        assert_eq!(e.rule, "generator_failed", "{e:?}");
        assert!(e.message.contains(expect), "{}: {}", expect, e.message);
    };

    broken(vec![dir.at("no-such-compiler").display().to_string()], "cannot start the generator");
    // Exits 0 saying nothing. `listening` reads end of stream rather than a line.
    broken(fake_generator(&dir.0, "exit 0"), "closed its stdout without naming a socket");
    // Names a socket nothing is listening on, and leaves. "Exited 0 having written nothing" is
    // reported as a failure rather than as zero notes.
    broken(
        fake_generator(&dir.0, &format!("echo unix:{}/nothing.sock", dir.0.display())),
        "did not answer the call",
    );
    // Exits non-zero, and its stderr travels with the report (ADR 0013 §3).
    broken(
        fake_generator(&dir.0, "echo 'no interpreter' >&2\nexit 3"),
        "no interpreter",
    );

    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
}

#[test]
fn a_child_that_answers_without_answering_is_a_failure_and_not_zero_notes() {
    // `proto/generate.proto`, and M1's `RenderResult::decode(&[]) == Ok` one boundary over.
    // proto3 has no required fields, so both of these are well-formed messages — and both
    // mean the child returned without compiling.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip.clone()));

    for (answer, expect) in [
        (CompileResponse::default(), "neither notes nor a diagnostic"),
        (
            CompileResponse {
                dsl_version: String::new(),
                python_version: PYTHON.to_string(),
                result: Some(compile_response::Result::Notes(Notes { notes: vec![] })),
            },
            "without stating its dsl_version",
        ),
    ] {
        let scratch = Scratch::new();
        let (_compiling, sandbox) = compiler(&scratch, Some(answer));
        session.set_sandbox(sandbox);
        let e = session
            .compile_generator(&CompileGeneratorRequest {
                generator_id: id.clone(),
                dry_run: false,
            })
            .expect_err("that is not an answer");
        assert_eq!(e.rule, "generator_failed");
        assert!(e.message.contains(expect), "{}: {}", expect, e.message);
    }
    assert!(notes_of(session.project().song(), &clip).notes.is_empty());
    assert!(lock(&session).get("toolchains").is_none());
}

#[test]
fn a_toolchain_the_project_did_not_record_refuses_the_compile_and_writes_nothing() {
    // ADR 0027 §2, and trap 5's guard: a `uv sync` that was not run is a child stating a
    // version the project did not record, and the compile is refused rather than a golden
    // passing on a stale sandbox. Three comparisons, because there are three ways to
    // disagree, and each must write nothing.
    let answers = |dsl: &str, python: &str| CompileResponse {
        dsl_version: dsl.to_string(),
        python_version: python.to_string(),
        result: Some(compile_response::Result::Notes(Notes {
            notes: vec![Note { pitch: 60, length_ticks: 240, velocity: 100, ..Note::default() }],
        })),
    };

    for (dsl, python, expect) in
        [("2", PYTHON, "DSL at version `1`"), (DSL, "3.13.1", "Python `3.12.12`")]
    {
        let dir = Scratch::new();
        let (mut session, _track, clip) = opened(&dir);
        let id = define(&mut session, "note(60, 0, 240)", Target::ClipId(clip.clone()));
        let (_first, sandbox) = compiler(&dir, Some(answers(DSL, PYTHON)));
        session.set_sandbox(sandbox);
        compile(&mut session, &id, false);
        assert_eq!(lock(&session)["toolchains"]["generator"]["dsl"], serde_json::json!(DSL));

        let second = Scratch::new();
        let (_moved, sandbox) = compiler(&second, Some(answers(dsl, python)));
        session.set_sandbox(sandbox);
        let before = entries(&session);
        let e = session
            .compile_generator(&CompileGeneratorRequest {
                generator_id: id.clone(),
                dry_run: false,
            })
            .expect_err("the toolchain moved under the project");
        assert_eq!(e.rule, "toolchain_mismatch");
        assert!(e.message.contains(expect), "{}: {}", expect, e.message);
        assert!(e.message.contains("uv sync --locked"), "{}", e.message);
        assert_eq!(entries(&session), before, "a mismatch wrote an entry");
        // Not the notes, not the hash, and not the block: it still says what it said.
        assert_eq!(lock(&session)["toolchains"]["generator"]["dsl"], serde_json::json!(DSL));
        assert_eq!(lock(&session)["toolchains"]["generator"]["python"], serde_json::json!(PYTHON));
    }
}

#[test]
fn a_generator_compiled_before_refuses_a_compiler_that_is_not_the_one_that_compiled_it() {
    // The second of ADR 0027 §2's two comparisons, shown on its own: a project with no block
    // at all — one whose compiles have all been a model's, inside a turn — still has the
    // generator's own `toolchain_version` to compare against.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(60, 0, 240)", Target::ClipId(clip));
    session
        .apply_patch(&ApplyPatchRequest {
            patch: format!(
                "[{{\"op\": \"replace\", \"path\": \"/generators/{id}/toolchain_version\", \
                  \"value\": \"0.4.1\"}}, {{\"op\": \"replace\", \"path\": \
                  \"/generators/{id}/compiled_hash\", \"value\": \"{}\"}}]",
                "a".repeat(64)
            )
            .into_bytes(),
            dry_run: false,
        })
        .expect("a generator compiled by something else");
    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[(60, 0)])));
    session.set_sandbox(sandbox);

    assert!(lock(&session).get("toolchains").is_none(), "the project pins nothing yet");
    let e = session
        .compile_generator(&CompileGeneratorRequest { generator_id: id, dry_run: false })
        .expect_err("this generator was compiled by a different DSL");
    assert_eq!(e.rule, "toolchain_mismatch");
    assert!(e.message.contains("0.4.1"), "{}", e.message);
}

// ---- lock.json's block (ADR 0027 §1) ----

#[test]
fn the_toolchains_block_is_written_on_the_first_compile_and_never_rewritten_by_a_tool() {
    // ADR 0027 §1, in the three states that matter: absent before any compile — which is what
    // keeps every M0.4 golden's `lock.json` byte-identical through a milestone that adds a
    // block — written from what the child reported on the first compile that commits, and
    // kept thereafter, because re-pinning is editing the text and never running a tool
    // (ADR 0010 §2).
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    assert!(lock(&session).get("toolchains").is_none(), "a project pins nothing until it compiles");

    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);
    assert_eq!(
        lock(&session)["toolchains"],
        serde_json::json!({"generator": {"dsl": DSL, "python": PYTHON}}),
    );

    // A person edits it to something the compiler does not state, and a later compile refuses
    // rather than putting it back: a tool that rewrote this would be re-pinning by running.
    let mut pinned = session.project().clone();
    pinned.record_toolchain("2", "3.13.1").expect("the lock is writable");
    assert!(
        pinned.toolchains().is_some_and(|t| t.generator.dsl == DSL),
        "`record_toolchain` overwrote a pin the project already had"
    );

    // And it survives an ordinary write of the whole project, which is what every later
    // commit does.
    session
        .add_track(&AddTrackRequest {
            name: "Another".to_string(),
            kind: TrackKind::Audio as i32,
            r#ref: None,
            dry_run: false,
        })
        .expect("a track");
    assert_eq!(lock(&session)["toolchains"]["generator"]["python"], serde_json::json!(PYTHON));

    // And a fresh process reads it back: `Project::open` carries the block and compares
    // nothing in it (ADR 0027 §2), so a project with a generator opens with no compiler.
    let reopened = Project::open(session.project().root(), manifest()).expect("it reopens");
    assert_eq!(
        reopened.toolchains().expect("the block").clone(),
        Toolchains { generator: Toolchain { dsl: DSL.to_string(), python: PYTHON.to_string() } },
    );
}

// ---- what crosses (ADR 0026 §1) ----

#[test]
fn the_sandbox_is_handed_what_it_compiles_and_nothing_about_the_project() {
    // ADR 0026 §1 field by field, read off what the generated server decoded rather than off
    // what `core` believed it sent. The two halves of the claim are what is here — the
    // source, the seed as an integer, the maps in tick order, the clip's bounds — and what is
    // not: no `Song`, no clip id, no track, no `toolchain_version`, and §4.3 blank on every
    // leaf, which is what keeps a tempo event's key out of `compiled_hash`.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    session
        .add_section(&escribass_proto::tools::AddSectionRequest {
            name: "B".to_string(),
            start_tick: BAR,
            end_tick: BAR * 2,
            dry_run: false,
        })
        .expect("a section");
    session
        .add_section(&escribass_proto::tools::AddSectionRequest {
            name: "A".to_string(),
            start_tick: 0,
            end_tick: BAR,
            dry_run: false,
        })
        .expect("a section");
    session
        .set_tempo(&escribass_proto::tools::SetTempoRequest {
            bpm: 90.0,
            tick: 1920,
            dry_run: false,
        })
        .expect("a tempo change");

    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let (compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);

    let sent = compiling.request();
    assert_eq!(sent.kind, GeneratorKind::Python as i32);
    assert_eq!(sent.source, "note(36, 0, 240)");
    assert_eq!(sent.seed, 9_007_199_254_740_993, "the integer, whole");
    assert_eq!(sent.clip_start_tick, 0);
    assert_eq!(sent.clip_length_ticks, BAR);
    // Tick order, and §4.3 blank: an id the child cannot use would be an input to the hash
    // that no compile reads.
    assert_eq!(sent.tempo.iter().map(|e| e.tick).collect::<Vec<_>>(), vec![0, 1920]);
    assert!(sent.tempo.iter().all(|e| e.id.is_empty()));
    assert_eq!(sent.signature.iter().map(|e| e.tick).collect::<Vec<_>>(), vec![0]);
    assert!(sent.signature.iter().all(|e| e.id.is_empty()));
    assert_eq!(
        sent.sections.iter().map(|s| (s.start_tick, s.name.as_str())).collect::<Vec<_>>(),
        vec![(0, "A"), (BAR, "B")],
    );
    assert!(sent.sections.iter().all(|s| s.id.is_empty() && s.provenance.is_none()));
}

#[test]
fn the_child_is_started_with_the_hash_seed_it_would_otherwise_re_exec_for() {
    // ADR 0024 §4, amended in M4 PR 4: the child owns the lock and re-executes itself when the
    // variable is missing, so what `core` setting it buys is the interpreter start that
    // re-exec would cost on every compile. Read back out of the child's own environment,
    // because "we meant to" is not a measurement.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let (_compiling, sandbox) = compiler(&dir, Some(answered(&[(36, 0)])));
    session.set_sandbox(sandbox);
    compile(&mut session, &id, false);
    assert_eq!(hash_seed(&dir.0).as_deref(), Some("0"));
}

#[test]
fn a_fresh_process_per_compile() {
    // ADR 0024 §1: never a warm interpreter. Two compiles, two processes — shown by the child
    // counting its own starts, since nothing in the answer could tell the two apart.
    let dir = Scratch::new();
    let (mut session, _track, clip) = opened(&dir);
    let id = define(&mut session, "note(36, 0, 240)", Target::ClipId(clip));
    let compiling = fake_compiler(&dir.0, Some(answered(&[(36, 0)])));
    let counter = dir.at("starts");
    let command = fake_generator(
        &dir.0,
        &format!("echo x >> '{}'\n{}", counter.display(), compiling.address()),
    );
    session.set_sandbox(Sandbox::new(command));

    compile(&mut session, &id, true);
    compile(&mut session, &id, true);
    assert_eq!(count(&counter), 2, "two compiles shared one process");
}

fn count(path: &Path) -> usize {
    std::fs::read_to_string(path).map(|text| text.lines().count()).unwrap_or(0)
}
