use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reel_api::{
    aiostreams::AioStreams,
    jellyfin::{Item, ItemType, ItemsQuery, Jellyfin, PlaybackInfoRequest},
    media::{SeasonNumber, TmdbId},
    playback::Playback,
    seerr::Seerr,
};
use serde_json::{Value, json};
use tower::ServiceExt;

struct Fixture {
    app: Router,
    jellyfin: Jellyfin,
    api_key: String,
}

impl Fixture {
    fn new() -> Self {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let _ = dotenvy::from_path(manifest_dir.join(".env.local"));
        let base_url = std::env::var("JELLYFIN_BASE_URL")
            .expect("JELLYFIN_BASE_URL must be set for playback E2E tests");
        let api_key = std::env::var("JELLYFIN_API_KEY")
            .expect("JELLYFIN_API_KEY must be set for playback E2E tests");
        let username = std::env::var("JELLYFIN_USERNAME")
            .expect("JELLYFIN_USERNAME must be set for playback E2E tests");
        let jellyfin = Jellyfin::new(base_url, api_key.clone()).expect("create Jellyfin client");
        Self {
            app: Playback::new(jellyfin.clone(), username).router(),
            jellyfin,
            api_key,
        }
    }

    async fn activate(&self, request: Value) -> Value {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/playback/activate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(request.to_string()))
                    .unwrap(),
            )
            .await
            .expect("activate real Jellyfin playback through Reel");
        assert_eq!(response.status(), StatusCode::OK, "activation must succeed");
        let text = response_text(response).await;
        self.assert_no_credentials(&text);
        let descriptor: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(descriptor["source"], "jellyfin");
        assert_eq!(
            descriptor["mediaUrl"],
            format!(
                "/v1/playback/sessions/{}/media",
                descriptor["sessionId"].as_str().unwrap()
            )
        );
        descriptor
    }

    async fn get(&self, path: &str, range: Option<&str>) -> Response {
        let mut request = Request::builder().uri(path);
        if let Some(range) = range {
            request = request.header(header::RANGE, range);
        }
        tokio::time::timeout(
            Duration::from_mins(1),
            self.app
                .clone()
                .oneshot(request.body(Body::empty()).unwrap()),
        )
        .await
        .expect("Jellyfin media request must finish within 60 seconds")
        .expect("proxy real Jellyfin media through Reel")
    }

    async fn playlist(&self, path: &str) -> String {
        let response = self.get(path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let text = response_text(response).await;
        assert!(text.starts_with("#EXTM3U"));
        self.assert_no_credentials(&text);
        text
    }

    fn assert_no_credentials(&self, text: &str) {
        assert!(
            !text.contains(&self.api_key),
            "Reel must not expose the API key"
        );
    }

    fn resource_url(&self, descriptor: &Value, path: &str, item: &Item) -> url::Url {
        let prefix = format!(
            "/v1/playback/sessions/{}/resources/",
            descriptor["sessionId"].as_str().unwrap()
        );
        let encoded = path
            .strip_prefix(&prefix)
            .expect("resource must stay in its Reel session");
        let decoded = URL_SAFE_NO_PAD
            .decode(encoded)
            .expect("decode scoped resource");
        let decoded = std::str::from_utf8(&decoded).unwrap();
        self.assert_no_credentials(decoded);
        let url = url::Url::parse(decoded).unwrap();
        assert!(self.jellyfin.has_same_origin(&url));
        assert!(
            url.path()
                .replace('-', "")
                .to_ascii_lowercase()
                .contains(&format!(
                    "/videos/{}/",
                    item.id.as_str().replace('-', "").to_ascii_lowercase()
                )),
            "resource must belong to the exact selected Jellyfin item"
        );
        assert!(
            !url.query_pairs()
                .any(|(key, _)| key.eq_ignore_ascii_case("apikey")
                    || key.eq_ignore_ascii_case("api_key"))
        );
        url
    }

    async fn stop_transcoding(&self, media_url: &url::Url) {
        let mut url = self.jellyfin.resolve_url("Videos/ActiveEncodings").unwrap();
        for name in ["DeviceId", "PlaySessionId"] {
            let value = media_url
                .query_pairs()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.into_owned())
                .expect("HLS URL must identify the test's encoding session");
            url.query_pairs_mut().append_pair(name, &value);
        }
        let response = reqwest::Client::new()
            .delete(url)
            .header(header::AUTHORIZATION, format!(
                "MediaBrowser Client=\"Reel E2E\", Device=\"Reel API\", DeviceId=\"reel-api\", Version=\"1\", Token=\"{}\"", self.api_key
            ))
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .expect("stop the test's Jellyfin encoding session");
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    async fn items(&self, kind: ItemType) -> Vec<Item> {
        self.jellyfin
            .items(&ItemsQuery {
                include_item_types: vec![kind],
                recursive: Some(true),
                limit: Some(100),
                ..ItemsQuery::default()
            })
            .await
            .expect("discover real Jellyfin playback fixtures")
            .items
    }

    async fn subtitled_episode(&self) -> (Item, Value, Value) {
        for episode in self.items(ItemType::Episode).await {
            let (Some(series_id), Some(season), Some(number)) = (
                episode.series_id.as_deref(),
                episode.parent_index_number,
                episode.index_number,
            ) else {
                continue;
            };
            let series = self
                .jellyfin
                .items(&ItemsQuery {
                    ids: vec![series_id.to_owned()],
                    ..ItemsQuery::default()
                })
                .await
                .expect("read episode's real series");
            let Some(series_tmdb_id) = series.items.first().and_then(provider_tmdb_id) else {
                continue;
            };
            // Jellyfin commonly stores TVDB IDs for episodes. Resolve their
            // canonical TMDb ID from the same Seerr season guide used by Reel.
            let tmdb_id = if let Some(id) = provider_tmdb_id(&episode) {
                id
            } else {
                let seerr = Seerr::new(
                    std::env::var("SEERR_BASE_URL")
                        .expect("SEERR_BASE_URL must be set for playback E2E tests"),
                    std::env::var("SEERR_API_KEY")
                        .expect("SEERR_API_KEY must be set for playback E2E tests"),
                )
                .expect("create Seerr client for canonical episode IDs");
                let guide = seerr
                    .season_details(
                        series_tmdb_id,
                        SeasonNumber::try_from(season).unwrap(),
                        None,
                    )
                    .await
                    .expect("resolve canonical episode ID from Seerr");
                let Some(canonical) = guide
                    .episodes
                    .iter()
                    .find(|item| item.episode_number == number)
                else {
                    continue;
                };
                canonical.id
            };
            let target = json!({"kind":"episode", "tmdbId":tmdb_id, "seriesTmdbId":series_tmdb_id, "seasonNumber":season, "episodeNumber":number});
            let descriptor = self.activate(json!({"target":target, "capabilities":capabilities(), "startPositionSeconds":3.705_481_155_982_247})).await;
            if text_subtitles(&descriptor).count() >= 2
                && text_subtitles(&descriptor)
                    .any(|track| track["isDefault"] != true && track["isForced"] != true)
                && !descriptor["audioTracks"].as_array().unwrap().is_empty()
            {
                return (episode, target, descriptor);
            }
        }
        panic!(
            "Jellyfin playback E2E tests require an episode among the first 100 with series TMDb metadata, audio, and at least two text subtitle tracks including a non-default, non-forced track"
        );
    }
}

async fn response_text(response: Response) -> String {
    let bytes = tokio::time::timeout(
        Duration::from_mins(1),
        to_bytes(response.into_body(), 2_000_000),
    )
    .await
    .expect("media body must finish within 60 seconds")
    .expect("read media body");
    String::from_utf8(bytes.to_vec()).expect("media response must be UTF-8")
}

fn provider_tmdb_id(item: &Item) -> Option<TmdbId> {
    item.provider_ids
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("tmdb"))
        .and_then(|(_, value)| value.parse().ok())
}

#[derive(Default)]
struct RemoteCalls {
    search_queries: Vec<String>,
    search_authenticated: bool,
    media_range: Option<String>,
    media_referer: Option<String>,
    media_authorization: Option<String>,
}

#[derive(Clone)]
struct RemoteState {
    base_url: String,
    calls: Arc<Mutex<RemoteCalls>>,
}

struct RemoteFixture {
    app: Router,
    calls: Arc<Mutex<RemoteCalls>>,
    server: tokio::task::JoinHandle<()>,
}

impl Drop for RemoteFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

impl RemoteFixture {
    async fn new() -> Self {
        Self::with_converter(false).await
    }

    async fn with_converter(convert: bool) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let calls = Arc::new(Mutex::new(RemoteCalls::default()));
        let upstream = Router::new()
            .fallback(any(mock_remote_upstreams))
            .with_state(RemoteState {
                base_url: base_url.clone(),
                calls: calls.clone(),
            });
        let server = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        let jellyfin = Jellyfin::new(&base_url, "jellyfin-secret").unwrap();
        let aiostreams = AioStreams::new(&base_url, "test-uuid", "test-password").unwrap();
        let mut playback =
            Playback::new_with_user_id(jellyfin, "reel-user").with_aiostreams(aiostreams);
        if convert {
            playback = playback.with_stremio(
                reel_api::stremio::Stremio::new(&base_url, Some("http://reel-api:3000")).unwrap(),
            );
        }
        Self {
            app: playback.router(),
            calls,
            server,
        }
    }

    async fn post(&self, path: &str, body: Value) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(path)
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

async fn mock_remote_upstreams(
    State(state): State<RemoteState>,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    match uri.path() {
        "/Items" => Json(json!({
            "Items": [], "TotalRecordCount": 0, "StartIndex": 0
        }))
        .into_response(),
        "/api/v1/search" => mock_aiostreams_search(&state, uri.query(), &headers),
        "/media/first.mp4" => {
            let mut calls = state.calls.lock().unwrap();
            calls.media_range = headers
                .get(header::RANGE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            calls.media_referer = headers
                .get(header::REFERER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            calls.media_authorization = headers
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            (
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_TYPE, "video/mp4"),
                    (header::CONTENT_RANGE, "bytes 0-3/8"),
                    (header::ACCEPT_RANGES, "bytes"),
                ],
                "data",
            )
                .into_response()
        }
        "/media/browser.mkv" | "/media/vc1.mkv" | "/media/browser.mp4" => (
            StatusCode::PARTIAL_CONTENT,
            [
                (header::CONTENT_TYPE, "application/force-download"),
                (header::CONTENT_RANGE, "bytes 0-3/8"),
                (header::ACCEPT_RANGES, "bytes"),
            ],
            "data",
        )
            .into_response(),
        "/media/master.m3u8" => (
            [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\nsegment.ts\n",
        )
            .into_response(),
        "/media/key.bin" => "key".into_response(),
        "/media/error.mp4" => (
            StatusCode::FOUND,
            [(header::LOCATION, "https://slate.elfhosted.com/error/slate.mp4")],
        ).into_response(),
        "/media/segment.ts" => ([(header::CONTENT_TYPE, "video/mp2t")], "segment").into_response(),
        path if path.starts_with("/hlsv2/") && path.ends_with("/master.m3u8") => (
            [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=2000000\nvideo0.m3u8\n",
        ).into_response(),
        path if path.starts_with("/hlsv2/") && path.ends_with("/video0.m3u8") => (
            [(header::CONTENT_TYPE, "application/vnd.apple.mpegurl")],
            "#EXTM3U\n#EXT-X-MAP:URI=\"video0/init.mp4\"\n#EXTINF:4.0,\nvideo0/segment0.m4s\n#EXT-X-ENDLIST\n",
        ).into_response(),
        path if path.starts_with("/hlsv2/") && path.ends_with("/segment0.m4s") => (
            [(header::CONTENT_TYPE, "video/mp4")], "converted-video",
        ).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mock_aiostreams_search(
    state: &RemoteState,
    query: Option<&str>,
    headers: &HeaderMap,
) -> Response {
    let query = query.unwrap_or_default();
    let mut calls = state.calls.lock().unwrap();
    calls.search_queries.push(query.to_owned());
    calls.search_authenticated = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("Basic "));
    drop(calls);

    let value = if query.contains("id=tmdb%3A410") {
        retry_search_response(&state.base_url)
    } else if query.contains("id=tmdb%3A404") || query.contains("id=tmdb%3A5920%3A1%3A1") {
        failed_search_response()
    } else if query.contains("id=tmdb%3A405") {
        non_web_ready_search_response(&state.base_url)
    } else if query.contains("id=tmdb%3A406") {
        browser_search_response(&state.base_url)
    } else if query.contains("id=tmdb%3A407") {
        ranked_search_response(&state.base_url)
    } else {
        default_search_response(&state.base_url)
    };
    Json(value).into_response()
}

fn retry_search_response(base_url: &str) -> Value {
    json!({"success":true,"data":{"results":[
        {"url":format!("{base_url}/media/error.mp4"),"parsedFile":{"container":"mp4","encode":"AVC"}},
        {"url":format!("{base_url}/media/first.mp4"),"parsedFile":{"container":"mp4","encode":"AVC"}}
    ]}})
}

fn failed_search_response() -> Value {
    json!({
        "success": true, "detail": null, "error": null,
        "data": {
            "filtered": 0, "results": [], "statistics": [],
            "errors": [
                {"title": "Provider one", "description": "forbidden"},
                {"title": "Provider two", "description": "authentication failed"}
            ]
        }
    })
}

fn non_web_ready_search_response(base_url: &str) -> Value {
    json!({
        "success": true, "detail": null, "error": null,
        "data": {
            "filtered": 0,
            "results": [{
                "url": format!("{base_url}/media/first.mp4"), "requestHeaders": {},
                "parsedFile": {"container": "mkv"}, "notWebReady": true
            }],
            "statistics": [],
            "errors": [{"title": "Optional provider", "description": "timed out"}]
        }
    })
}

fn browser_search_response(base_url: &str) -> Value {
    json!({
        "success": true, "detail": null, "error": null,
        "data": {
            "results": [{
                "url": format!("{base_url}/media/browser.mkv"), "requestHeaders": {},
                "parsedFile": {"container": "mkv", "encode": "AVC"}, "notWebReady": false
            }],
            "errors": []
        }
    })
}

fn ranked_search_response(base_url: &str) -> Value {
    json!({
        "success": true, "detail": null, "error": null,
        "data": {"results": [
            {"url": format!("{base_url}/media/vc1.mkv"), "requestHeaders": {},
             "parsedFile": {"container": "mkv", "encode": "VC-1"}, "notWebReady": false},
            {"url": format!("{base_url}/media/browser.mkv"), "requestHeaders": {},
             "parsedFile": {"container": "mkv", "encode": "AVC"}, "notWebReady": false},
            {"url": format!("{base_url}/media/browser.mp4"), "requestHeaders": {},
             "parsedFile": {"container": "mp4", "encode": "AVC"}, "notWebReady": false}
        ], "errors": []}
    })
}

fn default_search_response(base_url: &str) -> Value {
    json!({
        "success": true, "detail": null, "error": null,
        "data": {
            "filtered": 2,
            "results": [
                {
                    "url": format!("{base_url}/media/first.mp4"),
                    "requestHeaders": {
                        "Referer": "https://provider.example/",
                        "Authorization": "Bearer upstream-secret", "Range": "bytes=100-200"
                    },
                    "parsedFile": {"container": "mp4", "encode": "AVC", "resolution": "2160p", "quality": "WEB-DL"},
                    "addon": "First addon", "service": "debrid", "cached": true,
                    "size": 2_147_483_648_u64, "duration": 7200, "notWebReady": false,
                    "name": "First formatted source"
                },
                {
                    "url": format!("{base_url}/media/master.m3u8"), "requestHeaders": {},
                    "parsedFile": {"container": "hls", "resolution": "1080p", "quality": "WEB-DL"},
                    "addon": "Second addon", "service": null, "cached": null,
                    "size": null, "duration": null, "notWebReady": false,
                    "name": "Second formatted source"
                },
                {"url": "file:///etc/passwd", "requestHeaders": {}, "parsedFile": null},
                {"url": null, "requestHeaders": {}, "parsedFile": null, "infoHash": "torrent-only"}
            ],
            "statistics": [],
            "errors": [{"title": "Optional provider", "description": "timed out"}]
        }
    })
}

fn capabilities() -> Value {
    json!({
        "containers":["mp4","webm"], "videoCodecs":["h264","vp9"],
        "audioCodecs":["aac","opus"], "hls":true, "maxStreamingBitrate":40_000_000
    })
}

fn capabilities_with_matroska() -> Value {
    json!({
        "containers":["mp4","webm"], "videoCodecs":["h264","vp9"],
        "audioCodecs":["aac","opus"], "hls":true,
        "maxStreamingBitrate":40_000_000,
        "directPlayProfiles":[
            {"container":"mp4","videoCodec":"h264"},
            {"container":"webm","videoCodec":"vp9"},
            {"container":"mkv"},
            {"container":"mkv","videoCodec":"h264"}
        ]
    })
}

fn text_subtitles(descriptor: &Value) -> impl DoubleEndedIterator<Item = &Value> {
    descriptor["subtitleTracks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|track| {
            matches!(
                track["codec"].as_str(),
                Some("subrip" | "srt" | "vtt" | "webvtt")
            )
        })
}

fn resources(playlist: &str) -> impl Iterator<Item = &str> {
    playlist
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn attribute<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    line.split_once(&format!("{name}=\""))?.1.split('"').next()
}

#[tokio::test]
async fn activates_a_real_movie_and_proxies_byte_ranges_without_exposing_credentials() {
    let fixture = Fixture::new();
    let movie = fixture
        .items(ItemType::Movie)
        .await
        .into_iter()
        .find(|item| provider_tmdb_id(item).is_some())
        .expect("Jellyfin must contain a movie with TMDb metadata");
    let descriptor = fixture
        .activate(json!({
            "target":{"kind":"movie", "tmdbId":provider_tmdb_id(&movie).unwrap()},
            "capabilities":capabilities()
        }))
        .await;
    let path = if descriptor["delivery"] == "direct" {
        descriptor["mediaUrl"].as_str().unwrap().to_owned()
    } else {
        // A library may contain only MKV/HEVC movies. Exercise the same
        // session's byte-range proxy against its real static media resource.
        let username = std::env::var("JELLYFIN_USERNAME").unwrap();
        let user = fixture
            .jellyfin
            .users()
            .await
            .expect("list Jellyfin users")
            .into_iter()
            .find(|user| {
                user.name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&username))
            })
            .expect("configured Jellyfin user must exist");
        let info = fixture
            .jellyfin
            .playback_info(&movie.id, &user.id, &PlaybackInfoRequest::default())
            .await
            .expect("read the movie's real media source");
        let source = info
            .media_sources
            .first()
            .expect("movie must have a media source");
        let url = fixture
            .jellyfin
            .direct_play_url(
                &movie.id,
                source.container.as_deref(),
                source.id.as_ref().unwrap(),
                None,
            )
            .unwrap();
        format!(
            "/v1/playback/sessions/{}/resources/{}",
            descriptor["sessionId"].as_str().unwrap(),
            URL_SAFE_NO_PAD.encode(url.as_str())
        )
    };
    let response = fixture.get(&path, Some("bytes=0-3")).await;
    assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
    assert!(
        response.headers()[header::CONTENT_RANGE]
            .to_str()
            .unwrap()
            .starts_with("bytes 0-3/")
    );
    assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
    let bytes = to_bytes(response.into_body(), 4)
        .await
        .expect("read exactly the requested four bytes");
    assert_eq!(bytes.len(), 4);
}

#[tokio::test]
async fn selects_real_episode_tracks_and_proxies_hls_subtitles() {
    let fixture = Fixture::new();
    let (episode, target, initial) = fixture.subtitled_episode().await;
    assert!(initial["selectedSubtitleIndex"].is_null());
    let audio = initial["audioTracks"].as_array().unwrap().last().unwrap();
    // A later, non-default track catches Jellyfin ignoring stream indexes when
    // negotiation omits the media-source ID. Check the actual manifest as well
    // as the descriptor, which echoes the requested indexes.
    let subtitle = text_subtitles(&initial)
        .rev()
        .find(|track| track["isDefault"] != true && track["isForced"] != true)
        .unwrap();
    let selected_index = subtitle["index"].as_i64().unwrap();
    let selected = fixture.activate(json!({
        "target":target, "capabilities":capabilities(), "startPositionSeconds":3.705_481_155_982_247,
        "audioStreamIndex":audio["index"], "subtitleStreamIndex":selected_index
    })).await;
    assert_eq!(selected["delivery"], "hls");
    assert_eq!(selected["selectedAudioIndex"], audio["index"]);
    assert_eq!(selected["selectedSubtitleIndex"], selected_index);
    let master = fixture
        .playlist(selected["mediaUrl"].as_str().unwrap())
        .await;
    let defaults: Vec<_> = master
        .lines()
        .filter(|line| line.contains("TYPE=SUBTITLES") && line.contains("DEFAULT=YES"))
        .collect();
    assert_eq!(
        defaults.len(),
        1,
        "exactly the selected subtitle must be enabled"
    );
    let subtitle_path = attribute(defaults[0], "URI").expect("selected subtitle playlist URI");
    let subtitle_url = fixture.resource_url(&selected, subtitle_path, &episode);
    assert!(
        subtitle_url
            .path()
            .to_ascii_lowercase()
            .contains(&format!("/subtitles/{selected_index}/")),
        "Jellyfin must enable the requested subtitle index"
    );

    let video_path = resources(&master).next().expect("HLS video playlist");
    let video_url = fixture.resource_url(&selected, video_path, &episode);
    assert!(
        video_url
            .query_pairs()
            .any(|(key, value)| key.eq_ignore_ascii_case("AudioStreamIndex")
                && value == audio["index"].to_string()),
        "Jellyfin must select the requested audio index"
    );
    let video_playlist = fixture.playlist(video_path).await;
    let segment_path = resources(&video_playlist)
        .next()
        .expect("HLS video segment");
    fixture.resource_url(&selected, segment_path, &episode);
    let segment = fixture.get(segment_path, None).await;
    let segment_status = segment.status();
    // Consume just one segment so this test does not download the whole video.
    let bytes = tokio::time::timeout(
        Duration::from_mins(1),
        to_bytes(segment.into_body(), 32_000_000),
    )
    .await;
    fixture.stop_transcoding(&video_url).await;
    assert_eq!(segment_status, StatusCode::OK);
    assert!(!bytes.unwrap().unwrap().is_empty());

    let subtitles = fixture.playlist(subtitle_path).await;
    let mut found_cue = false;
    for cue_path in resources(&subtitles).take(10) {
        fixture.resource_url(&selected, cue_path, &episode);
        let response = fixture.get(cue_path, None).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response.headers()[header::CONTENT_TYPE]
                .to_str()
                .unwrap()
                .starts_with("text/vtt")
        );
        let text = response_text(response).await;
        assert!(text.starts_with("WEBVTT"));
        if text.contains(" --> ") {
            found_cue = true;
            break;
        }
    }
    assert!(
        found_cue,
        "the selected subtitle must contain a real cue within its first ten segments"
    );

    let off = fixture
        .activate(json!({"target":target, "capabilities":capabilities(), "subtitleStreamIndex":-1}))
        .await;
    assert_eq!(off["selectedSubtitleIndex"], -1);
    let master = fixture.playlist(off["mediaUrl"].as_str().unwrap()).await;
    assert!(
        !master
            .lines()
            .any(|line| line.contains("TYPE=SUBTITLES") && line.contains("DEFAULT=YES")),
        "turning subtitles off must disable every rendition"
    );
}

#[tokio::test]
async fn discovers_only_safe_direct_sources_in_aiostreams_order() {
    let fixture = RemoteFixture::new().await;
    let (status, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({"target":{"kind":"movie","tmdbId":10}}),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(discovery["sources"].as_array().unwrap().len(), 2);
    assert_eq!(discovery["sources"][0]["label"], "2160p · WEB-DL");
    assert_eq!(discovery["sources"][1]["label"], "1080p · WEB-DL");
    assert_eq!(discovery["sources"][0]["source"], "aioStreams");
    assert_eq!(discovery["issues"][0]["code"], "partialResults");
    let serialized = discovery.to_string();
    assert!(!serialized.contains("/media/"));
    assert!(!serialized.contains("upstream-secret"));
    assert!(!serialized.contains("provider.example"));

    let calls = fixture.calls.lock().unwrap();
    assert!(calls.search_authenticated);
    assert!(calls.search_queries[0].contains("type=movie"));
    assert!(calls.search_queries[0].contains("id=tmdb%3A10"));
    assert!(calls.search_queries[0].contains("requiredFields=url"));
}

#[tokio::test]
async fn distinguishes_failed_providers_from_a_genuine_empty_source_list() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":404});
    let (status, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target.clone()}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(discovery["sources"].as_array().unwrap().is_empty());
    assert_eq!(discovery["issues"][0]["code"], "upstreamUnavailable");

    let (status, activation) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"auto",
                    "discoveryId":discovery["discoveryId"]
                },
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(activation["error"]["code"], "aiostreams_unavailable");
    assert_eq!(fixture.calls.lock().unwrap().search_queries.len(), 1);
}

#[tokio::test]
async fn auto_activation_reuses_the_background_discovery_snapshot() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":10});
    let (_, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target.clone()}))
        .await;
    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"auto",
                    "discoveryId":discovery["discoveryId"]
                },
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "aioStreams");
    assert_eq!(fixture.calls.lock().unwrap().search_queries.len(), 1);
}

#[tokio::test]
async fn distinguishes_incompatible_results_from_failed_providers() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":405});
    let (_, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target.clone()}))
        .await;
    assert_eq!(discovery["sources"].as_array().unwrap().len(), 1);
    assert_eq!(discovery["sources"][0]["webReady"], false);
    assert_eq!(discovery["issues"][0]["code"], "partialResults");

    let (status, activation) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"auto",
                    "discoveryId":discovery["discoveryId"]
                },
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(activation["error"]["code"], "no_compatible_source");
}

#[tokio::test]
async fn uses_browser_direct_play_profiles_to_exclude_incompatible_streams() {
    let fixture = RemoteFixture::new().await;
    let (status, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({
                "target":{"kind":"movie","tmdbId":407},
                "capabilities":capabilities_with_matroska()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let sources = discovery["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 3);
    assert_eq!(sources[0]["container"], "mkv");
    assert_eq!(sources[0]["webReady"], false);
    assert_eq!(sources[1]["container"], "mkv");
    assert_eq!(sources[1]["webReady"], true);
    assert_eq!(sources[2]["container"], "mp4");
    assert_eq!(sources[2]["webReady"], true);
}

#[tokio::test]
async fn serves_direct_media_with_the_selected_container_mime_type() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":406});
    let (_, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({
                "target":target.clone(),
                "capabilities":capabilities_with_matroska()
            }),
        )
        .await;
    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"aioStreams",
                    "discoveryId":discovery["discoveryId"],
                    "candidateId":discovery["sources"][0]["id"]
                },
                "capabilities":capabilities_with_matroska()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let media = fixture
        .get(descriptor["mediaUrl"].as_str().unwrap(), Some("bytes=0-3"))
        .await;
    assert_eq!(media.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(media.headers()[header::CONTENT_TYPE], "video/x-matroska");
}

#[tokio::test]
async fn activates_only_an_opaque_discovered_source_and_forwards_the_client_range() {
    let fixture = RemoteFixture::new().await;
    let (_, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({"target":{"kind":"movie","tmdbId":10}}),
        )
        .await;
    let candidate_id = discovery["sources"][0]["id"].as_str().unwrap();
    let discovery_id = discovery["discoveryId"].as_str().unwrap();
    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":{"kind":"movie","tmdbId":10},
                "selection":{
                    "kind":"aioStreams",
                    "discoveryId":discovery_id,
                    "candidateId":candidate_id
                },
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "aioStreams");
    assert_eq!(descriptor["audioTracks"], json!([]));
    assert_eq!(descriptor["subtitleTracks"], json!([]));
    assert!(!descriptor.to_string().contains("upstream-secret"));

    let media = fixture
        .get(descriptor["mediaUrl"].as_str().unwrap(), Some("bytes=0-3"))
        .await;
    assert_eq!(media.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        to_bytes(media.into_body(), 100).await.unwrap().as_ref(),
        b"data"
    );
    {
        let calls = fixture.calls.lock().unwrap();
        assert_eq!(calls.media_range.as_deref(), Some("bytes=0-3"));
        assert_eq!(
            calls.media_referer.as_deref(),
            Some("https://provider.example/")
        );
        assert_eq!(
            calls.media_authorization.as_deref(),
            Some("Bearer upstream-secret")
        );
    }

    let (forged_status, _) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":{"kind":"movie","tmdbId":10},
                "selection":{
                    "kind":"aioStreams",
                    "discoveryId":discovery_id,
                    "candidateId":"https://attacker.example/video.mp4"
                }
            }),
        )
        .await;
    assert_eq!(forged_status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn maps_exact_episodes_and_rewrites_remote_hls_to_opaque_resources() {
    let fixture = RemoteFixture::new().await;
    let target = json!({
        "kind":"episode", "tmdbId":30, "seriesTmdbId":1399,
        "seasonNumber":1, "episodeNumber":1
    });
    let (_, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target.clone()}))
        .await;
    let candidate_id = discovery["sources"][1]["id"].as_str().unwrap();
    let discovery_id = discovery["discoveryId"].as_str().unwrap();
    let (_, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"aioStreams", "discoveryId":discovery_id,
                    "candidateId":candidate_id
                }
            }),
        )
        .await;
    assert_eq!(descriptor["delivery"], "hls");
    let playlist_response = fixture
        .get(descriptor["mediaUrl"].as_str().unwrap(), None)
        .await;
    let playlist = String::from_utf8(
        to_bytes(playlist_response.into_body(), 10_000)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    assert!(!playlist.contains("segment.ts"));
    assert!(!playlist.contains("key.bin"));
    let segment_url = playlist
        .lines()
        .find(|line| !line.starts_with('#'))
        .unwrap();
    assert_eq!(segment_url.rsplit('/').next().unwrap().len(), 32);
    let segment = fixture.get(segment_url, None).await;
    assert_eq!(
        to_bytes(segment.into_body(), 100).await.unwrap().as_ref(),
        b"segment"
    );
    assert!(fixture.calls.lock().unwrap().search_queries[0].contains("id=tmdb%3A1399%3A1%3A1"));
}

#[tokio::test]
async fn discovers_an_episode_with_its_stremio_imdb_identifier() {
    let fixture = RemoteFixture::new().await;
    let target = json!({
        "kind":"episode", "tmdbId":367_686, "seriesTmdbId":5920,
        "imdbId":"tt1196946", "seasonNumber":1, "episodeNumber":1
    });
    let (status, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target.clone()}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(discovery["sources"].as_array().unwrap().len(), 2);

    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target,
                "selection":{
                    "kind":"auto", "discoveryId":discovery["discoveryId"]
                },
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "aioStreams");

    let calls = fixture.calls.lock().unwrap();
    assert_eq!(calls.search_queries.len(), 1);
    assert!(
        calls.search_queries[0].contains("id=tt1196946%3A1%3A1"),
        "AIOStreams must receive Stremio's IMDb episode identifier"
    );
}

#[tokio::test]
async fn auto_falls_back_to_the_first_direct_source_when_jellyfin_is_not_local() {
    let fixture = RemoteFixture::new().await;
    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":{"kind":"movie","tmdbId":10},
                "capabilities":capabilities()
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "aioStreams");
    assert_eq!(descriptor["container"], "mp4");
}

#[tokio::test]
async fn rejects_error_videos_and_auto_tries_the_next_candidate() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":410});
    let (_, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target}))
        .await;
    assert!(
        fixture.calls.lock().unwrap().media_range.is_none(),
        "discovery must not access media"
    );
    let (status, error) = fixture.post("/v1/playback/activate", json!({
        "target":target, "selection":{"kind":"aioStreams", "discoveryId":discovery["discoveryId"], "candidateId":discovery["sources"][0]["id"]}
    })).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(error["error"]["code"], "source_not_ready");
    let (status, descriptor) = fixture
        .post(
            "/v1/playback/activate",
            json!({
                "target":target, "selection":{"kind":"auto", "discoveryId":discovery["discoveryId"]}
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["source"], "aioStreams");
    let media = fixture
        .get(descriptor["mediaUrl"].as_str().unwrap(), Some("bytes=0-3"))
        .await;
    assert_eq!(response_text(media).await, "data");
}

#[tokio::test]
async fn safari_can_discover_mkv_and_play_scoped_stremio_hls() {
    let fixture = RemoteFixture::with_converter(true).await;
    let target = json!({"kind":"movie","tmdbId":406});
    let safari = json!({"hls":true,"directPlayProfiles":[{"container":"mp4","videoCodec":"h264"}]});
    let (_, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({"target":target,"capabilities":safari}),
        )
        .await;
    assert_eq!(discovery["sources"][0]["container"], "mkv");
    assert_eq!(discovery["sources"][0]["webReady"], true);
    let (status, descriptor) = fixture.post("/v1/playback/activate", json!({
        "target":target,"capabilities":safari,
        "selection":{"kind":"aioStreams","discoveryId":discovery["discoveryId"],"candidateId":discovery["sources"][0]["id"]}
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(descriptor["delivery"], "hls");
    let master = response_text(
        fixture
            .get(descriptor["mediaUrl"].as_str().unwrap(), None)
            .await,
    )
    .await;
    assert!(!master.contains("mediaURL"));
    assert!(!master.contains("hlsv2"));
    let variant_url = master
        .lines()
        .find(|line| line.starts_with("/v1/"))
        .unwrap();
    let variant = response_text(fixture.get(variant_url, None).await).await;
    assert!(variant.contains("#EXT-X-MAP:URI=\"/v1/playback/sessions/"));
    let segment_url = variant
        .lines()
        .find(|line| line.starts_with("/v1/"))
        .unwrap();
    let segment = fixture.get(segment_url, None).await;
    assert_eq!(segment.headers()[header::CONTENT_TYPE], "video/mp4");
    assert_eq!(response_text(segment).await, "converted-video");
    let input_url = format!(
        "/v1/playback/sessions/{}/input",
        descriptor["sessionId"].as_str().unwrap()
    );
    let input = fixture.get(&input_url, Some("bytes=0-3")).await;
    assert_eq!(input.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(input.headers()[header::CONTENT_RANGE], "bytes 0-3/8");
}

#[tokio::test]
async fn cached_discovery_rechecks_capabilities_for_explicit_and_auto_activation() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":406});
    let (_, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({"target":target,"capabilities":capabilities_with_matroska()}),
        )
        .await;
    assert_eq!(discovery["sources"][0]["webReady"], true);
    for selection in [
        json!({"kind":"auto","discoveryId":discovery["discoveryId"]}),
        json!({"kind":"aioStreams","discoveryId":discovery["discoveryId"],"candidateId":discovery["sources"][0]["id"]}),
    ] {
        let (status, _) = fixture
            .post(
                "/v1/playback/activate",
                json!({"target":target,"selection":selection,"capabilities":capabilities()}),
            )
            .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert!(
        fixture.calls.lock().unwrap().media_range.is_none(),
        "incompatible sources must not be probed"
    );
    assert_eq!(fixture.calls.lock().unwrap().search_queries.len(), 1);
}

#[tokio::test]
async fn a_more_capable_player_can_activate_a_previously_incompatible_discovery() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":406});
    let (_, discovery) = fixture
        .post(
            "/v1/playback/sources",
            json!({"target":target,"capabilities":capabilities()}),
        )
        .await;
    assert_eq!(discovery["sources"][0]["webReady"], false);
    let (status, _) = fixture.post("/v1/playback/activate", json!({
        "target":target, "capabilities":capabilities_with_matroska(),
        "selection":{"kind":"aioStreams","discoveryId":discovery["discoveryId"],"candidateId":discovery["sources"][0]["id"]}
    })).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(fixture.calls.lock().unwrap().search_queries.len(), 1);
}

#[tokio::test]
async fn repeated_playlists_keep_resource_urls_stable() {
    let fixture = RemoteFixture::new().await;
    let target = json!({"kind":"movie","tmdbId":10});
    let (_, discovery) = fixture
        .post("/v1/playback/sources", json!({"target":target}))
        .await;
    let (status, descriptor) = fixture.post("/v1/playback/activate", json!({
        "target":target, "selection":{"kind":"aioStreams","discoveryId":discovery["discoveryId"],"candidateId":discovery["sources"][1]["id"]}
    })).await;
    assert_eq!(status, StatusCode::OK);
    let path = descriptor["mediaUrl"].as_str().unwrap();
    assert_eq!(
        response_text(fixture.get(path, None).await).await,
        response_text(fixture.get(path, None).await).await
    );
}
