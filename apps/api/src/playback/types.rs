use super::{
    ids::{CandidateId, DiscoveryId, SessionId},
    values::{AudioStreamIndex, PlaybackPosition, SubtitleSelection},
};
use crate::{
    aiostreams::SearchTarget,
    media::{EpisodeNumber, ImdbTitleId, SeasonNumber, TmdbId},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ActivationRequest {
    pub(super) target: PlaybackTarget,
    #[serde(default)]
    pub(super) selection: PlaybackSelection,
    #[serde(default)]
    pub(super) capabilities: PlayerCapabilities,
    pub(super) start_position_seconds: Option<PlaybackPosition>,
    pub(super) audio_stream_index: Option<AudioStreamIndex>,
    #[serde(default)]
    pub(super) subtitle_stream_index: SubtitleSelection,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum PlaybackTarget {
    Movie {
        tmdb_id: TmdbId,
        imdb_id: Option<ImdbTitleId>,
    },
    Episode {
        tmdb_id: TmdbId,
        series_tmdb_id: TmdbId,
        imdb_id: Option<ImdbTitleId>,
        season_number: SeasonNumber,
        episode_number: EpisodeNumber,
    },
}

impl PlaybackTarget {
    pub(super) fn aiostreams_target(&self) -> SearchTarget {
        match self {
            Self::Movie { tmdb_id, imdb_id } => SearchTarget::Movie {
                tmdb_id: *tmdb_id,
                imdb_id: imdb_id.clone(),
            },
            Self::Episode {
                series_tmdb_id,
                imdb_id,
                season_number,
                episode_number,
                ..
            } => SearchTarget::Episode {
                series_tmdb_id: *series_tmdb_id,
                imdb_id: imdb_id.clone(),
                season_number: *season_number,
                episode_number: *episode_number,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum PlaybackSelection {
    Auto {
        discovery_id: Option<DiscoveryId>,
    },
    Jellyfin,
    AioStreams {
        discovery_id: DiscoveryId,
        candidate_id: CandidateId,
    },
}

impl Default for PlaybackSelection {
    fn default() -> Self {
        Self::Auto { discovery_id: None }
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct DiscoveryRequest {
    pub(super) target: PlaybackTarget,
    #[serde(default)]
    pub(super) capabilities: PlayerCapabilities,
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(super) struct PlayerCapabilities {
    pub(super) containers: Vec<String>,
    pub(super) video_codecs: Vec<String>,
    pub(super) audio_codecs: Vec<String>,
    pub(super) hls: bool,
    pub(super) max_streaming_bitrate: Option<i32>,
    pub(super) direct_play_profiles: Vec<BrowserDirectPlayProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct BrowserDirectPlayProfile {
    pub(super) container: String,
    pub(super) video_codec: Option<String>,
}

impl Default for PlayerCapabilities {
    fn default() -> Self {
        Self {
            containers: default_containers(),
            video_codecs: default_video_codecs(),
            audio_codecs: default_audio_codecs(),
            hls: true,
            max_streaming_bitrate: Some(40_000_000),
            direct_play_profiles: Vec::new(),
        }
    }
}

pub(super) fn default_containers() -> Vec<String> {
    vec!["mp4".into(), "webm".into()]
}

pub(super) fn default_video_codecs() -> Vec<String> {
    vec!["h264".into(), "vp8".into(), "vp9".into()]
}

pub(super) fn default_audio_codecs() -> Vec<String> {
    vec!["aac".into(), "mp3".into(), "opus".into(), "vorbis".into()]
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlaybackDescriptor {
    pub(super) session_id: SessionId,
    pub(super) source: PlaybackSource,
    pub(super) delivery: Delivery,
    pub(super) media_url: String,
    pub(super) container: Option<String>,
    pub(super) duration_seconds: Option<u64>,
    pub(super) audio_tracks: Vec<PlaybackTrack>,
    pub(super) subtitle_tracks: Vec<PlaybackTrack>,
    pub(super) selected_audio_index: Option<i32>,
    pub(super) selected_subtitle_index: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlaybackTrack {
    pub(super) index: i32,
    pub(super) label: String,
    pub(super) language: Option<String>,
    pub(super) codec: Option<String>,
    pub(super) is_default: bool,
    pub(super) is_forced: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum PlaybackSource {
    AioStreams,
    Jellyfin,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DiscoveryResponse {
    pub(super) discovery_id: DiscoveryId,
    pub(super) sources: Vec<PlaybackCandidate>,
    pub(super) issues: Vec<PlaybackIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlaybackCandidate {
    pub(super) id: String,
    pub(super) source: PlaybackSource,
    pub(super) label: String,
    pub(super) description: Option<String>,
    pub(super) preferred: bool,
    pub(super) addon: Option<String>,
    pub(super) service: Option<String>,
    pub(super) cached: Option<bool>,
    pub(super) resolution: Option<String>,
    pub(super) quality: Option<String>,
    pub(super) container: Option<String>,
    pub(super) video_codec: Option<String>,
    pub(super) size_bytes: Option<u64>,
    pub(super) web_ready: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PlaybackIssue {
    pub(super) source: PlaybackSource,
    pub(super) code: PlaybackIssueCode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum PlaybackIssueCode {
    PartialResults,
    UpstreamUnavailable,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Delivery {
    Direct,
    Hls,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_and_omitted_capabilities_have_identical_defaults() {
        let omitted: ActivationRequest =
            serde_json::from_value(serde_json::json!({"target":{"kind":"movie","tmdbId":1}}))
                .unwrap();
        let empty: PlayerCapabilities = serde_json::from_str("{}").unwrap();
        assert_eq!(
            omitted.capabilities.max_streaming_bitrate,
            empty.max_streaming_bitrate
        );
        assert_eq!(omitted.capabilities.containers, empty.containers);
        assert_eq!(omitted.capabilities.video_codecs, empty.video_codecs);
        assert_eq!(omitted.capabilities.audio_codecs, empty.audio_codecs);
        assert_eq!(omitted.capabilities.hls, empty.hls);
    }
    #[test]
    fn validates_positions_even_when_remote_playback_is_selected() {
        let request = serde_json::json!({"target":{"kind":"movie","tmdbId":1}, "selection":{"kind":"aioStreams","discoveryId":"d","candidateId":"c"}, "startPositionSeconds":-1});
        assert!(serde_json::from_value::<ActivationRequest>(request).is_err());
    }
}
