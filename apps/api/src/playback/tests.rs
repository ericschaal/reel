use super::proxy::{is_item_video_url, rewrite_playlist};
use super::session::ProxyTarget;
use super::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::Url;

#[tokio::test]
async fn rewrites_jellyfin_playlist_resources_through_the_scoped_session() {
    let base = Url::parse("http://jellyfin/Videos/item/master.m3u8?x=1").unwrap();
    let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"keys/key.bin\"\nmain.m3u8?ApiKey=secret&quality=high\n";
    let playback = Playback::new_with_user_id(
        Jellyfin::new("http://jellyfin", "secret-api-key").unwrap(),
        "user",
    );
    let session = PlaybackSession {
        source: SessionSource::Jellyfin {
            item_id: "item".into(),
        },
        source_url: base.clone(),
        delivery: SessionDelivery::Original,
        resources: RwLock::new(ResourceRegistry::default()),
    };
    let session_id: SessionId = serde_json::from_str("\"session\"").unwrap();
    let rewritten = rewrite_playlist(
        &playback,
        playlist,
        &ProxyTarget::Original(base.clone()),
        &session_id,
        &session,
    )
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

#[tokio::test]
async fn expired_discoveries_cannot_be_activated() {
    let mut playback =
        Playback::new_with_user_id(Jellyfin::new("http://jellyfin", "secret").unwrap(), "user");
    playback.discoveries = Arc::new(Store::new(Duration::ZERO, 1));
    let id = DiscoveryId::generate();
    let target: PlaybackTarget =
        serde_json::from_value(serde_json::json!({"kind":"movie","tmdbId":1})).unwrap();
    playback
        .discoveries
        .insert(
            id.clone(),
            Discovery {
                target: target.clone(),
                candidates: Vec::new(),
                upstream_error_count: 0,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        playback
            .activate_first_discovered_aiostreams(&target, &id, &PlayerCapabilities::default())
            .await,
        Err(Error::DiscoveryNotFound)
    ));
}

#[tokio::test]
async fn converted_sessions_keep_input_playlist_resources_on_the_original_upstream() {
    let playback =
        Playback::new_with_user_id(Jellyfin::new("http://jellyfin", "secret").unwrap(), "user");
    let base: Url = "https://cdn.example/input/master.m3u8".parse().unwrap();
    let session = PlaybackSession {
        source: SessionSource::AioStreams {
            request_headers: axum::http::HeaderMap::new(),
            content_type: None,
        },
        source_url: base.clone(),
        delivery: SessionDelivery::Converted {
            playlist_url: "http://stremio/hlsv2/session/master.m3u8".parse().unwrap(),
        },
        resources: RwLock::new(ResourceRegistry::default()),
    };
    let id = SessionId::generate();
    let rewritten = rewrite_playlist(
        &playback,
        "#EXTM3U\nsegment.ts\n",
        &ProxyTarget::Original(base),
        &id,
        &session,
    )
    .await
    .unwrap();
    let resource_id = super::ids::ResourceId::from_wire(
        rewritten
            .lines()
            .last()
            .unwrap()
            .rsplit('/')
            .next()
            .unwrap()
            .to_owned(),
    );
    let resources = session.resources.read().await;
    let resource = resources.get(&resource_id).unwrap();
    assert!(matches!(resource, ProxyTarget::Original(_)));
    assert_eq!(
        resource.url().as_str(),
        "https://cdn.example/input/segment.ts"
    );
    assert!(matches!(session.output(), ProxyTarget::Converted(_)));
}

#[tokio::test]
async fn invalid_requests_use_the_json_error_contract_before_activation() {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    let app =
        Playback::new_with_user_id(Jellyfin::new("http://jellyfin", "secret").unwrap(), "user")
            .router();
    for (path, request) in [
        (
            "/v1/playback/activate",
            serde_json::json!({"target":{"kind":"movie","tmdbId":1},"startPositionSeconds":-1}),
        ),
        (
            "/v1/playback/activate",
            serde_json::json!({"target":{"kind":"movie","tmdbId":1},"subtitleStreamIndex":-2}),
        ),
        (
            "/v1/playback/sources",
            serde_json::json!({"target":{"kind":"episode","tmdbId":1,"seriesTmdbId":2,"seasonNumber":1,"episodeNumber":0}}),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
                    .header("content-type", "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(body["error"]["code"], "invalid_playback_request");
    }
}

#[test]
fn fallback_logs_each_failure_once_and_redacts_provider_details() {
    use axum::{http::StatusCode, response::IntoResponse};
    use std::{io::Write, sync::Mutex};

    struct LogWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for LogWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let output = Arc::new(Mutex::new(Vec::new()));
    let logs = Arc::clone(&output);
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(move || LogWriter(Arc::clone(&logs)))
        .finish();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    tracing::subscriber::with_default(subscriber, || {
        runtime.block_on(async {
            let playback = Playback::new_with_user_id(
                Jellyfin::new("http://jellyfin", "secret").unwrap(),
                "user",
            );
            let stream = DirectStream {
                url: "https://cdn.example/video.mp4?token=secret"
                    .parse()
                    .unwrap(),
                request_headers: axum::http::HeaderMap::new(),
                label: String::new(),
                description: None,
                addon: None,
                service: None,
                cached: None,
                resolution: None,
                quality: None,
                container: Some("mp4".into()),
                video_codec: Some("h264".into()),
                size_bytes: None,
                duration_seconds: None,
                web_ready: true,
            };
            // Both candidates fail before network access because AIOStreams is not configured.
            let error = playback
                .activate_candidates(
                    [&stream, &stream].into_iter(),
                    0,
                    &PlayerCapabilities::default(),
                )
                .await
                .unwrap_err();
            let _ = error.into_response();
            let _ = Error::Jellyfin(crate::integration::Error::Rejected {
                integration: crate::integration::Integration::Jellyfin,
                status: StatusCode::FORBIDDEN,
                body: "provider-secret-body".into(),
            })
            .into_response();
        })
    });
    let logs = String::from_utf8(output.lock().unwrap().clone()).unwrap();
    assert_eq!(
        logs.matches("playback operation failed").count(),
        3,
        "{logs}"
    );
    assert!(logs.contains("403"));
    assert!(!logs.contains("secret"));
}
