// @generated
/// Generated client implementations.
pub mod song_tools_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /** The project is named when the process starts, not per call: an MCP stdio process is not a
 session, and two writers on one .escri would race the write ordering ADR 0004 depends on
 (ADR 0006 §5).
*/
    #[derive(Debug, Clone)]
    pub struct SongToolsClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SongToolsClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SongToolsClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SongToolsClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::Body>,
            >>::Error: Into<StdError> + std::marker::Send + std::marker::Sync,
        {
            SongToolsClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        ///
        pub async fn get_song(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSongRequest>,
        ) -> std::result::Result<tonic::Response<super::SongResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/GetSong",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "GetSong"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn get_song_at(
            &mut self,
            request: impl tonic::IntoRequest<super::GetSongAtRequest>,
        ) -> std::result::Result<tonic::Response<super::SongResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/GetSongAt",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "GetSongAt"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn get_history(
            &mut self,
            request: impl tonic::IntoRequest<super::GetHistoryRequest>,
        ) -> std::result::Result<
            tonic::Response<super::HistoryResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/GetHistory",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "GetHistory"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn apply_patch(
            &mut self,
            request: impl tonic::IntoRequest<super::ApplyPatchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/ApplyPatch",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "ApplyPatch"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_track(
            &mut self,
            request: impl tonic::IntoRequest<super::AddTrackRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddTrack",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "AddTrack"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn set_track_instrument(
            &mut self,
            request: impl tonic::IntoRequest<super::SetTrackInstrumentRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/SetTrackInstrument",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("escribass.tools.v1.SongTools", "SetTrackInstrument"),
                );
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_effect(
            &mut self,
            request: impl tonic::IntoRequest<super::AddEffectRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddEffect",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "AddEffect"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn set_param(
            &mut self,
            request: impl tonic::IntoRequest<super::SetParamRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/SetParam",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "SetParam"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_clip(
            &mut self,
            request: impl tonic::IntoRequest<super::AddClipRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddClip",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "AddClip"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn set_notes(
            &mut self,
            request: impl tonic::IntoRequest<super::SetNotesRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/SetNotes",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "SetNotes"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn transpose(
            &mut self,
            request: impl tonic::IntoRequest<super::TransposeRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/Transpose",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "Transpose"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn quantize(
            &mut self,
            request: impl tonic::IntoRequest<super::QuantizeRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/Quantize",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "Quantize"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_automation(
            &mut self,
            request: impl tonic::IntoRequest<super::AddAutomationRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddAutomation",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("escribass.tools.v1.SongTools", "AddAutomation"),
                );
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_asset(
            &mut self,
            request: impl tonic::IntoRequest<super::AddAssetRequest>,
        ) -> std::result::Result<tonic::Response<super::AssetResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddAsset",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "AddAsset"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn set_tempo(
            &mut self,
            request: impl tonic::IntoRequest<super::SetTempoRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/SetTempo",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "SetTempo"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn add_section(
            &mut self,
            request: impl tonic::IntoRequest<super::AddSectionRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/AddSection",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "AddSection"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn move_section(
            &mut self,
            request: impl tonic::IntoRequest<super::MoveSectionRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/MoveSection",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "MoveSection"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn render_export(
            &mut self,
            request: impl tonic::IntoRequest<super::RenderExportRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/RenderExport",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "RenderExport"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn create_branch(
            &mut self,
            request: impl tonic::IntoRequest<super::CreateBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/CreateBranch",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "CreateBranch"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn switch_branch(
            &mut self,
            request: impl tonic::IntoRequest<super::SwitchBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/SwitchBranch",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "SwitchBranch"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn delete_branch(
            &mut self,
            request: impl tonic::IntoRequest<super::DeleteBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/DeleteBranch",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "DeleteBranch"));
            self.inner.unary(req, path, codec).await
        }
        ///
        pub async fn merge_branch(
            &mut self,
            request: impl tonic::IntoRequest<super::MergeBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic_prost::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/escribass.tools.v1.SongTools/MergeBranch",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("escribass.tools.v1.SongTools", "MergeBranch"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod song_tools_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SongToolsServer.
    #[async_trait]
    pub trait SongTools: std::marker::Send + std::marker::Sync + 'static {
        ///
        async fn get_song(
            &self,
            request: tonic::Request<super::GetSongRequest>,
        ) -> std::result::Result<tonic::Response<super::SongResponse>, tonic::Status>;
        ///
        async fn get_song_at(
            &self,
            request: tonic::Request<super::GetSongAtRequest>,
        ) -> std::result::Result<tonic::Response<super::SongResponse>, tonic::Status>;
        ///
        async fn get_history(
            &self,
            request: tonic::Request<super::GetHistoryRequest>,
        ) -> std::result::Result<tonic::Response<super::HistoryResponse>, tonic::Status>;
        ///
        async fn apply_patch(
            &self,
            request: tonic::Request<super::ApplyPatchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_track(
            &self,
            request: tonic::Request<super::AddTrackRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn set_track_instrument(
            &self,
            request: tonic::Request<super::SetTrackInstrumentRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_effect(
            &self,
            request: tonic::Request<super::AddEffectRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn set_param(
            &self,
            request: tonic::Request<super::SetParamRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_clip(
            &self,
            request: tonic::Request<super::AddClipRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn set_notes(
            &self,
            request: tonic::Request<super::SetNotesRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn transpose(
            &self,
            request: tonic::Request<super::TransposeRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn quantize(
            &self,
            request: tonic::Request<super::QuantizeRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_automation(
            &self,
            request: tonic::Request<super::AddAutomationRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_asset(
            &self,
            request: tonic::Request<super::AddAssetRequest>,
        ) -> std::result::Result<tonic::Response<super::AssetResponse>, tonic::Status>;
        ///
        async fn set_tempo(
            &self,
            request: tonic::Request<super::SetTempoRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn add_section(
            &self,
            request: tonic::Request<super::AddSectionRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn move_section(
            &self,
            request: tonic::Request<super::MoveSectionRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn render_export(
            &self,
            request: tonic::Request<super::RenderExportRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn create_branch(
            &self,
            request: tonic::Request<super::CreateBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn switch_branch(
            &self,
            request: tonic::Request<super::SwitchBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn delete_branch(
            &self,
            request: tonic::Request<super::DeleteBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
        ///
        async fn merge_branch(
            &self,
            request: tonic::Request<super::MergeBranchRequest>,
        ) -> std::result::Result<tonic::Response<super::ToolResult>, tonic::Status>;
    }
    /** The project is named when the process starts, not per call: an MCP stdio process is not a
 session, and two writers on one .escri would race the write ordering ADR 0004 depends on
 (ADR 0006 §5).
*/
    #[derive(Debug)]
    pub struct SongToolsServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> SongToolsServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SongToolsServer<T>
    where
        T: SongTools,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/escribass.tools.v1.SongTools/GetSong" => {
                    #[allow(non_camel_case_types)]
                    struct GetSongSvc<T: SongTools>(pub Arc<T>);
                    impl<T: SongTools> tonic::server::UnaryService<super::GetSongRequest>
                    for GetSongSvc<T> {
                        type Response = super::SongResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetSongRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::get_song(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetSongSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/GetSongAt" => {
                    #[allow(non_camel_case_types)]
                    struct GetSongAtSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::GetSongAtRequest>
                    for GetSongAtSvc<T> {
                        type Response = super::SongResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetSongAtRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::get_song_at(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetSongAtSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/GetHistory" => {
                    #[allow(non_camel_case_types)]
                    struct GetHistorySvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::GetHistoryRequest>
                    for GetHistorySvc<T> {
                        type Response = super::HistoryResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetHistoryRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::get_history(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GetHistorySvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/ApplyPatch" => {
                    #[allow(non_camel_case_types)]
                    struct ApplyPatchSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::ApplyPatchRequest>
                    for ApplyPatchSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ApplyPatchRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::apply_patch(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ApplyPatchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddTrack" => {
                    #[allow(non_camel_case_types)]
                    struct AddTrackSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::AddTrackRequest>
                    for AddTrackSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddTrackRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_track(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddTrackSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/SetTrackInstrument" => {
                    #[allow(non_camel_case_types)]
                    struct SetTrackInstrumentSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::SetTrackInstrumentRequest>
                    for SetTrackInstrumentSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetTrackInstrumentRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::set_track_instrument(&inner, request)
                                    .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SetTrackInstrumentSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddEffect" => {
                    #[allow(non_camel_case_types)]
                    struct AddEffectSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::AddEffectRequest>
                    for AddEffectSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddEffectRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_effect(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddEffectSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/SetParam" => {
                    #[allow(non_camel_case_types)]
                    struct SetParamSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::SetParamRequest>
                    for SetParamSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetParamRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::set_param(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SetParamSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddClip" => {
                    #[allow(non_camel_case_types)]
                    struct AddClipSvc<T: SongTools>(pub Arc<T>);
                    impl<T: SongTools> tonic::server::UnaryService<super::AddClipRequest>
                    for AddClipSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddClipRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_clip(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddClipSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/SetNotes" => {
                    #[allow(non_camel_case_types)]
                    struct SetNotesSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::SetNotesRequest>
                    for SetNotesSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetNotesRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::set_notes(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SetNotesSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/Transpose" => {
                    #[allow(non_camel_case_types)]
                    struct TransposeSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::TransposeRequest>
                    for TransposeSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TransposeRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::transpose(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = TransposeSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/Quantize" => {
                    #[allow(non_camel_case_types)]
                    struct QuantizeSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::QuantizeRequest>
                    for QuantizeSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QuantizeRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::quantize(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = QuantizeSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddAutomation" => {
                    #[allow(non_camel_case_types)]
                    struct AddAutomationSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::AddAutomationRequest>
                    for AddAutomationSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddAutomationRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_automation(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddAutomationSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddAsset" => {
                    #[allow(non_camel_case_types)]
                    struct AddAssetSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::AddAssetRequest>
                    for AddAssetSvc<T> {
                        type Response = super::AssetResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddAssetRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_asset(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddAssetSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/SetTempo" => {
                    #[allow(non_camel_case_types)]
                    struct SetTempoSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::SetTempoRequest>
                    for SetTempoSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetTempoRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::set_tempo(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SetTempoSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/AddSection" => {
                    #[allow(non_camel_case_types)]
                    struct AddSectionSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::AddSectionRequest>
                    for AddSectionSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddSectionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::add_section(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = AddSectionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/MoveSection" => {
                    #[allow(non_camel_case_types)]
                    struct MoveSectionSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::MoveSectionRequest>
                    for MoveSectionSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MoveSectionRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::move_section(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = MoveSectionSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/RenderExport" => {
                    #[allow(non_camel_case_types)]
                    struct RenderExportSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::RenderExportRequest>
                    for RenderExportSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RenderExportRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::render_export(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = RenderExportSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/CreateBranch" => {
                    #[allow(non_camel_case_types)]
                    struct CreateBranchSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::CreateBranchRequest>
                    for CreateBranchSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CreateBranchRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::create_branch(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = CreateBranchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/SwitchBranch" => {
                    #[allow(non_camel_case_types)]
                    struct SwitchBranchSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::SwitchBranchRequest>
                    for SwitchBranchSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SwitchBranchRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::switch_branch(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SwitchBranchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/DeleteBranch" => {
                    #[allow(non_camel_case_types)]
                    struct DeleteBranchSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::DeleteBranchRequest>
                    for DeleteBranchSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DeleteBranchRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::delete_branch(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = DeleteBranchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/escribass.tools.v1.SongTools/MergeBranch" => {
                    #[allow(non_camel_case_types)]
                    struct MergeBranchSvc<T: SongTools>(pub Arc<T>);
                    impl<
                        T: SongTools,
                    > tonic::server::UnaryService<super::MergeBranchRequest>
                    for MergeBranchSvc<T> {
                        type Response = super::ToolResult;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MergeBranchRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SongTools>::merge_branch(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = MergeBranchSvc(inner);
                        let codec = tonic_prost::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        let mut response = http::Response::new(
                            tonic::body::Body::default(),
                        );
                        let headers = response.headers_mut();
                        headers
                            .insert(
                                tonic::Status::GRPC_STATUS,
                                (tonic::Code::Unimplemented as i32).into(),
                            );
                        headers
                            .insert(
                                http::header::CONTENT_TYPE,
                                tonic::metadata::GRPC_CONTENT_TYPE,
                            );
                        Ok(response)
                    })
                }
            }
        }
    }
    impl<T> Clone for SongToolsServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "escribass.tools.v1.SongTools";
    impl<T> tonic::server::NamedService for SongToolsServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
