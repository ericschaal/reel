use std::{path::Path, time::Duration};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reel_api::{
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
            Duration::from_secs(60),
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
                    item.id.replace('-', "").to_ascii_lowercase()
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
            let descriptor = self.activate(json!({"target":target, "capabilities":capabilities(), "startPositionSeconds":3.705481155982247})).await;
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
        Duration::from_secs(60),
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

fn capabilities() -> Value {
    json!({
        "containers":["mp4","webm"], "videoCodecs":["h264","vp9"],
        "audioCodecs":["aac","opus"], "hls":true, "maxStreamingBitrate":40000000
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
                source.id.as_deref().unwrap(),
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
        "target":target, "capabilities":capabilities(), "startPositionSeconds":3.705481155982247,
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
        Duration::from_secs(60),
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
