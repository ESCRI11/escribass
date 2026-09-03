//! The gRPC surface (§5, ADR 0006).
//!
//! The second transport, and like the first it translates and decides nothing: every call goes
//! to the same [`Session`] the MCP server uses, so the two cannot answer one question
//! differently. What differs is only the shape of the answer.
//!
//! Two things are simpler here than over MCP, and both are worth naming because they are the
//! places MCP needed care:
//!
//! - **`patch` needs no rewriting.** The encoding is binary protobuf, so `bytes` is `bytes`;
//!   the base64 that proto3 *JSON* imposes never arises.
//! - **A song needs no canonical rendering.** Field order is the wire format's, not a JSON
//!   object's, so `to_value` cannot alphabetise anything on the way out.
//!
//! What is the same is ADR 0006 §2's line: a refused call is `OK` carrying `valid = false`,
//! because §6's retry loop has to be able to see it. Only an operator's problem becomes a
//! `Status`.
//!
//! Every RPC in `song_tools.proto` is implemented here. The tools §5 lists that are missing
//! are missing from the *contract* too, named in a comment there with the milestone each waits
//! for — so there is no stub answering `UNIMPLEMENTED` and no way to call one by accident.

use crate::session::Session;
use crate::ProjectError;
use escribass_proto::tools::song_tools_server::{SongTools, SongToolsServer};
use escribass_proto::tools::*;
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status};

/// One open project, served over gRPC.
///
/// The `Mutex` is the whole concurrency design, as it is for MCP: one project, one writer
/// (ADR 0001 §2), and calls short enough that queueing them costs nothing worth engineering
/// around.
pub struct Server {
    session: Arc<Mutex<Session>>,
}

impl Server {
    pub fn new(session: Session) -> Self {
        Self { session: Arc::new(Mutex::new(session)) }
    }

    /// Wraps this in the generated service, ready to hand to `tonic`.
    pub fn into_service(self) -> SongToolsServer<Self> {
        SongToolsServer::new(self)
    }

    /// The session, recovered if a previous call panicked while holding it.
    ///
    /// Propagating the poison would let one bad request take the process down permanently:
    /// every later call would panic on the guard. Nothing here can leave a session
    /// half-mutated — tools and `prepare` are pure, and `record` adopts its new state only
    /// after the write succeeded — so recovering is safe as well as necessary.
    fn locked(&self) -> std::sync::MutexGuard<'_, Session> {
        self.session.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// An operator's problem as a `Status`.
///
/// `FAILED_PRECONDITION` rather than `INTERNAL` for everything a project can be in the wrong
/// state for: gRPC's own guidance is that it means "retrying will not help until the system
/// state changes", which is exactly what `song_diverged` or an unwritable directory is.
fn status(e: ProjectError) -> Status {
    let message = format!("{} [{}]: {}", e.path, e.rule, e.message);
    match e.rule {
        "unreadable" | "unwritable" => Status::internal(message),
        _ => Status::failed_precondition(message),
    }
}

/// The whole service, so the seventeen mutating RPCs are written once rather than seventeen
/// times.
///
/// The macro wraps the entire `impl` rather than generating one method at a time, because
/// `#[tonic::async_trait]` rewrites `async fn` into a boxed future and attribute macros run
/// before `macro_rules!` expands — a per-method macro produces methods the attribute never
/// sees, and the lifetimes stop matching the trait.
///
/// Every one of them returns `Ok` even when the call was refused. That is the half that
/// matters: an invalid tool call is a *result*, and sending it as a transport error would put
/// it where §6's retry loop cannot see it (ADR 0006 §2).
macro_rules! service {
    ($($tool:ident: $request:ty,)*) => {
        #[tonic::async_trait]
        impl SongTools for Server {
            async fn get_song(
                &self,
                _request: Request<GetSongRequest>,
            ) -> Result<Response<SongResponse>, Status> {
                Ok(Response::new(self.locked().get_song()))
            }

            async fn get_song_at(
                &self,
                request: Request<GetSongAtRequest>,
            ) -> Result<Response<SongResponse>, Status> {
                let song = self.locked().get_song_at(&request.into_inner()).map_err(status)?;
                Ok(Response::new(song))
            }

            async fn get_history(
                &self,
                _request: Request<GetHistoryRequest>,
            ) -> Result<Response<HistoryResponse>, Status> {
                Ok(Response::new(self.locked().get_history()))
            }

            $(
                async fn $tool(
                    &self,
                    request: Request<$request>,
                ) -> Result<Response<ToolResult>, Status> {
                    let result =
                        self.locked().$tool(&request.into_inner()).map_err(status)?;
                    Ok(Response::new(result))
                }
            )*
        }
    };
}

service! {
    apply_patch: ApplyPatchRequest,
    add_track: AddTrackRequest,
    set_track_instrument: SetTrackInstrumentRequest,
    add_effect: AddEffectRequest,
    set_param: SetParamRequest,
    add_clip: AddClipRequest,
    set_notes: SetNotesRequest,
    transpose: TransposeRequest,
    quantize: QuantizeRequest,
    add_automation: AddAutomationRequest,
    set_tempo: SetTempoRequest,
    add_section: AddSectionRequest,
    move_section: MoveSectionRequest,
    create_branch: CreateBranchRequest,
    switch_branch: SwitchBranchRequest,
    delete_branch: DeleteBranchRequest,
    merge_branch: MergeBranchRequest,
}
