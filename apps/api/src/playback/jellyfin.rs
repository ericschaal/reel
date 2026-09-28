use super::{
    Error, Playback, PlaybackUpstream,
    capabilities::web_device_profile,
    ids::SessionId,
    proxy::without_jellyfin_credentials,
    session::{PlaybackSession, SessionDelivery, SessionSource},
    store::ResourceRegistry,
    types::{
        ActivationRequest, Delivery, PlaybackDescriptor, PlaybackSource, PlaybackTarget,
        PlaybackTrack,
    },
    values::{PlaybackPosition, SubtitleSelection},
};
use crate::{
    jellyfin::{
        Item, ItemType, ItemsQuery, Jellyfin, JellyfinItemId, JellyfinUserId, MediaSource,
        PlaybackInfoRequest,
    },
    media::{EpisodeNumber, SeasonNumber, TmdbId},
};
use reqwest::Url;
use std::sync::Arc;
use tokio::sync::RwLock;
const TICKS_PER_SECOND: i64 = 10_000_000;

impl Playback {
    pub(super) async fn activate_jellyfin(
        &self,
        request: &ActivationRequest,
    ) -> Result<PlaybackDescriptor, Error> {
        let item = self.resolve_item(&request.target).await?;
        let user_id = self.user_id().await?;
        let start_time_ticks = request
            .start_position_seconds
            .map(PlaybackPosition::jellyfin_ticks);
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
        if request.audio_stream_index.is_some()
            || !matches!(request.subtitle_stream_index, SubtitleSelection::Default)
        {
            // Jellyfin only applies track indexes when MediaSourceId matches.
            // Resolve the preferred source before negotiating its selected tracks.
            let SelectedSource { source, .. } = choose_source(&self.jellyfin, &item.id, &playback)?;
            playback_request.media_source_id =
                Some(source.id.clone().ok_or(Error::InvalidUpstreamResponse {
                    upstream: PlaybackUpstream::Jellyfin,
                })?);
            playback_request.audio_stream_index = request.audio_stream_index.map(i32::from);
            playback_request.subtitle_stream_index = request.subtitle_stream_index.into();
            playback_request.enable_direct_play = Some(false);
            playback = self
                .jellyfin
                .playback_info(&item.id, &user_id, &playback_request)
                .await?;
        }
        let SelectedSource {
            source,
            url: source_url,
            delivery,
        } = choose_source(&self.jellyfin, &item.id, &playback)?;
        let source_url = without_jellyfin_credentials(source_url);
        let session_id = SessionId::generate();
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
            .map(i32::from)
            .or(source.default_audio_stream_index);
        let selected_subtitle_index = request.subtitle_stream_index.into();

        self.sessions
            .insert(
                session_id.clone(),
                PlaybackSession {
                    source: SessionSource::Jellyfin { item_id: item.id },
                    source_url,
                    delivery: SessionDelivery::Original,
                    resources: RwLock::new(ResourceRegistry::default()),
                },
            )
            .await?;

        Ok(PlaybackDescriptor {
            session_id: session_id.clone(),
            source: PlaybackSource::Jellyfin,
            delivery,
            media_url: session_id.media_path(),
            container,
            duration_seconds,
            audio_tracks,
            subtitle_tracks,
            selected_audio_index,
            selected_subtitle_index,
        })
    }

    pub(super) async fn resolve_item(&self, target: &PlaybackTarget) -> Result<Item, Error> {
        match target {
            PlaybackTarget::Movie { tmdb_id, .. } => self.resolve_movie(*tmdb_id).await,
            PlaybackTarget::Episode {
                tmdb_id,
                series_tmdb_id,
                season_number,
                episode_number,
                ..
            } => {
                self.resolve_episode(*tmdb_id, *series_tmdb_id, *season_number, *episode_number)
                    .await
            }
        }
    }

    pub(super) async fn resolve_movie(&self, tmdb_id: TmdbId) -> Result<Item, Error> {
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

    pub(super) async fn resolve_episode(
        &self,
        tmdb_id: TmdbId,
        series_tmdb_id: TmdbId,
        season_number: SeasonNumber,
        episode_number: EpisodeNumber,
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
                item.index_number == Some(episode_number.get())
                    && provider_tmdb_id(item).is_none_or(|id| id == tmdb_id)
            })
            .ok_or(Error::NotLocal)
    }

    pub(super) async fn user_id(&self) -> Result<Arc<JellyfinUserId>, Error> {
        if let Some(user_id) = self.user_id.read().await.clone() {
            return Ok(user_id);
        }
        let user_id = Arc::new(
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
fn provider_tmdb_id(item: &Item) -> Option<TmdbId> {
    item.provider_ids
        .iter()
        .find(|(provider, _)| provider.eq_ignore_ascii_case("tmdb"))
        .and_then(|(_, id)| id.parse().ok())
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

struct SelectedSource<'a> {
    source: &'a MediaSource,
    url: Url,
    delivery: Delivery,
}

fn choose_source<'a>(
    jellyfin: &Jellyfin,
    item_id: &JellyfinItemId,
    playback: &'a crate::jellyfin::PlaybackInfoResponse,
) -> Result<SelectedSource<'a>, Error> {
    if let Some((source, id)) = playback.media_sources.iter().find_map(|source| {
        if source.supports_direct_play {
            source.id.as_ref().map(|id| (source, id))
        } else {
            None
        }
    }) {
        let url = jellyfin.direct_play_url(
            item_id,
            source.container.as_deref(),
            id,
            playback.play_session_id.as_deref(),
        )?;
        return Ok(SelectedSource {
            source,
            url,
            delivery: Delivery::Direct,
        });
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
                jellyfin.resolve_url(url).map(|url| SelectedSource {
                    source,
                    url,
                    delivery,
                })
            })
        })
        .transpose()?
        .ok_or(Error::NoCompatibleSource)
}
