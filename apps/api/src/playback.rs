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
    jellyfin::{
        DeviceProfile, DirectPlayProfile, DlnaProfileType, Item, ItemType, ItemsQuery, Jellyfin,
        MediaSource, MediaStreamProtocol, PlaybackInfoRequest, SubtitleDeliveryMethod,
        SubtitleProfile, TranscodingProfile,
    },
    media::{SeasonNumber, TmdbId},
};

const SESSION_TTL: Duration = Duration::from_secs(6 * 60 * 60);
const TICKS_PER_SECOND: i64 = 10_000_000;

#[derive(Clone)]
pub struct Playback {
    jellyfin: Jellyfin,
    username: Arc<str>,
    user_id: Arc<RwLock<Option<Arc<str>>>>,
    sessions: Arc<RwLock<HashMap<String, PlaybackSession>>>,
}

#[derive(Clone)]
struct PlaybackSession {
    item_id: String,
    source_url: Url,
    expires_at: Instant,
}

impl Playback {
    #[must_use]
    pub fn new(jellyfin: Jellyfin, username: impl Into<String>) -> Self {
        Self {
            jellyfin,
            username: Arc::from(username.into()),
            user_id: Arc::new(RwLock::new(None)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[must_use]
    pub fn with_user_id(jellyfin: Jellyfin, user_id: impl Into<String>) -> Self {
        Self {
            jellyfin,
            username: Arc::from(""),
            user_id: Arc::new(RwLock::new(Some(Arc::from(user_id.into())))),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/playback/activate", post(activate))
            .route("/v1/playback/sessions/{session_id}/media", get(media))
            .route(
                "/v1/playback/sessions/{session_id}/resources/{resource}",
                get(resource),
            )
            .with_state(self)
    }

    async fn activate(&self, request: ActivationRequest) -> Result<PlaybackDescriptor, Error> {
        let item = self.resolve_item(&request.target).await?;
        let user_id = self.user_id().await?;
        let start_time_ticks = request
            .start_position_seconds
            .map(seconds_to_ticks)
            .transpose()?;
        let playback = self
            .jellyfin
            .playback_info(
                &item.id,
                &user_id,
                &PlaybackInfoRequest {
                    max_streaming_bitrate: request.capabilities.max_streaming_bitrate,
                    start_time_ticks,
                    audio_stream_index: request.audio_stream_index,
                    subtitle_stream_index: request.subtitle_stream_index,
                    device_profile: Some(web_device_profile(&request.capabilities)),
                    enable_direct_play: Some(
                        request.audio_stream_index.is_none()
                            && request.subtitle_stream_index.is_none(),
                    ),
                    enable_direct_stream: Some(true),
                    enable_transcoding: Some(request.capabilities.hls),
                    allow_video_stream_copy: Some(true),
                    allow_audio_stream_copy: Some(true),
                    ..PlaybackInfoRequest::default()
                },
            )
            .await?;
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
                item_id: item.id,
                source_url,
                expires_at: Instant::now() + SESSION_TTL,
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

async fn media(
    State(playback): State<Playback>,
    Path(session_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    proxy(&playback, &session_id, session.source_url.clone(), headers).await
}

async fn resource(
    State(playback): State<Playback>,
    Path((session_id, resource)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Response, Error> {
    let session = playback.session(&session_id).await?;
    let decoded = URL_SAFE_NO_PAD
        .decode(resource)
        .map_err(|_| Error::InvalidResource)?;
    let url = Url::parse(std::str::from_utf8(&decoded).map_err(|_| Error::InvalidResource)?)
        .map_err(|_| Error::InvalidResource)?;
    if !playback.jellyfin.has_same_origin(&url) || !is_item_video_url(&url, &session.item_id) {
        return Err(Error::InvalidResource);
    }
    proxy(&playback, &session_id, url, headers).await
}

async fn proxy(
    playback: &Playback,
    session_id: &str,
    url: Url,
    request_headers: HeaderMap,
) -> Result<Response, Error> {
    let range = request_headers
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok());
    let upstream = playback.jellyfin.media_response(url.clone(), range).await?;
    let status = upstream.status();
    let upstream_headers = upstream.headers().clone();
    let is_playlist = url.path().ends_with(".m3u8")
        || upstream_headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.contains("mpegurl"));

    if is_playlist && status.is_success() {
        let text = upstream
            .text()
            .await
            .map_err(|source| crate::jellyfin::Error::Transport {
                integration: crate::integration::Integration::Jellyfin,
                source,
            })?;
        let rewritten = rewrite_playlist(&text, &url, session_id)?;
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

fn rewrite_playlist(text: &str, base_url: &Url, session_id: &str) -> Result<String, Error> {
    let mut output = String::with_capacity(text.len() + 256);
    for line in text.lines() {
        let rewritten = if line.starts_with('#') {
            rewrite_uri_attributes(line, base_url, session_id)?
        } else if line.trim().is_empty() {
            String::new()
        } else {
            proxy_resource_url(
                base_url.join(line).map_err(|_| Error::InvalidResource)?,
                session_id,
            )
        };
        output.push_str(&rewritten);
        output.push('\n');
    }
    Ok(output)
}

fn rewrite_uri_attributes(line: &str, base_url: &Url, session_id: &str) -> Result<String, Error> {
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
        let replacement = proxy_resource_url(resolved, session_id);
        output.replace_range(value_start..value_end, &replacement);
        search_from = value_start + replacement.len() + 1;
    }
    Ok(output)
}

fn proxy_resource_url(url: Url, session_id: &str) -> String {
    let url = without_jellyfin_credentials(url);
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
            method: SubtitleDeliveryMethod::External,
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
    capabilities: PlayerCapabilities,
    start_position_seconds: Option<f64>,
    audio_stream_index: Option<i32>,
    subtitle_stream_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum PlaybackSource {
    Jellyfin,
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
    #[error("Jellyfin returned an invalid playback response")]
    InvalidUpstreamResponse,
    #[error(transparent)]
    Jellyfin(#[from] crate::jellyfin::Error),
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
                "Jellyfin could not provide a compatible playback source",
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

    #[test]
    fn rewrites_playlist_resources_through_the_scoped_session() {
        let base = Url::parse("http://jellyfin/Videos/item/master.m3u8?x=1").unwrap();
        let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"keys/key.bin\"\nmain.m3u8?ApiKey=secret&quality=high\n";
        let rewritten = rewrite_playlist(playlist, &base, "session").unwrap();
        assert!(rewritten.contains("/v1/playback/sessions/session/resources/"));
        assert!(!rewritten.contains("keys/key.bin"));
        let encoded = rewritten
            .lines()
            .find(|line| !line.starts_with('#'))
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap();
        let decoded = URL_SAFE_NO_PAD.decode(encoded).unwrap();
        let decoded = std::str::from_utf8(&decoded).unwrap();
        assert!(!decoded.to_ascii_lowercase().contains("apikey"));
        assert!(decoded.contains("quality=high"));
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
