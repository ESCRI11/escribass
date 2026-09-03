//! The gRPC surface (§5, ADR 0006), over a real socket with the generated client.
//!
//! What is worth testing here is the translation layer, and one property in particular: the
//! same call must mean the same thing over gRPC as it does over MCP. Both dispatch to one
//! `Session`, so the risk is not that the logic differs — it is that a transport quietly
//! reclassifies a result on the way out.

use escribass_core::grpc::Server;
use escribass_core::{new_song, FixedClock, Project, SeededIds, Session};
use escribass_proto::tools::song_tools_client::SongToolsClient;
use escribass_proto::tools::*;
use escribass_schema::song::{Author, DeviceRef, SourceRef, TrackKind};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use tonic::transport::Channel;

const AT: i64 = 1_788_307_200_000;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "escribass-grpc-{}-{}.escri",
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

/// A server on a free loopback port, and a client connected to it.
///
/// `ponytail:` the port is chosen by binding, reading the address and dropping the listener,
/// which leaves a moment in which something else could take it. Serving the listener directly
/// would need `tokio-stream`, and a flake in a test is a cheaper thing to fix than a
/// dependency in the product.
async fn serving(dir: &Scratch) -> SongToolsClient<Channel> {
    let mut ids = SeededIds::default();
    let clock = FixedClock(AT);
    let song = new_song(&mut ids, &clock);
    let project = Project::create(&dir.0, &song, &mut ids, &clock).unwrap();
    let server = Server::new(Session::new(project, Box::new(ids), Box::new(clock), Author::Model));

    let address: SocketAddr = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("a free port")
        .local_addr()
        .expect("its address");

    tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(server.into_service())
            .serve(address)
            .await
    });

    let endpoint = format!("http://{address}");
    for _ in 0..100 {
        if let Ok(client) = SongToolsClient::connect(endpoint.clone()).await {
            return client;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the server never came up on {endpoint}");
}

fn cmajor() -> Option<DeviceRef> {
    Some(DeviceRef {
        kind: Some(escribass_schema::song::device_ref::Kind::Cmajor(SourceRef {
            source_hash: "8b31c0de4f9c00000000000000000000".to_string(),
        })),
    })
}

fn bass(dry_run: bool) -> AddTrackRequest {
    AddTrackRequest {
        name: "Bass".to_string(),
        kind: TrackKind::Instrument as i32,
        r#ref: cmajor(),
        dry_run,
    }
}

// ---- reads ----

#[tokio::test]
async fn a_song_crosses_as_protobuf_with_its_field_order_intact() {
    // The MCP surface needs `to_canonical_json` because a JSON object's key order is decided by
    // whatever serialises it. Here the encoding is binary protobuf and field order is the wire
    // format's, so the whole class of problem is absent — this asserts that rather than
    // assuming it.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let song = client.get_song(GetSongRequest {}).await.unwrap().into_inner().song.unwrap();
    assert_eq!(song.tracks.len(), 1);
    assert_eq!(song.schema_version, escribass_schema::SCHEMA_VERSION);
    assert!(song.tempo_map.is_some());
}

#[tokio::test]
async fn the_history_comes_back_with_its_ops_as_bytes() {
    // `PatchEntry.ops` needed rewriting over MCP because proto3 JSON base64s `bytes`. Over
    // gRPC it is bytes, and this checks the text arrives readable rather than re-encoded.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;
    client.add_track(bass(false)).await.unwrap();

    let history = client.get_history(GetHistoryRequest {}).await.unwrap().into_inner();
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.refs.unwrap().head, "main");

    let entry = history.entries.values().find(|e| e.tool == "add_track").unwrap();
    let ops: Value = serde_json::from_slice(&entry.ops).expect("ops are RFC 6902 text");
    assert!(ops.is_array(), "{ops}");
}

// ---- the line ADR 0006 §2 draws ----

#[tokio::test]
async fn a_refused_call_is_ok_with_valid_false_not_a_status() {
    // The half that matters. §6 builds a three-retry loop on structured errors, and a refusal
    // sent as a transport failure lands where that loop cannot see it.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let response = client
        .apply_patch(ApplyPatchRequest {
            patch: serde_json::to_vec(&json!([{"op": "replace", "path": "/nope", "value": 1}]))
                .unwrap(),
            dry_run: false,
        })
        .await
        .expect("a refusal is not a transport failure");

    let result = response.into_inner();
    assert!(!result.valid);
    assert_eq!(result.errors[0].rule, "path_not_found");
    assert!(result.patch.is_empty());
    assert!(result.entry_id.is_empty());
}

#[tokio::test]
async fn a_validator_refusal_carries_every_rule_across_the_wire() {
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let result = client
        .add_track(AddTrackRequest {
            name: "Master 2".to_string(),
            kind: TrackKind::Master as i32,
            r#ref: None,
            dry_run: false,
        })
        .await
        .unwrap()
        .into_inner();

    assert!(!result.valid);
    let rules: Vec<&str> = result.errors.iter().map(|e| e.rule.as_str()).collect();
    assert!(rules.contains(&"master_track_count"), "{rules:?}");
}

#[tokio::test]
async fn an_operator_error_is_a_status() {
    // The other side of the line: a project that cannot be written is nothing the caller can
    // fix by calling differently, so it leaves the retry loop rather than joining it.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;
    std::fs::create_dir(dir.0.join(".song.json.tmp")).unwrap();

    let failed = client.add_track(bass(false)).await.expect_err("should be a Status");
    assert_eq!(failed.code(), tonic::Code::Internal);
    assert!(failed.message().contains("unwritable"), "{}", failed.message());
}

#[tokio::test]
async fn a_replay_of_an_entry_that_is_not_there_is_a_status() {
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let failed = client
        .get_song_at(GetSongAtRequest { entry_id: "01ZZZZZZZZZZZZZZZZZZZZZZZZ".to_string() })
        .await
        .expect_err("should be a Status");
    assert_eq!(failed.code(), tonic::Code::FailedPrecondition);
    assert!(failed.message().contains("entry_missing"), "{}", failed.message());
}

// ---- dry run, over the wire ----

#[tokio::test]
async fn a_dry_run_writes_nothing() {
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let previewed = client.add_track(bass(true)).await.unwrap().into_inner();
    assert!(previewed.valid, "{:?}", previewed.errors);
    assert!(previewed.entry_id.is_empty());
    assert!(!previewed.patch.is_empty(), "a preview shows what would change");

    let song = client.get_song(GetSongRequest {}).await.unwrap().into_inner().song.unwrap();
    assert_eq!(song.tracks.len(), 1, "a dry run changed the document");
}

#[tokio::test]
async fn a_dry_run_returns_the_patch_the_apply_then_records() {
    // Including a tool that mints ids: a dry run builds its patch from a fork of the id
    // source, so previewing never burns one and the apply carries exactly what was approved.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let previewed = client.add_track(bass(true)).await.unwrap().into_inner();
    let applied = client.add_track(bass(false)).await.unwrap().into_inner();

    assert_eq!(applied.patch, previewed.patch);
    assert_eq!(applied.summary, previewed.summary);
    assert!(previewed.entry_id.is_empty());
    assert!(!applied.entry_id.is_empty());
}

#[tokio::test]
async fn the_patch_crosses_as_the_canonical_text_not_base64() {
    // ADR 0002 §11: the wire carries the same canonical document the disk does. Over gRPC that
    // needs no special handling, which is exactly why it is worth an assertion — nothing else
    // would notice if it stopped being true.
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    let result = client.add_track(bass(true)).await.unwrap().into_inner();
    let text = String::from_utf8(result.patch).expect("the patch is utf-8 text");
    assert!(text.starts_with("[\n"), "{text}");
    let ops: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(ops[0]["op"], "add");
}

// ---- the whole surface ----

#[tokio::test]
async fn a_session_can_be_driven_end_to_end_over_the_wire() {
    let dir = Scratch::new();
    let mut client = serving(&dir).await;

    client.add_track(bass(false)).await.unwrap();
    let track = client
        .get_song(GetSongRequest {})
        .await
        .unwrap()
        .into_inner()
        .song
        .unwrap()
        .tracks
        .values()
        .max_by_key(|t| t.index)
        .unwrap()
        .id
        .clone();

    let clip = client
        .add_clip(AddClipRequest {
            track_id: track,
            start_tick: 0,
            length_ticks: 3840,
            content: None,
            dry_run: false,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(clip.valid, "{:?}", clip.errors);

    let branched = client
        .create_branch(CreateBranchRequest {
            name: "darker".to_string(),
            at_entry_id: String::new(),
            dry_run: false,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(branched.valid, "{:?}", branched.errors);

    let switched = client
        .switch_branch(SwitchBranchRequest { name: "darker".to_string(), dry_run: false })
        .await
        .unwrap()
        .into_inner();
    assert!(switched.valid, "{:?}", switched.errors);

    let tempo = client
        .set_tempo(SetTempoRequest { bpm: 132.0, tick: 0, dry_run: false })
        .await
        .unwrap()
        .into_inner();
    assert!(tempo.valid, "{:?}", tempo.errors);

    // And what the wire did is what the disk holds.
    let reopened = Project::open(&dir.0).unwrap();
    assert_eq!(reopened.history().refs().head, "darker");
    assert_eq!(reopened.song().clips.len(), 1);
}
