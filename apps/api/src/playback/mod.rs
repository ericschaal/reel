//! Playback source resolution and activation. Provider negotiation, delivery,
//! and ephemeral state live behind private modules.
mod capabilities;
mod error;
mod ids;
mod jellyfin;
mod proxy;
mod session;
mod store;
mod types;
mod values;
use crate::{
    aiostreams::{AioStreams, DirectStream, SearchOutcome},
    jellyfin::{Jellyfin, JellyfinUserId},
};
use axum::{
    Router,
    routing::{get, post},
};
use capabilities::{Container, direct_content_type, normalize_container, supports_direct_stream};
pub use error::{Error, PlaybackUpstream};
use ids::{CandidateId, DiscoveryId, SessionId};
use proxy::{activate, discover, input, media, resource};
use session::{DiscoveredCandidate, Discovery, PlaybackSession, SessionDelivery, SessionSource};
use std::{sync::Arc, time::Duration};
use store::{ResourceRegistry, Store};
use tokio::sync::RwLock;
use types::{
    ActivationRequest, Delivery, DiscoveryRequest, DiscoveryResponse, PlaybackCandidate,
    PlaybackDescriptor, PlaybackIssue, PlaybackIssueCode, PlaybackSelection, PlaybackSource,
    PlaybackTarget, PlayerCapabilities,
};

const SESSION_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const DISCOVERY_TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone)]
pub struct Playback {
    jellyfin: Jellyfin,
    aiostreams: Option<AioStreams>,
    stremio: Option<crate::stremio::Stremio>,
    username: Arc<str>,
    user_id: Arc<RwLock<Option<Arc<JellyfinUserId>>>>,
    sessions: Arc<Store<SessionId, PlaybackSession>>,
    discoveries: Arc<Store<DiscoveryId, Discovery>>,
}

impl Playback {
    #[must_use]
    pub fn new(jellyfin: Jellyfin, username: impl Into<String>) -> Self {
        Self {
            jellyfin,
            aiostreams: None,
            stremio: None,
            username: Arc::from(username.into()),
            user_id: Arc::new(RwLock::new(None)),
            sessions: Arc::new(Store::new(SESSION_TTL, 1024)),
            discoveries: Arc::new(Store::new(DISCOVERY_TTL, 1024)),
        }
    }

    #[must_use]
    pub fn with_aiostreams(mut self, aiostreams: AioStreams) -> Self {
        self.aiostreams = Some(aiostreams);
        self
    }

    #[must_use]
    pub fn with_stremio(mut self, stremio: crate::stremio::Stremio) -> Self {
        self.stremio = Some(stremio);
        self
    }

    #[must_use]
    pub fn new_with_user_id(jellyfin: Jellyfin, user_id: impl Into<JellyfinUserId>) -> Self {
        let mut playback = Self::new(jellyfin, "");
        playback.user_id = Arc::new(RwLock::new(Some(Arc::new(user_id.into()))));
        playback
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/playback/sources", post(discover))
            .route("/v1/playback/activate", post(activate))
            .route("/v1/playback/sessions/{session_id}/media", get(media))
            .route("/v1/playback/sessions/{session_id}/input", get(input))
            .route(
                "/v1/playback/sessions/{session_id}/resources/{resource}",
                get(resource),
            )
            .with_state(self)
    }

    async fn activate(&self, request: ActivationRequest) -> Result<PlaybackDescriptor, Error> {
        match &request.selection {
            PlaybackSelection::Auto { discovery_id } => {
                match self.activate_jellyfin(&request).await {
                    Ok(descriptor) => Ok(descriptor),
                    Err(Error::NotLocal | Error::NoCompatibleSource) => {
                        if let Some(discovery_id) = discovery_id {
                            self.activate_first_discovered_aiostreams(
                                &request.target,
                                discovery_id,
                                &request.capabilities,
                            )
                            .await
                        } else {
                            self.activate_first_aiostreams(&request.target, &request.capabilities)
                                .await
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            PlaybackSelection::Jellyfin => self.activate_jellyfin(&request).await,
            PlaybackSelection::AioStreams {
                discovery_id,
                candidate_id,
            } => {
                self.activate_discovered_aiostreams(
                    &request.target,
                    discovery_id,
                    candidate_id,
                    &request.capabilities,
                )
                .await
            }
        }
    }

    async fn discover(&self, request: DiscoveryRequest) -> Result<DiscoveryResponse, Error> {
        let local = self.resolve_item(&request.target);
        let remote = self.search_aiostreams(&request.target);
        let (local, remote) = tokio::join!(local, remote);
        let mut sources = Vec::new();
        let mut issues = Vec::new();
        match local {
            Ok(_) => sources.push(PlaybackCandidate {
                id: "jellyfin".into(),
                source: PlaybackSource::Jellyfin,
                label: "Local copy".into(),
                description: Some("Jellyfin".into()),
                preferred: true,
                addon: None,
                service: None,
                cached: None,
                resolution: None,
                quality: None,
                container: None,
                video_codec: None,
                size_bytes: None,
                web_ready: true,
            }),
            Err(Error::NotLocal) => {}
            Err(error) => {
                error.log_failure("local_discovery", None);
                issues.push(PlaybackIssue {
                    source: PlaybackSource::Jellyfin,
                    code: PlaybackIssueCode::UpstreamUnavailable,
                });
            }
        }

        let mut stored_candidates = Vec::new();
        let upstream_error_count;
        match remote {
            Ok(SearchOutcome {
                streams,
                upstream_error_count: error_count,
            }) => {
                upstream_error_count = error_count;
                if error_count > 0 {
                    issues.push(PlaybackIssue {
                        source: PlaybackSource::AioStreams,
                        code: if streams.is_empty() {
                            PlaybackIssueCode::UpstreamUnavailable
                        } else {
                            PlaybackIssueCode::PartialResults
                        },
                    });
                }
                for stream in streams {
                    let id = CandidateId::generate();
                    sources.push(PlaybackCandidate {
                        id: id.to_string(),
                        source: PlaybackSource::AioStreams,
                        label: stream.label.clone(),
                        description: stream.description.clone(),
                        preferred: false,
                        addon: stream.addon.clone(),
                        service: stream.service.clone(),
                        cached: stream.cached,
                        resolution: stream.resolution.clone(),
                        quality: stream.quality.clone(),
                        container: stream.container.clone(),
                        video_codec: stream.video_codec.clone(),
                        size_bytes: stream.size_bytes,
                        web_ready: self.can_deliver(&stream, &request.capabilities),
                    });
                    stored_candidates.push(DiscoveredCandidate { id, stream });
                }
            }
            Err(error) => {
                error.log_failure("remote_discovery", None);
                upstream_error_count = 1;
                issues.push(PlaybackIssue {
                    source: PlaybackSource::AioStreams,
                    code: PlaybackIssueCode::UpstreamUnavailable,
                });
            }
        }

        let discovery_id = DiscoveryId::generate();
        self.discoveries
            .insert(
                discovery_id.clone(),
                Discovery {
                    target: request.target,
                    candidates: stored_candidates,
                    upstream_error_count,
                },
            )
            .await?;
        Ok(DiscoveryResponse {
            discovery_id,
            sources,
            issues,
        })
    }

    async fn search_aiostreams(&self, target: &PlaybackTarget) -> Result<SearchOutcome, Error> {
        let aiostreams = self
            .aiostreams
            .as_ref()
            .ok_or(Error::AioStreamsNotConfigured)?;
        aiostreams
            .search(target.aiostreams_target())
            .await
            .map_err(Into::into)
    }

    fn can_deliver(&self, stream: &DirectStream, capabilities: &PlayerCapabilities) -> bool {
        if self.stremio.is_some() {
            capabilities.hls
        } else {
            stream.web_ready && supports_direct_stream(stream, capabilities)
        }
    }

    async fn activate_first_aiostreams(
        &self,
        target: &PlaybackTarget,
        capabilities: &PlayerCapabilities,
    ) -> Result<PlaybackDescriptor, Error> {
        let outcome = self.search_aiostreams(target).await?;
        self.activate_candidates(
            outcome.streams.iter(),
            outcome.upstream_error_count,
            capabilities,
        )
        .await
    }

    async fn activate_first_discovered_aiostreams(
        &self,
        target: &PlaybackTarget,
        discovery_id: &DiscoveryId,
        capabilities: &PlayerCapabilities,
    ) -> Result<PlaybackDescriptor, Error> {
        let discovery = self.discovery(target, discovery_id).await?;
        self.activate_candidates(
            discovery
                .candidates
                .iter()
                .map(|candidate| &candidate.stream),
            discovery.upstream_error_count,
            capabilities,
        )
        .await
    }

    async fn activate_candidates<'a>(
        &self,
        candidates: impl ExactSizeIterator<Item = &'a DirectStream>,
        upstream_error_count: usize,
        capabilities: &PlayerCapabilities,
    ) -> Result<PlaybackDescriptor, Error> {
        let no_sources = no_remote_sources_error(candidates.len(), upstream_error_count);
        let mut last_failure: Option<(usize, Error)> = None;
        for (index, stream) in candidates.enumerate() {
            if !self.can_deliver(stream, capabilities) {
                continue;
            }
            // A previous failure is handled here by trying another source.
            // The final failure propagates to the HTTP boundary, which logs it once.
            if let Some((previous_index, error)) = last_failure.take() {
                error.log_failure("candidate_activation", Some(previous_index));
            }
            match self.activate_aiostreams(stream, capabilities).await {
                Ok(descriptor) => return Ok(descriptor),
                Err(Error::CapacityExceeded) => return Err(Error::CapacityExceeded),
                Err(error) => last_failure = Some((index, error)),
            }
        }
        Err(last_failure.map_or(no_sources, |(_, error)| error))
    }

    async fn activate_discovered_aiostreams(
        &self,
        target: &PlaybackTarget,
        discovery_id: &DiscoveryId,
        candidate_id: &CandidateId,
        capabilities: &PlayerCapabilities,
    ) -> Result<PlaybackDescriptor, Error> {
        let discovery = self.discovery(target, discovery_id).await?;
        let candidate = discovery
            .candidates
            .iter()
            .find(|candidate| &candidate.id == candidate_id)
            .ok_or(Error::CandidateNotFound)?;
        self.activate_aiostreams(&candidate.stream, capabilities)
            .await
    }

    async fn discovery(
        &self,
        target: &PlaybackTarget,
        discovery_id: &DiscoveryId,
    ) -> Result<Arc<Discovery>, Error> {
        let discovery = self
            .discoveries
            .get(discovery_id)
            .await
            .ok_or(Error::DiscoveryNotFound)?;
        if &discovery.target != target {
            return Err(Error::CandidateNotFound);
        }
        Ok(discovery)
    }

    async fn activate_aiostreams(
        &self,
        stream: &DirectStream,
        capabilities: &PlayerCapabilities,
    ) -> Result<PlaybackDescriptor, Error> {
        if !self.can_deliver(stream, capabilities) {
            return Err(Error::NoCompatibleSource);
        }
        // Discovery never contacts media. Resolve only after the user asks to
        // play, so automatic selection can fall back before returning a player.
        let response = self
            .aiostreams
            .as_ref()
            .ok_or(Error::AioStreamsNotConfigured)?
            .media_response(
                stream.url.clone(),
                Some("bytes=0-0"),
                &stream.request_headers,
            )
            .await?;
        if !response.status().is_success() {
            return Err(Error::RemoteProvidersUnavailable);
        }
        let source_url = response.url().clone();
        drop(response);
        let session_id = SessionId::generate();
        let hls_url = self
            .stremio
            .as_ref()
            .map(|server| {
                server.playlist_url(
                    session_id.as_str(),
                    &source_url,
                    !stream.request_headers.is_empty(),
                )
            })
            .transpose()?;
        let delivery = if hls_url.is_some()
            || source_url.path().to_ascii_lowercase().ends_with(".m3u8")
            || stream.container.as_deref().and_then(normalize_container) == Some(Container::Hls)
        {
            Delivery::Hls
        } else {
            Delivery::Direct
        };
        let duration_seconds = stream.duration_seconds;
        let container = stream.container.clone();
        let content_type = container.as_deref().and_then(direct_content_type);
        let pending_session = self
            .sessions
            .insert_pending(
                session_id.clone(),
                PlaybackSession {
                    source: SessionSource::AioStreams {
                        request_headers: stream.request_headers.clone(),
                        content_type,
                    },
                    source_url,
                    delivery: hls_url
                        .clone()
                        .map_or(SessionDelivery::Original, |playlist_url| {
                            SessionDelivery::Converted { playlist_url }
                        }),
                    resources: RwLock::new(ResourceRegistry::default()),
                },
            )
            .await?;
        if let (Some(server), Some(url)) = (&self.stremio, hls_url)
            && let Err(error) = server.media_response(url).await
        {
            return Err(error);
        }
        pending_session.commit();
        Ok(PlaybackDescriptor {
            session_id: session_id.clone(),
            source: PlaybackSource::AioStreams,
            delivery,
            media_url: session_id.media_path(),
            container,
            duration_seconds,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            selected_audio_index: None,
            selected_subtitle_index: None,
        })
    }

    async fn session(&self, session_id: &SessionId) -> Result<Arc<PlaybackSession>, Error> {
        self.sessions
            .get(session_id)
            .await
            .ok_or(Error::SessionNotFound)
    }
}

fn no_remote_sources_error(candidate_count: usize, upstream_error_count: usize) -> Error {
    if candidate_count > 0 {
        Error::NoCompatibleSource
    } else if upstream_error_count > 0 {
        Error::RemoteProvidersUnavailable
    } else {
        Error::NoRemoteSources
    }
}

#[cfg(test)]
mod tests;
