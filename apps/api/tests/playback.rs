use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reel_api::{jellyfin::Jellyfin, playback::Playback};
use serde_json::{Value, json};
use tower::ServiceExt;

#[derive(Default)]
struct Calls {
    paths: Vec<String>,
    range: Option<String>,
}

struct Fixture {
    app: Router,
    calls: Arc<Mutex<Calls>>,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl Fixture {
    async fn new() -> Self {
        let calls = Arc::new(Mutex::new(Calls::default()));
        let upstream = Router::new()
            .fallback(any(mock_jellyfin))
            .with_state(calls.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let jellyfin = Jellyfin::new(base_url, "secret-api-key").unwrap();
        Self {
            app: Playback::with_user_id(jellyfin, "reel-user").router(),
            calls,
            server,
        }
    }

    async fn post(&self, body: Value) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/playback/activate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    async fn get(&self, path: &str, range: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path);
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        self.app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }
}

async fn mock_jellyfin(
    State(calls): State<Arc<Mutex<Calls>>>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    {
        let mut calls = calls.lock().unwrap();
        calls.paths.push(uri.path().to_owned());
        if let Some(range) = headers.get(header::RANGE) {
            calls.range = range.to_str().ok().map(str::to_owned);
        }
    }
    match uri.path() {
        "/Items" if uri.query().is_some_and(|query| query.contains("Movie")) => Json(json!({
            "Items": [{"Id":"jf-movie","Type":"Movie","ProviderIds":{"Tmdb":"10"},"RunTimeTicks":72000000000_i64}],
            "TotalRecordCount":1,
            "StartIndex":0
        }))
        .into_response(),
        "/Items" => Json(json!({
            "Items": [{"Id":"jf-series","Type":"Series","ProviderIds":{"Tmdb":"20"}}],
            "TotalRecordCount":1,
            "StartIndex":0
        }))
        .into_response(),
        "/Shows/jf-series/Seasons" => Json(json!({
            "Items": [{"Id":"jf-season","Type":"Season","IndexNumber":2}],
            "TotalRecordCount":1,
            "StartIndex":0
        }))
        .into_response(),
        "/Shows/jf-series/Episodes" => Json(json!({
            "Items": [{"Id":"jf-episode","Type":"Episode","IndexNumber":3,"ProviderIds":{"Tmdb":"30"}}],
            "TotalRecordCount":1,
            "StartIndex":0
        }))
        .into_response(),
        "/Items/jf-movie/PlaybackInfo" => Json(json!({
            "MediaSources":[{
                "Id":"movie-source","Container":"mp4","RunTimeTicks":72000000000_i64,
                "SupportsDirectPlay":true,"SupportsDirectStream":true,"SupportsTranscoding":true
            }],
            "PlaySessionId":"movie-play"
        }))
        .into_response(),
        "/Items/jf-episode/PlaybackInfo" => Json(json!({
            "MediaSources":[{
                "Id":"episode-source","Container":"mkv","RunTimeTicks":36000000000_i64,
                "SupportsDirectPlay":false,"SupportsDirectStream":true,"SupportsTranscoding":true,
                "TranscodingUrl":"/Videos/jf-episode/master.m3u8?ApiKey=secret-api-key",
                "TranscodingContainer":"ts"
            }],
            "PlaySessionId":"episode-play"
        }))
        .into_response(),
        "/Videos/jf-movie/stream.mp4" => (
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_TYPE, "video/mp4"),
                (header::CONTENT_RANGE, "bytes 0-3/8"),
                (header::ACCEPT_RANGES, "bytes"),
            ],
            "data",
        )
            .into_response(),
        "/Videos/jf-episode/master.m3u8" => (
            [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
            "#EXTM3U\nsegments/0.ts\n",
        )
            .into_response(),
        "/Videos/jf-episode/segments/0.ts" => {
            ([(header::CONTENT_TYPE, "video/mp2t")], "segment").into_response()
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn capabilities() -> Value {
    json!({
        "containers":["mp4","webm"],
        "videoCodecs":["h264","vp9"],
        "audioCodecs":["aac","opus"],
        "hls":true,
        "maxStreamingBitrate":40000000
    })
}

#[tokio::test]
async fn activates_a_movie_and_proxies_byte_ranges_without_exposing_credentials() {
    let fixture = Fixture::new().await;
    let (status, descriptor) = fixture
        .post(json!({
            "target":{"kind":"movie","tmdbId":10},
            "capabilities":capabilities()
        }))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "jellyfin");
    assert_eq!(descriptor["delivery"], "direct");
    let media_url = descriptor["mediaUrl"].as_str().unwrap();
    assert!(!descriptor.to_string().contains("secret-api-key"));

    let response = fixture.get(media_url, Some("bytes=0-3")).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 0-3/8");
    assert_eq!(
        to_bytes(response.into_body(), 100).await.unwrap().as_ref(),
        b"data"
    );
    assert_eq!(
        fixture.calls.lock().unwrap().range.as_deref(),
        Some("bytes=0-3")
    );
}

#[tokio::test]
async fn activates_the_exact_episode_and_scopes_its_hls_resources() {
    let fixture = Fixture::new().await;
    let (status, descriptor) = fixture
        .post(json!({
            "target":{
                "kind":"episode","tmdbId":30,"seriesTmdbId":20,
                "seasonNumber":2,"episodeNumber":3
            },
            "capabilities":capabilities(),
            "startPositionSeconds":90
        }))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["delivery"], "hls");

    let response = fixture
        .get(descriptor["mediaUrl"].as_str().unwrap(), None)
        .await;
    let playlist = String::from_utf8(
        to_bytes(response.into_body(), 10_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(playlist.contains("/v1/playback/sessions/"));
    assert!(!playlist.contains("upstream-only"));
    let resource = playlist
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap();
    let encoded = resource.rsplit('/').next().unwrap();
    let upstream_url = URL_SAFE_NO_PAD.decode(encoded).unwrap();
    assert!(
        !String::from_utf8(upstream_url)
            .unwrap()
            .contains("secret-api-key")
    );
    let segment = fixture.get(resource, None).await;
    assert_eq!(segment.status(), StatusCode::OK);
    assert_eq!(
        to_bytes(segment.into_body(), 100).await.unwrap().as_ref(),
        b"segment"
    );
    assert!(
        fixture
            .calls
            .lock()
            .unwrap()
            .paths
            .contains(&"/Items/jf-episode/PlaybackInfo".to_owned())
    );
}
