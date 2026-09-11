use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore as _;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use thiserror::Error as ThisError;
use tokio::sync::RwLock;

use crate::{
    aiostreams::{AioStreams, DirectStream, SearchOutcome, SearchTarget},
    jellyfin::{
        DeviceProfile, DirectPlayProfile, DlnaProfileType, Item, ItemType, ItemsQuery, Jellyfin,
        MediaSource, MediaStreamProtocol, PlaybackInfoRequest, SubtitleDeliveryMethod,
        SubtitleProfile, TranscodingProfile,
    },
    media::{SeasonNumber, TmdbId},
};

const SESSION_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const DISCOVERY_TTL: Duration = Duration::from_secs(15 * 60);
const TICKS_PER_SECOND: i64 = 10_000_000;

#[derive(Clone)]
pub struct Playback {
    jellyfin: Jellyfin,
    aiostreams: Option<AioStreams>,
    username: Arc<str>,
    user_id: Arc<RwLock<Option<Arc<str>>>>,
    sessions: Arc<RwLock<HashMap<String, PlaybackSession>>>,
    discoveries: Arc<RwLock<HashMap<String, Discovery>>>,
}

#[derive(Clone)]
struct PlaybackSession {
    source: SessionSource,
    source_url: Url,
    expires_at: Instant,
    resources: Arc<RwLock<HashMap<String, Url>>>,
}

#[derive(Clone)]
enum SessionSource {
    Jellyfin { item_id: String },
    AioStreams { request_headers: HeaderMap },
}

#[derive(Clone)]
struct Discovery {
    target: PlaybackTarget,
    candidates: Vec<(String, DirectStream)>,
    expires_at: Instant,
}

impl Playback {
    #[must_use]
    pub fn new(jellyfin: Jellyfin, username: impl Into<String>) -> Self {
        Self {
            jellyfin,
            aiostreams: None,
            username: Arc::from(username.into()),
            user_id: Arc::new(RwLock::new(None)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            discoveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn with_aiostreams(mut self, aiostreams: AioStreams) -> Self {
        self.aiostreams = Some(aiostreams);
        self
    }

    #[must_use]
    pub fn with_user_id(jellyfin: Jellyfin, user_id: impl Into<String>) -> Self {
        Self {
            jellyfin,
            aiostreams: None,
            username: Arc::from(""),
            user_id: Arc::new(RwLock::new(Some(Arc::from(user_id.into())))),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            discoveries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/playback/sources", post(discover))
            .route("/v1/playback/activate", post(activate))
            .route("/v1/playback/sessions/{session_id}/media", get(media))
            .route(
                "/v1/playback/sessions/{session_id}/resources/{resource}",
                get(resource),
            )
            .with_state(self)
    }

    async fn activate(&self, request: ActivationRequest) -> Result<PlaybackDescriptor, Error> {
        match &request.selection {
            PlaybackSelection::Auto => match self.activate_jellyfin(&request).await {
                Ok(descriptor) => Ok(descriptor),
                Err(Error::NotLocal | Error::NoCompatibleSource) => {
                    self.activate_first_aiostreams(&request.target).await
                }
                Err(error) => Err(error),
            },
            PlaybackSelection::Jellyfin => self.activate_jellyfin(&request).await,
            PlaybackSelection::AioStreams {
                discovery_id,
                candidate_id,
            } => {
                self.activate_discovered_aiostreams(&request.target, discovery_id, candidate_id)
                    .await
            }
        }
    }

    async fn activate_jellyfin(
        &self,
        request: &ActivationRequest,
    ) -> Result<PlaybackDescriptor, Error> {
        let item = self.resolve_item(&request.target).await?;
        let user_id = self.user_id().await?;
        let start_time_ticks = request
            .start_position_seconds
            .map(seconds_to_ticks)
            .transpose()?;
        let mut playback_request = PlaybackInfoRequest {
            max_streaming_bitrate: request.capabilities.max_streaming_bitrate,
            start_time_ticks,
            device_profile: Some(web_device_profile(&request.capabilities)),
            enable_direct_play: Some(true),
            enable_direct_stream: Some(true),
            enable_transcoding: Some(request.capabilities.hls),
            allow_video_stream_copy: Some(true),
            allow_audio_stream_copy: Some(true),
            ..PlaybackInfoRequest::default()
        };
        let mut playback = self
            .jellyfin
            .playback_info(&item.id, &user_id, &playback_request)
            .await?;
        if request.audio_stream_index.is_some() || request.subtitle_stream_index.is_some() {
            // Jellyfin only applies track indexes when MediaSourceId matches.
            // Resolve the preferred source before negotiating its selected tracks.
            let (source, _, _) = choose_source(&self.jellyfin, &item.id, &playback)?;
            playback_request.media_source_id =
                Some(source.id.clone().ok_or(Error::InvalidUpstreamResponse)?);
            playback_request.audio_stream_index = request.audio_stream_index;
            playback_request.subtitle_stream_index = request.subtitle_stream_index;
            playback_request.enable_direct_play = Some(false);
            playback = self
                .jellyfin
                .playback_info(&item.id, &user_id, &playback_request)
                .await?;
        }
        let (source, source_url, delivery) = choose_source(&self.jellyfin, &item.id, &playback)?;
        let source_url = without_jellyfin_credentials(source_url);
        let session_id = random_session_id();
        let duration_seconds = source
            .run_time_ticks
            .or(item.run_time_ticks)
            .and_then(|ticks| u64::try_from(ticks / TICKS_PER_SECOND).ok());
        let container = source
            .transcoding_container
            .clone()
            .or_else(|| source.container.clone());
        let audio_tracks = playback_tracks(source, "Audio");
        let subtitle_tracks = playback_tracks(source, "Subtitle");
        let selected_audio_index = request
            .audio_stream_index
            .or(source.default_audio_stream_index);
        let selected_subtitle_index = request.subtitle_stream_index;

        self.remove_expired_sessions().await;
        self.sessions.write().await.insert(
            session_id.clone(),
            PlaybackSession {
                source: SessionSource::Jellyfin { item_id: item.id },
                source_url,
                expires_at: Instant::now() + SESSION_TTL,
                resources: Arc::new(RwLock::new(HashMap::new())),
            },
        );

        Ok(PlaybackDescriptor {
            session_id: session_id.clone(),
            source: PlaybackSource::Jellyfin,
            delivery,
            media_url: format!("/v1/playback/sessions/{session_id}/media"),
            container,
            duration_seconds,
            audio_tracks,
            subtitle_tracks,
            selected_audio_index,
            selected_subtitle_index,
        })
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
                size_bytes: None,
                web_ready: true,
            }),
            Err(Error::NotLocal) => {}
            Err(_) => issues.push(PlaybackIssue {
                source: PlaybackSource::Jellyfin,
                code: PlaybackIssueCode::UpstreamUnavailable,
            }),
        }

        let mut stored_candidates = Vec::new();
        match remote {
            Ok(SearchOutcome {
                streams,
                upstream_error_count,
            }) => {
                if upstream_error_count > 0 {
                    issues.push(PlaybackIssue {
                        source: PlaybackSource::AioStreams,
                        code: PlaybackIssueCode::PartialResults,
                    });
                }
                for stream in streams {
                    let id = random_session_id();
                    sources.push(PlaybackCandidate {
                        id: id.clone(),
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
                        size_bytes: stream.size_bytes,
                        web_ready: stream.web_ready,
                    });
                    stored_candidates.push((id, stream));
                }
            }
            Err(_) => issues.push(PlaybackIssue {
                source: PlaybackSource::AioStreams,
                code: PlaybackIssueCode::UpstreamUnavailable,
            }),
        }

        let discovery_id = random_session_id();
        self.remove_expired_discoveries().await;
        self.discoveries.write().await.insert(
            discovery_id.clone(),
            Discovery {
                target: request.target,
                candidates: stored_candidates,
                expires_at: Instant::now() + DISCOVERY_TTL,
            },
        );
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

    async fn activate_first_aiostreams(
        &self,
        target: &PlaybackTarget,
    ) -> Result<PlaybackDescriptor, Error> {
        let outcome = self.search_aiostreams(target).await?;
        let stream = outcome
            .streams
            .into_iter()
            .find(|stream| stream.web_ready)
            .ok_or(Error::NoRemoteSources)?;
        self.activate_aiostreams(stream).await
    }

    async fn activate_discovered_aiostreams(
        &self,
        target: &PlaybackTarget,
        discovery_id: &str,
        candidate_id: &str,
    ) -> Result<PlaybackDescriptor, Error> {
        let discovery = self
            .discoveries
            .read()
            .await
            .get(discovery_id)
            .cloned()
            .ok_or(Error::DiscoveryNotFound)?;
        if discovery.expires_at <= Instant::now() {
            self.discoveries.write().await.remove(discovery_id);
            return Err(Error::DiscoveryNotFound);
        }
        if &discovery.target != target {
            return Err(Error::CandidateNotFound);
        }
        let stream = discovery
            .candidates
            .into_iter()
            .find_map(|(id, stream)| (id == candidate_id).then_some(stream))
            .ok_or(Error::CandidateNotFound)?;
        self.activate_aiostreams(stream).await
    }

    async fn activate_aiostreams(&self, stream: DirectStream) -> Result<PlaybackDescriptor, Error> {
        if !stream.web_ready {
            return Err(Error::NoCompatibleSource);
        }
        let delivery = if stream.url.path().to_ascii_lowercase().ends_with(".m3u8")
            || stream.container.as_deref().is_some_and(|container| {
                matches!(container.to_ascii_lowercase().as_str(), "hls" | "m3u8")
            }) {
            Delivery::Hls
        } else {
            Delivery::Direct
        };
        let session_id = random_session_id();
        let duration_seconds = stream.duration_seconds;
        let container = stream.container;
        self.remove_expired_sessions().await;
        self.sessions.write().await.insert(
            session_id.clone(),
            PlaybackSession {
                source: SessionSource::AioStreams {
                    request_headers: stream.request_headers,
                },
                source_url: stream.url,
                expires_at: Instant::now() + SESSION_TTL,
                resources: Arc::new(RwLock::new(HashMap::new())),
            },
        );
        Ok(PlaybackDescriptor {
            session_id: session_id.clone(),
            source: PlaybackSource::AioStreams,
            delivery,
            media_url: format!("/v1/playback/sessions/{session_id}/media"),
            container,
            duration_seconds,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            selected_audio_index: None,
            selected_subtitle_index: None,
        })
    }

    async fn resolve_item(&self, target: &PlaybackTarget) -> Result<Item, Error> {
        match target {
            PlaybackTarget::Movie { tmdb_id } => self.resolve_movie(*tmdb_id).await,
            PlaybackTarget::Episode {
                tmdb_id,
                series_tmdb_id,
                season_number,
                episode_number,
            } => {
                self.resolve_episode(*tmdb_id, *series_tmdb_id, *season_number, *episode_number)
                    .await
            }
        }
    }

    async fn resolve_movie(&self, tmdb_id: TmdbId) -> Result<Item, Error> {
        self.jellyfin
            .items(&ItemsQuery {
                include_item_types: vec![ItemType::Movie],
                recursive: Some(true),
                ..ItemsQuery::default()
            })
            .await?
            .items
            .into_iter()
            .find(|item| provider_tmdb_id(item) == Some(tmdb_id))
            .ok_or(Error::NotLocal)
    }

    async fn resolve_episode(
        &self,
        tmdb_id: TmdbId,
        series_tmdb_id: TmdbId,
        season_number: SeasonNumber,
        episode_number: i32,
    ) -> Result<Item, Error> {
        let series = self
            .jellyfin
            .items(&ItemsQuery {
                include_item_types: vec![ItemType::Series],
                recursive: Some(true),
                ..ItemsQuery::default()
            })
            .await?
            .items
            .into_iter()
            .find(|item| provider_tmdb_id(item) == Some(series_tmdb_id))
            .ok_or(Error::NotLocal)?;
        let season = self
            .jellyfin
            .seasons(&series.id)
            .await?
            .items
            .into_iter()
            .find(|item| item.index_number == Some(season_number.get()))
            .ok_or(Error::NotLocal)?;
        self.jellyfin
            .episodes(&series.id, Some(&season.id))
            .await?
            .items
            .into_iter()
            .find(|item| {
                item.index_number == Some(episode_number)
                    && provider_tmdb_id(item).is_none_or(|id| id == tmdb_id)
            })
            .ok_or(Error::NotLocal)
    }

    async fn session(&self, session_id: &str) -> Result<PlaybackSession, Error> {
        let session = self
            .sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or(Error::SessionNotFound)?;
        if session.expires_at <= Instant::now() {
            self.sessions.write().await.remove(session_id);
            return Err(Error::SessionNotFound);
        }
        Ok(session)
    }

    async fn remove_expired_sessions(&self) {
        let now = Instant::now();
        self.sessions
            .write()
            .await
            .retain(|_, session| session.expires_at > now);
    }

    async fn remove_expired_discoveries(&self) {
        let now = Instant::now();
        self.discoveries
            .write()
            .await
            .retain(|_, discovery| discovery.expires_at > now);
    }

    async fn user_id(&self) -> Result<Arc<str>, Error> {
        if let Some(user_id) = self.user_id.read().await.clone() {
            return Ok(user_id);
        }
        let user_id: Arc<str> = Arc::from(
            self.jellyfin
                .users()
                .await?
                .into_iter()
                .find(|user| {
                    user.name
                        .as_deref()
                        .is_some_and(|name| name.eq_ignore_ascii_case(&self.username))
                })
                .map(|user| user.id)
                .ok_or(Error::UserNotFound)?,
        );
        *self.user_id.write().await = Some(user_id.clone());
        Ok(user_id)
    }
}

async fn activate(
    State(playback): State<Playback>,
    Json(request): Json<ActivationRequest>,
) -> Result<Json<PlaybackDescriptor>, Error> {
    playback.activate(request).await.map(Json)
}

async fn discover(
    State(playback): State<Playback>,
    Json(request): Json<DiscoveryRequest>,
) -> Result<Json<DiscoveryResponse>, Error> {
    playback.discover(request).await.map(Json)
}

async fn media(
    State(playback): State<Playback>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    proxy(
        &playback,
        &session_id,
        &session,
        session.source_url.clone(),
        headers,
    )
    .await
}

async fn resource(
    State(playback): State<Playback>,
    Path((session_id, resource)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    let url = match &session.source {
        SessionSource::Jellyfin { item_id } => {
            let decoded = URL_SAFE_NO_PAD
                .decode(resource)
                .map_err(|_| Error::InvalidResource)?;
            let url =
                Url::parse(std::str::from_utf8(&decoded).map_err(|_| Error::InvalidResource)?)
                    .map_err(|_| Error::InvalidResource)?;
            if !playback.jellyfin.has_same_origin(&url) || !is_item_video_url(&url, item_id) {
                return Err(Error::InvalidResource);
            }
            url
        }
        SessionSource::AioStreams { .. } => session
            .resources
            .read()
            .await
            .get(&resource)
            .cloned()
            .ok_or(Error::InvalidResource)?,
    };
    proxy(&playback, &session_id, &session, url, headers).await
}

async fn proxy(
    playback: &Playback,
    session_id: &str,
    session: &PlaybackSession,
    url: Url,
    request_headers: HeaderMap,
) -> Result<Response, Error> {
    let range = request_headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let upstream = match &session.source {
        SessionSource::Jellyfin { .. } => {
            playback.jellyfin.media_response(url.clone(), range).await?
        }
        SessionSource::AioStreams { request_headers } => {
            playback
                .aiostreams
                .as_ref()
                .ok_or(Error::AioStreamsNotConfigured)?
                .media_response(url.clone(), range, request_headers)
                .await?
        }
    };
    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let upstream_url = upstream.url().clone();
    let is_playlist = upstream_url.path().to_ascii_lowercase().ends_with(".m3u8")
        || upstream_headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("mpegurl"));

    if is_playlist && status.is_success() {
        let text = upstream.text().await.map_err(Error::InvalidUpstreamBody)?;
        let rewritten =
            rewrite_playlist(playback, &text, &upstream_url, session_id, session).await?;
        return Response::builder()
            .status(status)
            .header(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")
            .header(header::CACHE_CONTROL, "no-store")
            .body(Body::from(rewritten))
            .map_err(|_| Error::InvalidUpstreamResponse);
    }

    let mut response = Response::builder().status(status);
    for name in [
        header::CONTENT_TYPE,
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
        header::CACHE_CONTROL,
        header::ETAG,
        header::LAST_MODIFIED,
    ] {
        if let Some(value) = upstream_headers.get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .map_err(|_| Error::InvalidUpstreamResponse)
}

async fn rewrite_playlist(
    playback: &Playback,
    text: &str,
    base_url: &Url,
    session_id: &str,
    session: &PlaybackSession,
) -> Result<String, Error> {
    let mut output = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        let rewritten = if line.starts_with('#') {
            rewrite_uri_attributes(playback, line, base_url, session_id, session).await?
        } else if line.trim().is_empty() {
            String::new()
        } else {
            register_resource_url(
                playback,
                base_url.join(line).map_err(|_| Error::InvalidResource)?,
                session_id,
                session,
            )
            .await?
        };
        output.push_str(&rewritten);
        output.push('\n');
    }
    Ok(output)
}

async fn rewrite_uri_attributes(
    playback: &Playback,
    line: &str,
    base_url: &Url,
    session_id: &str,
    session: &PlaybackSession,
) -> Result<String, Error> {
    let mut output = line.to_owned();
    let mut search_from = 0;
    while let Some(relative_start) = output[search_from..].find("URI=\"") {
        let value_start = search_from + relative_start + 5;
        let Some(relative_end) = output[value_start..].find('"') else {
            return Err(Error::InvalidUpstreamResponse);
        };
        let value_end = value_start + relative_end;
        let resolved = base_url
            .join(&output[value_start..value_end])
            .map_err(|_| Error::InvalidResource)?;
        let replacement = register_resource_url(playback, resolved, session_id, session).await?;
        output.replace_range(value_start..value_end, &replacement);
        search_from = value_start + replacement.len() + 1;
    }
    Ok(output)
}

async fn register_resource_url(
    playback: &Playback,
    mut url: Url,
    session_id: &str,
    session: &PlaybackSession,
) -> Result<String, Error> {
    match &session.source {
        SessionSource::Jellyfin { item_id } => {
            if !playback.jellyfin.has_same_origin(&url) || !is_item_video_url(&url, item_id) {
                return Err(Error::InvalidResource);
            }
            url = without_jellyfin_credentials(url);
            return Ok(proxy_resource_url(url, session_id));
        }
        SessionSource::AioStreams { .. } => {
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(Error::InvalidResource);
            }
            url.set_fragment(None);
        }
    }
    let resource_id = random_session_id();
    session
        .resources
        .write()
        .await
        .insert(resource_id.clone(), url);
    Ok(format!(
        "/v1/playback/sessions/{session_id}/resources/{resource_id}"
    ))
}

fn proxy_resource_url(url: Url, session_id: &str) -> String {
    let encoded = URL_SAFE_NO_PAD.encode(url.as_str());
    format!("/v1/playback/sessions/{session_id}/resources/{encoded}")
}

fn without_jellyfin_credentials(mut url: Url) -> Url {
    let pairs = url
        .query_pairs()
        .filter(|(name, _)| {
            let normalized = name
                .chars()
                .filter(|character| *character != '_' && *character != '-')
                .flat_map(char::to_lowercase)
                .collect::<String>();
            normalized != "apikey" && normalized != "xembytoken" && normalized != "accesstoken"
        })
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    url.set_query(None);
    if !pairs.is_empty() {
        url.query_pairs_mut().extend_pairs(pairs);
    }
    url
}

fn is_item_video_url(url: &Url, item_id: &str) -> bool {
    let segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let normalized_item_id = item_id.replace('-', "");
    segments.windows(2).any(|pair| {
        pair[0].eq_ignore_ascii_case("videos")
            && pair[1]
                .replace('-', "")
                .eq_ignore_ascii_case(&normalized_item_id)
    })
}

fn provider_tmdb_id(item: &Item) -> Option<TmdbId> {
    item.provider_ids
        .iter()
        .find(|(provider, _)| provider.eq_ignore_ascii_case("tmdb"))
        .and_then(|(_, id)| id.parse().ok())
}

fn seconds_to_ticks(seconds: f64) -> Result<i64, Error> {
    let max_seconds = i64::MAX as f64 / TICKS_PER_SECOND as f64;
    if !seconds.is_finite() || seconds < 0.0 || seconds > max_seconds {
        return Err(Error::InvalidStartPosition);
    }
    Ok((seconds * TICKS_PER_SECOND as f64).round() as i64)
}

fn playback_tracks(source: &MediaSource, stream_type: &str) -> Vec<PlaybackTrack> {
    source
        .media_streams
        .iter()
        .filter(|stream| stream.stream_type.eq_ignore_ascii_case(stream_type))
        .map(|stream| PlaybackTrack {
            index: stream.index,
            label: stream
                .display_title
                .clone()
                .or_else(|| stream.title.clone())
                .or_else(|| stream.language.clone())
                .or_else(|| stream.codec.clone())
                .unwrap_or_else(|| format!("{stream_type} {}", stream.index)),
            language: stream.language.clone(),
            codec: stream.codec.clone(),
            is_default: stream.is_default,
            is_forced: stream.is_forced,
        })
        .collect()
}

fn choose_source<'a>(
    jellyfin: &Jellyfin,
    item_id: &str,
    playback: &'a crate::jellyfin::PlaybackInfoResponse,
) -> Result<(&'a MediaSource, Url, Delivery), Error> {
    if let Some((source, id)) = playback.media_sources.iter().find_map(|source| {
        source
            .supports_direct_play
            .then(|| source.id.as_deref().map(|id| (source, id)))
            .flatten()
    }) {
        let url = jellyfin.direct_play_url(
            item_id,
            source.container.as_deref(),
            id,
            playback.play_session_id.as_deref(),
        )?;
        return Ok((source, url, Delivery::Direct));
    }
    playback
        .media_sources
        .iter()
        .find_map(|source| {
            source.transcoding_url.as_deref().map(|url| {
                let delivery = if url.contains(".m3u8")
                    || source.transcoding_container.as_deref() == Some("ts")
                {
                    Delivery::Hls
                } else {
                    Delivery::Direct
                };
                jellyfin.resolve_url(url).map(|url| (source, url, delivery))
            })
        })
        .transpose()?
        .ok_or(Error::NoCompatibleSource)
}

fn web_device_profile(capabilities: &PlayerCapabilities) -> DeviceProfile {
    let supports = |values: &[String], wanted: &str| {
        values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(wanted))
    };
    let mut direct_play_profiles = Vec::new();
    if supports(&capabilities.containers, "mp4")
        && supports(&capabilities.video_codecs, "h264")
        && supports(&capabilities.audio_codecs, "aac")
    {
        direct_play_profiles.push(DirectPlayProfile {
            container: "mp4,m4v,mov".into(),
            audio_codec: Some("aac,mp3".into()),
            video_codec: Some("h264".into()),
            profile_type: DlnaProfileType::Video,
        });
    }
    if supports(&capabilities.containers, "webm") {
        direct_play_profiles.push(DirectPlayProfile {
            container: "webm".into(),
            audio_codec: Some("opus,vorbis".into()),
            video_codec: Some("vp8,vp9".into()),
            profile_type: DlnaProfileType::Video,
        });
    }
    let transcoding_profiles = capabilities
        .hls
        .then(|| TranscodingProfile {
            container: "ts".into(),
            profile_type: DlnaProfileType::Video,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            protocol: MediaStreamProtocol::Hls,
            max_audio_channels: Some("2".into()),
            estimate_content_length: false,
            enable_subtitles_in_manifest: true,
        })
        .into_iter()
        .collect();
    DeviceProfile {
        name: Some("Reel Web".into()),
        max_streaming_bitrate: capabilities.max_streaming_bitrate,
        max_static_bitrate: capabilities.max_streaming_bitrate,
        direct_play_profiles,
        transcoding_profiles,
        subtitle_profiles: vec![SubtitleProfile {
            format: Some("vtt".into()),
            method: SubtitleDeliveryMethod::Hls,
            language: None,
            container: None,
        }],
        ..DeviceProfile::default()
    }
}

fn random_session_id() -> String {
    let mut bytes = [0_u8; 24];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivationRequest {
    target: PlaybackTarget,
    #[serde(default)]
    selection: PlaybackSelection,
    #[serde(default)]
    capabilities: PlayerCapabilities,
    start_position_seconds: Option<f64>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum PlaybackTarget {
    Movie {
        tmdb_id: TmdbId,
    },
    Episode {
        tmdb_id: TmdbId,
        series_tmdb_id: TmdbId,
        season_number: SeasonNumber,
        episode_number: i32,
    },
}

impl PlaybackTarget {
    fn aiostreams_target(&self) -> SearchTarget {
        match self {
            Self::Movie { tmdb_id } => SearchTarget::Movie {
                tmdb_id: tmdb_id.get(),
            },
            Self::Episode {
                series_tmdb_id,
                season_number,
                episode_number,
                ..
            } => SearchTarget::Episode {
                series_tmdb_id: series_tmdb_id.get(),
                season_number: season_number.get(),
                episode_number: *episode_number,
            },
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum PlaybackSelection {
    #[default]
    Auto,
    Jellyfin,
    AioStreams {
        discovery_id: String,
        candidate_id: String,
    },
}

#[derive(Debug, Deserialize)]
struct DiscoveryRequest {
    target: PlaybackTarget,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerCapabilities {
    #[serde(default = "default_containers")]
    containers: Vec<String>,
    #[serde(default = "default_video_codecs")]
    video_codecs: Vec<String>,
    #[serde(default = "default_audio_codecs")]
    audio_codecs: Vec<String>,
    #[serde(default = "default_true")]
    hls: bool,
    max_streaming_bitrate: Option<i32>,
}

impl Default for PlayerCapabilities {
    fn default() -> Self {
        Self {
            containers: default_containers(),
            video_codecs: default_video_codecs(),
            audio_codecs: default_audio_codecs(),
            hls: true,
            max_streaming_bitrate: Some(40_000_000),
        }
    }
}

fn default_containers() -> Vec<String> {
    vec!["mp4".into(), "webm".into()]
}

fn default_video_codecs() -> Vec<String> {
    vec!["h264".into(), "vp8".into(), "vp9".into()]
}

fn default_audio_codecs() -> Vec<String> {
    vec!["aac".into(), "mp3".into(), "opus".into(), "vorbis".into()]
}

const fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackDescriptor {
    session_id: String,
    source: PlaybackSource,
    delivery: Delivery,
    media_url: String,
    container: Option<String>,
    duration_seconds: Option<u64>,
    audio_tracks: Vec<PlaybackTrack>,
    subtitle_tracks: Vec<PlaybackTrack>,
    selected_audio_index: Option<i32>,
    selected_subtitle_index: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackTrack {
    index: i32,
    label: String,
    language: Option<String>,
    codec: Option<String>,
    is_default: bool,
    is_forced: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum PlaybackSource {
    AioStreams,
    Jellyfin,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryResponse {
    discovery_id: String,
    sources: Vec<PlaybackCandidate>,
    issues: Vec<PlaybackIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackCandidate {
    id: String,
    source: PlaybackSource,
    label: String,
    description: Option<String>,
    preferred: bool,
    addon: Option<String>,
    service: Option<String>,
    cached: Option<bool>,
    resolution: Option<String>,
    quality: Option<String>,
    container: Option<String>,
    size_bytes: Option<u64>,
    web_ready: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaybackIssue {
    source: PlaybackSource,
    code: PlaybackIssueCode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum PlaybackIssueCode {
    PartialResults,
    UpstreamUnavailable,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum Delivery {
    Direct,
    Hls,
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("the configured Jellyfin playback user was not found")]
    UserNotFound,
    #[error("the requested media is not available locally")]
    NotLocal,
    #[error("Jellyfin did not return a compatible playback source")]
    NoCompatibleSource,
    #[error("the playback session was not found or has expired")]
    SessionNotFound,
    #[error("the playback resource is invalid")]
    InvalidResource,
    #[error("the playback start position is invalid")]
    InvalidStartPosition,
    #[error("the source discovery was not found or has expired")]
    DiscoveryNotFound,
    #[error("the selected playback candidate was not found")]
    CandidateNotFound,
    #[error("AIOStreams is not configured")]
    AioStreamsNotConfigured,
    #[error("AIOStreams returned no compatible direct sources")]
    NoRemoteSources,
    #[error("Jellyfin returned an invalid playback response")]
    InvalidUpstreamResponse,
    #[error("the upstream media body could not be read: {0}")]
    InvalidUpstreamBody(reqwest::Error),
    #[error(transparent)]
    Jellyfin(#[from] crate::jellyfin::Error),
    #[error(transparent)]
    AioStreams(#[from] crate::aiostreams::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::NotLocal => (
                StatusCode::NOT_FOUND,
                "not_local",
                "The requested media is not available in Jellyfin",
            ),
            Self::NoCompatibleSource => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "no_compatible_source",
                "No compatible playback source is available",
            ),
            Self::SessionNotFound => (
                StatusCode::NOT_FOUND,
                "playback_session_not_found",
                "The playback session was not found or has expired",
            ),
            Self::InvalidResource => (
                StatusCode::BAD_REQUEST,
                "invalid_playback_resource",
                "The playback resource is invalid",
            ),
            Self::InvalidStartPosition => (
                StatusCode::BAD_REQUEST,
                "invalid_playback_position",
                "The playback start position must be a finite non-negative number",
            ),
            Self::DiscoveryNotFound => (
                StatusCode::NOT_FOUND,
                "source_discovery_not_found",
                "The source list was not found or has expired",
            ),
            Self::CandidateNotFound => (
                StatusCode::BAD_REQUEST,
                "playback_candidate_not_found",
                "The selected playback source is invalid",
            ),
            Self::AioStreamsNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "aiostreams_not_configured",
                "Remote playback is not configured",
            ),
            Self::NoRemoteSources => (
                StatusCode::NOT_FOUND,
                "no_remote_sources",
                "No compatible direct streams are available",
            ),
            Self::AioStreams(_) | Self::InvalidUpstreamBody(_) => (
                StatusCode::BAD_GATEWAY,
                "aiostreams_unavailable",
                "Remote playback is temporarily unavailable",
            ),
            Self::UserNotFound | Self::InvalidUpstreamResponse | Self::Jellyfin(_) => (
                StatusCode::BAD_GATEWAY,
                "jellyfin_unavailable",
                "Jellyfin playback is temporarily unavailable",
            ),
        };
        (
            status,
            Json(serde_json::json!({"error": {"code": code, "message": message}})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rewrites_jellyfin_playlist_resources_through_the_scoped_session() {
        let base = Url::parse("http://jellyfin/Videos/item/master.m3u8?x=1").unwrap();
        let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"keys/key.bin\"\nmain.m3u8?ApiKey=secret&quality=high\n";
        let playback = Playback::with_user_id(
            Jellyfin::new("http://jellyfin", "secret-api-key").unwrap(),
            "user",
        );
        let session = PlaybackSession {
            source: SessionSource::Jellyfin {
                item_id: "item".into(),
            },
            source_url: base.clone(),
            expires_at: Instant::now() + SESSION_TTL,
            resources: Arc::new(RwLock::new(HashMap::new())),
        };
        let rewritten = rewrite_playlist(&playback, playlist, &base, "session", &session)
            .await
            .unwrap();
        assert!(rewritten.contains("/v1/playback/sessions/session/resources/"));
        assert!(!rewritten.contains("keys/key.bin"));
        let resource_id = rewritten
            .lines()
            .find(|line| !line.starts_with('#'))
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(resource_id).unwrap();
        let resource = std::str::from_utf8(&decoded).unwrap();
        assert!(!resource.to_ascii_lowercase().contains("apikey"));
        assert!(resource.contains("quality=high"));
    }

    #[test]
    fn accepts_only_video_resources_for_the_selected_item() {
        assert!(is_item_video_url(
            &Url::parse("http://jellyfin/Videos/abc/hls1/main/0.ts").unwrap(),
            "abc"
        ));
        assert!(is_item_video_url(
            &Url::parse("http://jellyfin/videos/09ffae59-97f6-0223-9553-a73c336799cf/main.m3u8")
                .unwrap(),
            "09ffae5997f602239553a73c336799cf"
        ));
        assert!(is_item_video_url(
            &Url::parse("http://jellyfin/prefix/Videos/ABC/stream").unwrap(),
            "abc"
        ));
        assert!(!is_item_video_url(
            &Url::parse("http://jellyfin/Users").unwrap(),
            "abc"
        ));
        assert!(!is_item_video_url(
            &Url::parse("http://jellyfin/Videos/other/stream").unwrap(),
            "abc"
        ));
    }
}
