use super::{JellyfinItemId, JellyfinMediaSourceId, JellyfinUserId};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PublicSystemInfo {
    pub local_address: Option<String>,
    pub server_name: Option<String>,
    pub version: Option<String>,
    pub product_name: Option<String>,
    pub operating_system: Option<String>,
    pub id: Option<String>,
    pub startup_wizard_completed: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    pub id: JellyfinUserId,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ItemQueryResult {
    #[serde(default)]
    pub items: Vec<Item>,
    pub total_record_count: i32,
    pub start_index: i32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub id: JellyfinItemId,
    pub name: Option<String>,
    #[serde(rename = "Type")]
    pub item_type: Option<String>,
    pub media_type: Option<String>,
    pub overview: Option<String>,
    pub production_year: Option<i32>,
    pub run_time_ticks: Option<i64>,
    pub index_number: Option<i32>,
    pub parent_index_number: Option<i32>,
    pub series_id: Option<String>,
    pub series_name: Option<String>,
    pub season_id: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub provider_ids: HashMap<String, String>,
    #[serde(default)]
    pub image_tags: HashMap<String, String>,
    #[serde(default)]
    pub backdrop_image_tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ItemsQuery {
    pub parent_id: Option<String>,
    pub ids: Vec<String>,
    pub search_term: Option<String>,
    pub include_item_types: Vec<ItemType>,
    pub recursive: Option<bool>,
    pub start_index: Option<u32>,
    pub limit: Option<u32>,
    pub sort_by: Vec<ItemSort>,
    pub sort_order: Option<SortOrder>,
    pub is_played: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    Movie,
    Series,
    Season,
    Episode,
    Video,
    BoxSet,
}

impl ItemType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "Movie",
            Self::Series => "Series",
            Self::Season => "Season",
            Self::Episode => "Episode",
            Self::Video => "Video",
            Self::BoxSet => "BoxSet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSort {
    SortName,
    PremiereDate,
    DateCreated,
    DatePlayed,
    Random,
    Runtime,
}

impl ItemSort {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::SortName => "SortName",
            Self::PremiereDate => "PremiereDate",
            Self::DateCreated => "DateCreated",
            Self::DatePlayed => "DatePlayed",
            Self::Random => "Random",
            Self::Runtime => "Runtime",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl SortOrder {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Ascending => "Ascending",
            Self::Descending => "Descending",
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackInfoRequest {
    pub user_id: Option<JellyfinUserId>,
    pub max_streaming_bitrate: Option<i32>,
    pub start_time_ticks: Option<i64>,
    pub audio_stream_index: Option<i32>,
    pub subtitle_stream_index: Option<i32>,
    pub max_audio_channels: Option<i32>,
    pub media_source_id: Option<JellyfinMediaSourceId>,
    pub device_profile: Option<DeviceProfile>,
    pub enable_direct_play: Option<bool>,
    pub enable_direct_stream: Option<bool>,
    pub enable_transcoding: Option<bool>,
    pub allow_video_stream_copy: Option<bool>,
    pub allow_audio_stream_copy: Option<bool>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DeviceProfile {
    pub name: Option<String>,
    pub max_streaming_bitrate: Option<i32>,
    pub max_static_bitrate: Option<i32>,
    #[serde(default)]
    pub direct_play_profiles: Vec<DirectPlayProfile>,
    #[serde(default)]
    pub transcoding_profiles: Vec<TranscodingProfile>,
    #[serde(default)]
    pub container_profiles: Vec<ContainerProfile>,
    #[serde(default)]
    pub codec_profiles: Vec<CodecProfile>,
    #[serde(default)]
    pub subtitle_profiles: Vec<SubtitleProfile>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct DirectPlayProfile {
    pub container: String,
    pub audio_codec: Option<String>,
    pub video_codec: Option<String>,
    #[serde(rename = "Type")]
    pub profile_type: DlnaProfileType,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct TranscodingProfile {
    pub container: String,
    #[serde(rename = "Type")]
    pub profile_type: DlnaProfileType,
    pub video_codec: String,
    pub audio_codec: String,
    pub protocol: MediaStreamProtocol,
    pub max_audio_channels: Option<String>,
    #[serde(default)]
    pub estimate_content_length: bool,
    #[serde(default)]
    pub enable_subtitles_in_manifest: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerProfile {
    #[serde(rename = "Type")]
    pub profile_type: DlnaProfileType,
    pub container: Option<String>,
    pub sub_container: Option<String>,
    #[serde(default)]
    pub conditions: Vec<ProfileCondition>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct CodecProfile {
    #[serde(rename = "Type")]
    pub profile_type: CodecType,
    pub codec: Option<String>,
    pub container: Option<String>,
    #[serde(default)]
    pub conditions: Vec<ProfileCondition>,
    #[serde(default)]
    pub apply_conditions: Vec<ProfileCondition>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ProfileCondition {
    pub condition: ProfileConditionType,
    pub property: ProfileConditionProperty,
    pub value: String,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct SubtitleProfile {
    pub format: Option<String>,
    pub method: SubtitleDeliveryMethod,
    pub language: Option<String>,
    pub container: Option<String>,
}

macro_rules! string_enum {
    ($name:ident { $($variant:tt),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
        pub enum $name { $($variant),+ }
    };
}

string_enum!(DlnaProfileType {
    Audio,
    Video,
    Photo,
    Subtitle,
    Lyric
});
string_enum!(CodecType {
    Video,
    VideoAudio,
    Audio
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MediaStreamProtocol {
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "hls")]
    Hls,
}

string_enum!(SubtitleDeliveryMethod {
    Encode,
    Embed,
    External,
    Hls,
    Drop
});
string_enum!(ProfileConditionType {
    Equals,
    NotEquals,
    LessThanEqual,
    GreaterThanEqual,
    EqualsAny
});
string_enum!(ProfileConditionProperty {
    AudioChannels,
    AudioBitrate,
    AudioProfile,
    Width,
    Height,
    Has64BitOffsets,
    PacketLength,
    VideoBitDepth,
    VideoBitrate,
    VideoFramerate,
    VideoLevel,
    VideoProfile,
    VideoTimestamp,
    IsAnamorphic,
    RefFrames,
    NumAudioStreams,
    NumVideoStreams,
    IsSecondaryAudio,
    VideoCodecTag,
    IsAvc,
    IsInterlaced,
    AudioSampleRate,
    AudioBitDepth,
    VideoRangeType,
    NumStreams,
    VideoRotation,
});

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackInfoResponse {
    #[serde(default)]
    pub media_sources: Vec<MediaSource>,
    pub play_session_id: Option<String>,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MediaSource {
    pub id: Option<JellyfinMediaSourceId>,
    pub name: Option<String>,
    pub protocol: Option<String>,
    #[serde(rename = "Type")]
    pub source_type: Option<String>,
    pub container: Option<String>,
    pub size: Option<i64>,
    pub run_time_ticks: Option<i64>,
    pub bitrate: Option<i32>,
    pub is_remote: Option<bool>,
    pub supports_transcoding: bool,
    pub supports_direct_stream: bool,
    pub supports_direct_play: bool,
    pub transcoding_url: Option<String>,
    pub transcoding_container: Option<String>,
    pub live_stream_id: Option<String>,
    pub default_audio_stream_index: Option<i32>,
    pub default_subtitle_stream_index: Option<i32>,
    #[serde(default)]
    pub required_http_headers: HashMap<String, String>,
    #[serde(default)]
    pub media_streams: Vec<MediaStream>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MediaStream {
    pub index: i32,
    #[serde(rename = "Type")]
    pub stream_type: String,
    pub codec: Option<String>,
    pub profile: Option<String>,
    pub display_title: Option<String>,
    pub language: Option<String>,
    pub title: Option<String>,
    pub bit_rate: Option<i32>,
    pub bit_depth: Option<i32>,
    pub channels: Option<i32>,
    pub sample_rate: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub average_frame_rate: Option<f64>,
    pub is_default: bool,
    pub is_forced: bool,
    pub is_external: bool,
    pub delivery_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    Primary,
    Art,
    Backdrop,
    Banner,
    Logo,
    Thumb,
}

impl ImageType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Art => "Art",
            Self::Backdrop => "Backdrop",
            Self::Banner => "Banner",
            Self::Logo => "Logo",
            Self::Thumb => "Thumb",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImageOptions {
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub quality: Option<u8>,
    pub tag: Option<String>,
    pub image_index: Option<u32>,
}
