use std::path::Path;

use reel_api::jellyfin::{
    DeviceProfile, DirectPlayProfile, DlnaProfileType, Item, ItemQueryResult, ItemType, ItemsQuery,
    Jellyfin, MediaStreamProtocol, PlaybackInfoRequest, SubtitleDeliveryMethod, SubtitleProfile,
    TranscodingProfile,
};

fn client() -> Jellyfin {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env.local"));

    let base_url = std::env::var("JELLYFIN_BASE_URL")
        .expect("JELLYFIN_BASE_URL must be set for Jellyfin E2E tests");
    let api_key = std::env::var("JELLYFIN_API_KEY")
        .expect("JELLYFIN_API_KEY must be set for Jellyfin E2E tests");

    Jellyfin::new(base_url, api_key).expect("create Jellyfin client")
}

async fn playback_context_user_id(jellyfin: &Jellyfin) -> reel_api::jellyfin::JellyfinUserId {
    let username = std::env::var("JELLYFIN_USERNAME")
        .expect("JELLYFIN_USERNAME must be set for Jellyfin E2E tests");
    let users = jellyfin.users().await.expect("list Jellyfin users");

    users
        .into_iter()
        .find(|user| {
            user.name
                .as_deref()
                .is_some_and(|name| name.eq_ignore_ascii_case(&username))
        })
        .map(|user| user.id)
        .expect("JELLYFIN_USERNAME must identify an existing Jellyfin user")
}

fn catalog_query(item_types: Vec<ItemType>, limit: u32) -> ItemsQuery {
    ItemsQuery {
        include_item_types: item_types,
        recursive: Some(true),
        limit: Some(limit),
        ..ItemsQuery::default()
    }
}

async fn playable_item(jellyfin: &Jellyfin) -> Item {
    jellyfin
        .items(&catalog_query(vec![ItemType::Movie, ItemType::Episode], 20))
        .await
        .expect("query playable Jellyfin items")
        .items
        .into_iter()
        .next()
        .expect("the configured Jellyfin server must expose at least one movie or episode")
}

fn prototype_device_profile() -> DeviceProfile {
    DeviceProfile {
        name: Some("Reel E2E".into()),
        max_streaming_bitrate: Some(120_000_000),
        max_static_bitrate: Some(120_000_000),
        direct_play_profiles: vec![
            DirectPlayProfile {
                container: "mp4,m4v".into(),
                audio_codec: Some("aac,mp3,ac3,eac3,opus,flac".into()),
                video_codec: Some("h264,hevc,av1,vp9".into()),
                profile_type: DlnaProfileType::Video,
            },
            DirectPlayProfile {
                container: "mkv,webm".into(),
                audio_codec: Some("aac,mp3,ac3,eac3,opus,flac".into()),
                video_codec: Some("h264,hevc,av1,vp9".into()),
                profile_type: DlnaProfileType::Video,
            },
        ],
        transcoding_profiles: vec![TranscodingProfile {
            container: "ts".into(),
            profile_type: DlnaProfileType::Video,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            protocol: MediaStreamProtocol::Hls,
            max_audio_channels: Some("6".into()),
            estimate_content_length: false,
            enable_subtitles_in_manifest: true,
        }],
        subtitle_profiles: vec![
            SubtitleProfile {
                format: Some("vtt".into()),
                method: SubtitleDeliveryMethod::External,
                language: None,
                container: None,
            },
            SubtitleProfile {
                format: Some("srt".into()),
                method: SubtitleDeliveryMethod::External,
                language: None,
                container: None,
            },
        ],
        ..DeviceProfile::default()
    }
}

#[tokio::test]
async fn authenticates_with_the_service_api_key() {
    let info = client()
        .system_info()
        .await
        .expect("authenticate and read private system information");

    assert!(info.id.is_some(), "Jellyfin should return its server ID");
    assert!(
        info.version.is_some(),
        "Jellyfin should return its server version"
    );
}

#[tokio::test]
async fn browses_details_and_searches_the_real_catalog() {
    let jellyfin = client();
    let user_id = playback_context_user_id(&jellyfin).await;
    let item = playable_item(&jellyfin).await;

    let details = jellyfin
        .item(&item.id, &user_id)
        .await
        .expect("read item details from Jellyfin");
    assert_eq!(details.id, item.id);

    let name = details
        .name
        .as_deref()
        .expect("the selected Jellyfin item should have a name");
    let search = jellyfin
        .search(name, 20)
        .await
        .expect("search the Jellyfin catalog");
    assert!(
        search.items.iter().any(|candidate| candidate.id == item.id),
        "searching for an item's exact name should find that item"
    );

    let latest = jellyfin
        .latest(&user_id, 20)
        .await
        .expect("read latest Jellyfin items");
    assert!(
        latest.len() <= 20,
        "Jellyfin should respect the requested latest-item limit"
    );
}

#[tokio::test]
async fn browses_real_series_hierarchy_when_available() {
    let jellyfin = client();
    let ItemQueryResult { items, .. } = jellyfin
        .items(&catalog_query(vec![ItemType::Series], 20))
        .await
        .expect("query Jellyfin series");

    let Some(series) = items.into_iter().next() else {
        return;
    };
    let seasons = jellyfin
        .seasons(&series.id)
        .await
        .expect("read Jellyfin seasons");
    let Some(season) = seasons.items.first() else {
        return;
    };
    let episodes = jellyfin
        .episodes(&series.id, Some(&season.id))
        .await
        .expect("read Jellyfin episodes");

    assert!(
        episodes
            .items
            .iter()
            .all(|episode| episode.series_id.as_deref() == Some(series.id.as_str())),
        "every returned episode should belong to the requested series"
    );
}

#[tokio::test]
async fn negotiates_playback_with_api_key_authentication() {
    let jellyfin = client();
    let user_id = playback_context_user_id(&jellyfin).await;
    let item = playable_item(&jellyfin).await;
    let request = PlaybackInfoRequest {
        max_streaming_bitrate: Some(120_000_000),
        device_profile: Some(prototype_device_profile()),
        enable_direct_play: Some(true),
        enable_direct_stream: Some(true),
        enable_transcoding: Some(true),
        allow_video_stream_copy: Some(true),
        allow_audio_stream_copy: Some(true),
        ..PlaybackInfoRequest::default()
    };

    let playback = jellyfin
        .playback_info(&item.id, &user_id, &request)
        .await
        .expect("negotiate playback with Jellyfin");

    assert!(
        !playback.media_sources.is_empty(),
        "Jellyfin should return at least one media source for a playable item"
    );
    assert!(
        playback.media_sources.iter().any(|source| {
            source.supports_direct_play
                || source.supports_direct_stream
                || source.supports_transcoding
        }),
        "Jellyfin should expose at least one supported playback method"
    );
}
