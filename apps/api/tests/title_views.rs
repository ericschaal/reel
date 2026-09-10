//! Offline contract tests exercise the real HTTP router and mock upstreams.
use axum::{
    Json, Router,
    body::{Body, to_bytes},
    extract::{OriginalUri, State},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use reel_api::{app, catalogue::Catalogue, jellyfin::Jellyfin, seerr::Seerr};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower::ServiceExt;

type Calls = Arc<Mutex<HashMap<String, usize>>>;
struct Fixture {
    app: Router,
    calls: Calls,
    server: tokio::task::JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}
impl Fixture {
    async fn new() -> Self {
        let calls = Calls::default();
        let upstream = Router::new().fallback(get(mock)).with_state(calls.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, upstream).await.unwrap();
        });
        Self {
            app: app(Catalogue::new(
                Seerr::new(&url, "test").unwrap(),
                Jellyfin::new(&url, "test").unwrap(),
            )),
            calls,
            server,
        }
    }
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    fn count(&self, path: &str) -> usize {
        *self.calls.lock().unwrap().get(path).unwrap_or(&0)
    }
}
async fn mock(State(calls): State<Calls>, OriginalUri(uri): OriginalUri) -> Response {
    *calls.lock().unwrap().entry(uri.path().into()).or_default() += 1;
    tokio::time::sleep(Duration::from_millis(10)).await;
    let value = match uri.path() {
        "/api/v1/movie/1" => {
            json!({"id":1,"title":"Movie","runtime":124,"overview":"Long synopsis"})
        }
        "/api/v1/tv/2" => {
            json!({"id":2,"name":"Series","numberOfSeasons":2,"seasons":[{"id":20,"seasonNumber":1,"name":"Season 1","episodeCount":1}]})
        }
        "/api/v1/tv/2/season/1" => {
            json!({"id":20,"name":"Season 1","seasonNumber":1,"episodes":[{"id":21,"name":"Pilot","seasonNumber":1,"episodeNumber":1}]})
        }
        "/api/v1/tv/3" => {
            json!({"id":3,"name":"Partial Series","seasons":[{"id":30,"seasonNumber":1,"name":"Season 1","episodeCount":1}]})
        }
        "/api/v1/movie/500" => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        "/api/v1/discover/trending" => json!({"page":1,"totalPages":1,"totalResults":3,"results":[
            {"id":1,"mediaType":"movie","title":"Movie"},
            {"id":2,"mediaType":"tv","name":"Series"},
            {"id":404,"mediaType":"movie","title":"Missing metadata"}
        ]}),
        "/api/v1/discover/movies" => {
            json!({"page":1,"totalPages":1,"totalResults":1,"results":[{"id":1,"mediaType":"movie","title":"Movie","overview":"Do not return this on cards"}]})
        }
        // Unavailable Jellyfin deliberately distinguishes unknown from not local.
        "/Items" => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    Json(value).into_response()
}

#[tokio::test]
async fn summary_batch_is_compact_ordered_deduplicated_and_has_per_title_errors() {
    let fixture = Fixture::new().await;
    let (status, body) = fixture.get("/v1/titles/summaries?ids=tmdb:series:2,tmdb:movie:1,tmdb:series:2,tmdb:movie:404,tmdb:movie:500&language=en").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "items":[{"kind":"series","id":"tmdb:series:2","numberOfSeasons":2},{"kind":"movie","id":"tmdb:movie:1","runtimeMinutes":124}],
            "issues":[{"id":"tmdb:movie:404","code":"media_not_found"},{"id":"tmdb:movie:500","code":"catalogue_unavailable"}]
        })
    );
    assert_eq!(fixture.count("/api/v1/tv/2"), 1);
    assert_eq!(fixture.count("/api/v1/tv/2/season/1"), 0);
    assert_eq!(fixture.count("/Items"), 0);
}

#[tokio::test]
async fn summary_and_detail_share_metadata_but_never_cache_availability_in_metadata() {
    let fixture = Fixture::new().await;
    let (summary, details) = tokio::join!(
        fixture.get("/v1/titles/movie/1?view=summary&language=en"),
        fixture.get("/v1/titles/movie/1?language=en"),
    );
    assert_eq!(summary.0, StatusCode::OK);
    assert_eq!(
        summary.1,
        json!({"kind":"movie","id":"tmdb:movie:1","runtimeMinutes":124})
    );
    assert_eq!(details.0, StatusCode::OK);
    assert_eq!(details.1["overview"], "Long synopsis");
    assert_eq!(details.1["availability"], "unknown");
    assert_eq!(details.1["issues"][0]["source"], "jellyfin");
    assert!(details.1.get("localCopy").is_none());
    assert_eq!(fixture.count("/api/v1/movie/1"), 1);
    fixture.get("/v1/titles/movie/1?language=en").await;
    assert_eq!(fixture.count("/api/v1/movie/1"), 1);
    assert_eq!(
        fixture.count("/Items"),
        2,
        "failed availability is retried independently"
    );
    fixture
        .get("/v1/titles/movie/1?view=summary&language=fr")
        .await;
    assert_eq!(
        fixture.count("/api/v1/movie/1"),
        2,
        "cache must distinguish locales"
    );
}

#[tokio::test]
async fn series_episode_guide_is_an_explicit_expansion() {
    let fixture = Fixture::new().await;
    let (_, details) = fixture.get("/v1/titles/series/2").await;
    assert_eq!(details["kind"], "series");
    assert_eq!(details["availability"], "episodeBased");
    assert!(details["initialSeason"].is_null());
    assert_eq!(fixture.count("/api/v1/tv/2/season/1"), 0);
    assert_eq!(fixture.count("/Items"), 0);
    let (status, details) = fixture
        .get("/v1/titles/series/2?include=initialSeason")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        details["initialSeason"]["episodes"][0]["id"],
        "tmdb:episode:21"
    );
    assert_eq!(
        details["initialSeason"]["episodes"][0]["availability"],
        "unknown"
    );
    assert_eq!(fixture.count("/api/v1/tv/2"), 1);
    assert_eq!(fixture.count("/api/v1/tv/2/season/1"), 1);
    let (status, partial) = fixture
        .get("/v1/titles/series/3?include=initialSeason")
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(partial["initialSeason"].is_null());
    assert_eq!(partial["issues"][0]["source"], "seerr");
}

#[tokio::test]
async fn invalid_queries_are_rejected_before_any_upstream_work() {
    let fixture = Fixture::new().await;
    let too_many = std::iter::repeat_n("tmdb:movie:1", 41)
        .collect::<Vec<_>>()
        .join(",");
    for path in [
        "/v1/titles/summaries".to_owned(),
        "/v1/titles/summaries?ids=".to_owned(),
        "/v1/titles/summaries?ids=tmdb:movie:1,tmdb:episode:2".to_owned(),
        "/v1/titles/summaries?ids=tmdb:movie:01".to_owned(),
        "/v1/titles/summaries?ids=tmdb:movie:0".to_owned(),
        format!("/v1/titles/summaries?ids={too_many}"),
        "/v1/titles/movie/1?include=initialSeason".to_owned(),
        "/v1/titles/series/2?view=summary&include=initialSeason".to_owned(),
        "/v1/titles/series/2?view=anything".to_owned(),
        "/v1/titles/series/2?include=everything".to_owned(),
        "/v1/titles/series/2?typo=en".to_owned(),
    ] {
        let (status, body) = fixture.get(&path).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
        assert_eq!(body["error"]["code"], "invalid_query");
    }
    assert!(fixture.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn collection_cards_omit_synopsis_and_provider_ids_and_disclose_unknown_availability() {
    let fixture = Fixture::new().await;
    let (status, body) = fixture
        .get("/v1/catalogue/collections/popular-movies")
        .await;
    assert_eq!(status, StatusCode::OK);
    let card = &body["items"][0];
    assert_eq!(card["availability"], "unknown");
    assert_eq!(card["runtimeMinutes"], 124);
    assert!(card["numberOfSeasons"].is_null());
    assert!(card.get("overview").is_none());
    assert!(card.get("localCopy").is_none());
    assert_eq!(body["issues"][0]["source"], "jellyfin");
    assert_eq!(
        fixture.count("/api/v1/movie/1"),
        1,
        "server completes card facts in the collection response"
    );
}

#[tokio::test]
async fn rail_contains_complete_card_facts_and_keeps_cards_when_enrichment_fails() {
    let fixture = Fixture::new().await;
    let (status, body) = fixture.get("/v1/catalogue/discover/rails/trending").await;
    assert_eq!(status, StatusCode::OK);
    let cards = body["section"]["items"].as_array().unwrap();
    assert_eq!(cards.len(), 3);
    assert_eq!(cards[0]["id"], "tmdb:movie:1");
    assert_eq!(cards[0]["runtimeMinutes"], 124);
    assert_eq!(cards[1]["id"], "tmdb:series:2");
    assert_eq!(cards[1]["numberOfSeasons"], 2);
    assert_eq!(cards[2]["id"], "tmdb:movie:404");
    assert!(cards[2]["runtimeMinutes"].is_null());
    assert!(
        body["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["source"] == "seerr" && issue["sectionId"] == "trending")
    );
    assert_eq!(fixture.count("/api/v1/tv/2/season/1"), 0);
    fixture
        .get("/v1/catalogue/collections/popular-movies")
        .await;
    assert_eq!(
        fixture.count("/api/v1/movie/1"),
        1,
        "rails and collections share metadata cache"
    );
}
