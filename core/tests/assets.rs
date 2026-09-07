//! `add_asset` and the content-addressed `assets/` directory (§10, ADR 0011 Consequences).
//!
//! Every asset here is added through `Session`, never by writing a file: `assets/` is part of
//! the project and CLAUDE.md #2's rule about the tool API is the one that keeps it honest.

mod common;
use common::manifest;

use escribass_core::{asset_hash, new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::AddAssetRequest;
use escribass_schema::song::Author;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const AT: i64 = 1_788_307_200_000;

/// SHA-256 of `"abc"`, from FIPS 180-4 — checkable without trusting this crate.
const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
/// SHA-256 of nothing at all.
const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-assets-{}-{}.escri",
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

fn opened() -> (Scratch, Session) {
    let dir = Scratch::new();
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock, Author::Model);
    let project = Project::create(&dir.0, &song, &mut ids, &clock, Author::Model, manifest()).unwrap();
    let session = Session::new(project, Box::new(ids), Box::new(clock), Author::Model);
    (dir, session)
}

fn add(session: &Session, content: &[u8], dry_run: bool) -> String {
    session
        .add_asset(&AddAssetRequest { content: content.to_vec(), dry_run })
        .expect("a writable project")
        .asset_hash
}

fn listing(dir: &Scratch) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir.0.join("assets"))
        .expect("assets/ exists")
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn the_hash_is_sha256_lowercase_hex() {
    // The name an `asset_hash` means is decided by this function and nowhere else (ADR 0011
    // Consequences), so it is pinned against a published vector rather than against itself.
    assert_eq!(asset_hash(b"abc"), ABC);
    assert_eq!(asset_hash(b""), EMPTY);
}

#[test]
fn an_asset_is_stored_under_its_hash_with_its_bytes() {
    let (dir, session) = opened();
    assert_eq!(add(&session, b"abc", false), ABC);
    assert_eq!(std::fs::read(dir.0.join("assets").join(ABC)).unwrap(), b"abc");
    assert_eq!(listing(&dir), vec![ABC.to_string()]);
}

#[test]
fn adding_the_same_content_twice_writes_one_file() {
    // Content-addressed means the second call has nothing to do, and a caller retrying after
    // a lost answer does no harm.
    let (dir, session) = opened();
    add(&session, b"abc", false);
    add(&session, b"abc", false);
    add(&session, b"", false);
    assert_eq!(listing(&dir), vec![ABC.to_string(), EMPTY.to_string()]);
}

#[test]
fn a_dry_run_names_the_asset_and_writes_nothing() {
    // The first half of the real path (ADR 0006 §3): the hash is a function of the bytes,
    // decided before anything touches disk, so a preview and the call that follows agree.
    let (dir, session) = opened();
    assert_eq!(add(&session, b"abc", true), ABC);
    assert!(listing(&dir).is_empty(), "{:?}", listing(&dir));
    assert_eq!(add(&session, b"abc", false), ABC);
    assert_eq!(listing(&dir), vec![ABC.to_string()]);
}

#[test]
fn the_directory_is_created_when_a_checkout_lacks_it() {
    // `Project::write` creates `assets/`, but git stores no empty directory, so a project
    // cloned before its first asset arrives without one (docs/plan.md, trap 16).
    let (dir, session) = opened();
    std::fs::remove_dir(dir.0.join("assets")).expect("empty, so removable");
    assert_eq!(add(&session, b"abc", false), ABC);
    assert_eq!(listing(&dir), vec![ABC.to_string()]);
}

#[test]
fn an_asset_touches_neither_the_song_nor_the_log() {
    // Nothing in the song refers to the asset until a clip does, so there is no patch and no
    // entry: the log records changes to the document, not to the directory beside it.
    let (_dir, session) = opened();
    let before = session.get_song();
    let entries = session.project().history().entries().len();
    add(&session, b"abc", false);
    assert_eq!(session.get_song(), before);
    assert_eq!(session.project().history().entries().len(), entries);
}
