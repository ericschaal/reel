use super::types::PlayerCapabilities;
use crate::{
    aiostreams::DirectStream,
    jellyfin::{
        DeviceProfile, DirectPlayProfile, DlnaProfileType, MediaStreamProtocol,
        SubtitleDeliveryMethod, SubtitleProfile, TranscodingProfile,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Container {
    Mp4,
    Webm,
    Matroska,
    Hls,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VideoCodec {
    H264,
    Hevc,
    Vp8,
    Vp9,
    Av1,
}

pub(super) fn supports_direct_stream(
    stream: &DirectStream,
    capabilities: &PlayerCapabilities,
) -> bool {
    let container = stream.container.as_deref().and_then(normalize_container);
    if stream.url.path().to_ascii_lowercase().ends_with(".m3u8")
        || matches!(container, Some(Container::Hls))
    {
        return capabilities.hls;
    }
    let Some(container) = container else {
        return false;
    };
    let Some(codec) = stream
        .video_codec
        .as_deref()
        .and_then(normalize_video_codec)
    else {
        return false;
    };
    capabilities.supports_video(container, codec)
}

impl PlayerCapabilities {
    fn supports_video(&self, container: Container, codec: VideoCodec) -> bool {
        if self.direct_play_profiles.is_empty() {
            return self
                .containers
                .iter()
                .any(|value| normalize_container(value) == Some(container))
                && self
                    .video_codecs
                    .iter()
                    .any(|value| normalize_video_codec(value) == Some(codec));
        }
        self.direct_play_profiles.iter().any(|profile| {
            normalize_container(&profile.container) == Some(container)
                && profile
                    .video_codec
                    .as_deref()
                    .and_then(normalize_video_codec)
                    == Some(codec)
        })
    }
}

pub(super) fn normalize_container(value: &str) -> Option<Container> {
    match value.trim().to_ascii_lowercase().as_str() {
        "mp4" | "m4v" | "mov" => Some(Container::Mp4),
        "webm" => Some(Container::Webm),
        "mkv" | "matroska" => Some(Container::Matroska),
        "hls" | "m3u8" => Some(Container::Hls),
        _ => None,
    }
}

pub(super) fn normalize_video_codec(value: &str) -> Option<VideoCodec> {
    match value
        .trim()
        .to_ascii_lowercase()
        .replace(['.', '-', '_'], "")
        .as_str()
    {
        "avc" | "h264" | "avc1" => Some(VideoCodec::H264),
        "hevc" | "h265" | "hvc1" | "hev1" => Some(VideoCodec::Hevc),
        "vp8" => Some(VideoCodec::Vp8),
        "vp9" => Some(VideoCodec::Vp9),
        "av1" | "av01" => Some(VideoCodec::Av1),
        _ => None,
    }
}

pub(super) fn direct_content_type(container: &str) -> Option<&'static str> {
    match normalize_container(container) {
        Some(Container::Mp4) => Some("video/mp4"),
        Some(Container::Webm) => Some("video/webm"),
        Some(Container::Matroska) => Some("video/x-matroska"),
        Some(Container::Hls) => Some("application/vnd.apple.mpegurl"),
        _ => None,
    }
}

pub(super) fn web_device_profile(capabilities: &PlayerCapabilities) -> DeviceProfile {
    let supports = |values: &[String], wanted: &str| {
        values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(wanted))
    };
    let mut direct_play_profiles = Vec::new();
    if capabilities.supports_video(Container::Mp4, VideoCodec::H264)
        && supports(&capabilities.audio_codecs, "aac")
    {
        direct_play_profiles.push(DirectPlayProfile {
            container: "mp4,m4v,mov".into(),
            audio_codec: Some(
                if supports(&capabilities.audio_codecs, "mp3") {
                    "aac,mp3"
                } else {
                    "aac"
                }
                .into(),
            ),
            video_codec: Some("h264".into()),
            profile_type: DlnaProfileType::Video,
        });
    }
    let webm_video: Vec<_> = [("vp8", VideoCodec::Vp8), ("vp9", VideoCodec::Vp9)]
        .into_iter()
        .filter(|(_, codec)| capabilities.supports_video(Container::Webm, *codec))
        .map(|(name, _)| name)
        .collect();
    let webm_audio: Vec<_> = ["opus", "vorbis"]
        .into_iter()
        .filter(|codec| supports(&capabilities.audio_codecs, codec))
        .collect();
    if !webm_video.is_empty() && !webm_audio.is_empty() {
        direct_play_profiles.push(DirectPlayProfile {
            container: "webm".into(),
            audio_codec: Some(webm_audio.join(",")),
            video_codec: Some(webm_video.join(",")),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    fn stream(container: &str, codec: Option<&str>) -> DirectStream {
        DirectStream {
            url: "https://cdn.example/video".parse().unwrap(),
            request_headers: HeaderMap::new(),
            label: String::new(),
            description: None,
            addon: None,
            service: None,
            cached: None,
            resolution: None,
            quality: None,
            container: Some(container.into()),
            video_codec: codec.map(str::to_owned),
            size_bytes: None,
            duration_seconds: None,
            web_ready: true,
        }
    }
    #[test]
    fn defaults_reject_unknown_codecs_and_unadvertised_containers() {
        let capabilities = PlayerCapabilities::default();
        assert!(supports_direct_stream(
            &stream("m4v", Some("AVC")),
            &capabilities
        ));
        assert!(!supports_direct_stream(&stream("mp4", None), &capabilities));
        assert!(!supports_direct_stream(
            &stream("mp4", Some("VC-1")),
            &capabilities
        ));
        assert!(!supports_direct_stream(
            &stream("mkv", Some("AVC")),
            &capabilities
        ));
        let no_hls = PlayerCapabilities {
            hls: false,
            ..PlayerCapabilities::default()
        };
        assert!(!supports_direct_stream(&stream("m3u8", None), &no_hls));
    }
    #[test]
    fn detailed_profiles_constrain_both_jellyfin_and_remote_delivery() {
        let capabilities: PlayerCapabilities = serde_json::from_value(serde_json::json!({
            "directPlayProfiles":[{"container":"webm","videoCodec":"vp9"}], "audioCodecs":["opus"]
        }))
        .unwrap();
        assert!(!supports_direct_stream(
            &stream("mp4", Some("h264")),
            &capabilities
        ));
        assert!(supports_direct_stream(
            &stream("webm", Some("vp9")),
            &capabilities
        ));
        let profile = web_device_profile(&capabilities);
        assert_eq!(profile.direct_play_profiles.len(), 1);
        assert_eq!(
            profile.direct_play_profiles[0].video_codec.as_deref(),
            Some("vp9")
        );
        assert_eq!(
            profile.direct_play_profiles[0].audio_codec.as_deref(),
            Some("opus")
        );
    }
}
