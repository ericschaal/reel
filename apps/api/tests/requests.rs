use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    body::{Body, Bytes, to_bytes},
    extract::{OriginalUri, State},
    http::{HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
};
use reel_api::{requests, seerr::Seerr};
use serde_json::{Value, json};
use tower::ServiceExt;

type Calls = Arc<Mutex<Vec<Value>>>;

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
        let upstream = Router::new().fallback(any(mock)).with_state(calls.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock Seerr");
        let url = format!("http://{}", listener.local_addr().expect("mock address"));
        let server = tokio::spawn(async move {
            axum::serve(listener, upstream)
                .await
                .expect("serve mock Seerr");
        });
        Self {
            app: requests::router(Seerr::new(url, "test-key").expect("create Seerr client")),
            calls,
            server,
        }
    }

    async fn send(&self, method: Method, path: &str, body: Value) -> (StatusCode, Value) {
        let request = Request::builder()
            .method(method)
            .uri(path)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .expect("build request");
        let response = self.app.clone().oneshot(request).await.expect("call Reel");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("read response");
        (
            status,
            serde_json::from_slice(&bytes).expect("JSON response"),
        )
    }
}

async fn mock(
    State(calls): State<Calls>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    assert_eq!(headers.get("x-api-key").expect("API key"), "test-key");
    if method == Method::POST && uri.path() == "/api/v1/request" {
        let value: Value = serde_json::from_slice(&body).expect("request JSON");
        calls.lock().expect("lock calls").push(value.clone());
        return match value["mediaId"].as_i64() {
            Some(202) => (
                StatusCode::ACCEPTED,
                Json(json!({"message":"No seasons available to request"})),
            )
                .into_response(),
            Some(403) => (
                StatusCode::FORBIDDEN,
                Json(json!({"message":"Series Quota exceeded."})),
            )
                .into_response(),
            Some(404) => (
                StatusCode::FORBIDDEN,
                Json(json!({"message":"You do not have permission to make movie requests."})),
            )
                .into_response(),
            Some(409) => (
                StatusCode::CONFLICT,
                Json(json!({"message":"Request for this media already exists."})),
            )
                .into_response(),
            _ => (StatusCode::CREATED, Json(json!({"id":77,"status":1}))).into_response(),
        };
    }
    if uri.path() == "/api/v1/settings/radarr" || uri.path() == "/api/v1/settings/sonarr" {
        let port = headers
            .get("host")
            .expect("host")
            .to_str()
            .expect("host text")
            .rsplit(':')
            .next()
            .expect("port")
            .parse::<u16>()
            .expect("numeric port");
        return Json(json!([{"id":0,"hostname":"127.0.0.1","port":port,"baseUrl":"","useSsl":false,"apiKey":"test-key"}])).into_response();
    }
    if uri.path() == "/api/v3/queue/details" {
        return match uri.query() {
            Some("movieId=157") => Json(json!([{"status":"downloading","trackedDownloadStatus":"ok","trackedDownloadState":"downloading"}])).into_response(),
            Some("movieId=158") => Json(json!([])).into_response(),
            Some("seriesId=55") => Json(json!([{"status":"downloading","trackedDownloadState":"downloading","seasonNumbers":[2]}])).into_response(),
            _ => StatusCode::NOT_FOUND.into_response(),
        };
    }
    match uri.path() {
        "/api/v1/service/radarr" | "/api/v1/service/sonarr" => Json(json!([
            {"id":0,"name":"Default","is4k":false,"isDefault":true,"activeProfileId":11},
            {"id":1,"name":"4K","is4k":true,"isDefault":true,"activeProfileId":20}
        ])).into_response(),
        "/api/v1/service/radarr/0" | "/api/v1/service/sonarr/0" => Json(json!({
            "server":{"id":0,"name":"Default","is4k":false,"isDefault":true,"activeProfileId":11},
            "profiles":[{"id":11,"name":"HD"},{"id":12,"name":"UHD"}]
        })).into_response(),
        "/api/v1/movie/1" => Json(json!({"id":1,"title":"Movie","mediaInfo":{
            "id":10,"status":3,"requests":[{"id":7,"status":2,"is4k":false,"createdAt":"2026-09-01T14:00:12.000Z"},{"id":8,"status":1,"is4k":true}]
        }})).into_response(),
        "/api/v1/movie/3" => Json(json!({"id":3,"title":"Downloading","mediaInfo":{
            "id":12,"status":3,"serviceId":0,"externalServiceId":157,
            "requests":[{"id":10,"status":2,"is4k":false}]
        }})).into_response(),
        "/api/v1/movie/4" => Json(json!({"id":4,"title":"Waiting","mediaInfo":{
            "id":13,"status":3,"serviceId":0,"externalServiceId":158,
            "requests":[{"id":11,"status":2,"is4k":false}]
        }})).into_response(),
        "/api/v1/tv/2" => Json(json!({"id":2,"name":"Series","seasons":[
            {"id":20,"name":"Specials","seasonNumber":0},
            {"id":21,"name":"One","seasonNumber":1},
            {"id":22,"name":"Two","seasonNumber":2}
        ],"mediaInfo":{"id":11,"status":4,"seasons":[
            {"seasonNumber":1,"status":5},{"seasonNumber":2,"status":3}
        ],"requests":[{"id":9,"status":2,"is4k":false,"seasons":[
            {"seasonNumber":1,"status":5},{"seasonNumber":2,"status":2}
        ]}]}})).into_response(),
        "/api/v1/tv/3" => Json(json!({"id":3,"name":"Downloading series","seasons":[
            {"id":31,"name":"One","seasonNumber":1},
            {"id":32,"name":"Two","seasonNumber":2}
        ],"mediaInfo":{"id":14,"status":3,"serviceId":0,"externalServiceId":55,
            "seasons":[{"seasonNumber":1,"status":3},{"seasonNumber":2,"status":3}],
            "requests":[{"id":12,"status":2,"is4k":false,"seasons":[
                {"seasonNumber":1,"status":2},{"seasonNumber":2,"status":2}
            ]}]
        }})).into_response(),
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

#[tokio::test]
async fn submits_movie_and_distinct_regular_seasons() {
    let fixture = Fixture::new().await;
    let (status, created) = fixture
        .send(
            Method::POST,
            "/v1/requests",
            json!({"kind":"movie","tmdbId":1,"profileId":12}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(created, json!({"id":77,"requestStatus":"pending"}));
    let (status, _) = fixture
        .send(
            Method::POST,
            "/v1/requests",
            json!({"kind":"series","tmdbId":2,"seasonNumbers":[3,1,3],"profileId":11}),
        )
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        *fixture.calls.lock().expect("lock calls"),
        vec![
            json!({"mediaType":"movie","mediaId":1,"serverId":0,"profileId":12}),
            json!({"mediaType":"tv","mediaId":2,"seasons":[1,3],"serverId":0,"profileId":11}),
        ]
    );
}

#[tokio::test]
async fn invalid_inputs_never_contact_seerr() {
    let fixture = Fixture::new().await;
    for body in [
        json!({"kind":"series","tmdbId":2,"profileId":11}),
        json!({"kind":"series","tmdbId":2,"seasonNumbers":[],"profileId":11}),
        json!({"kind":"series","tmdbId":2,"seasonNumbers":[0],"profileId":11}),
        json!({"kind":"movie","tmdbId":1,"seasonNumbers":[1],"profileId":11}),
        json!({"kind":"movie","tmdbId":0,"profileId":11}),
    ] {
        let (status, error) = fixture.send(Method::POST, "/v1/requests", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(error["error"]["code"], "invalid_request");
    }
    assert!(fixture.calls.lock().expect("lock calls").is_empty());
}

#[tokio::test]
async fn maps_seerr_request_rejections() {
    let fixture = Fixture::new().await;
    for (id, expected_status, code) in [
        (202, StatusCode::CONFLICT, "no_seasons_available"),
        (403, StatusCode::FORBIDDEN, "request_quota_exceeded"),
        (404, StatusCode::FORBIDDEN, "request_forbidden"),
        (409, StatusCode::CONFLICT, "request_exists"),
    ] {
        let (status, body) = fixture
            .send(
                Method::POST,
                "/v1/requests",
                json!({"kind":"movie","tmdbId":id,"profileId":11}),
            )
            .await;
        assert_eq!(status, expected_status);
        assert_eq!(body["error"]["code"], code);
    }
}

#[tokio::test]
async fn exposes_profiles_and_rejects_stale_profile_before_posting() {
    let fixture = Fixture::new().await;
    let (status, profiles) = fixture
        .send(Method::GET, "/v1/requests/profiles/movie", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        profiles,
        json!({
            "serverId":0,"serverName":"Default","defaultProfileId":11,
            "profiles":[{"id":11,"name":"HD"},{"id":12,"name":"UHD"}]
        })
    );
    let (status, body) = fixture
        .send(
            Method::POST,
            "/v1/requests",
            json!({"kind":"movie","tmdbId":1,"profileId":99}),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"]["code"], "invalid_profile");
    assert!(fixture.calls.lock().expect("lock calls").is_empty());
}

#[tokio::test]
async fn reads_movie_and_per_season_state_without_jellyfin_claims() {
    let fixture = Fixture::new().await;
    let (status, movie) = fixture
        .send(Method::GET, "/v1/requests/movie/1", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        movie,
        json!({"kind":"movie","tmdbId":1,"requestStatus":"approved","requestedAt":"2026-09-01T14:00:12.000Z","acquisitionStatus":"processing","seasons":[]})
    );
    let (status, series) = fixture
        .send(Method::GET, "/v1/requests/series/2", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        series,
        json!({"kind":"series","tmdbId":2,"requestStatus":"approved","acquisitionStatus":"partiallyAvailable","seasons":[
            {"seasonNumber":1,"requestStatus":"completed","acquisitionStatus":"available"},
            {"seasonNumber":2,"requestStatus":"approved","acquisitionStatus":"processing"}
        ]})
    );
    assert!(movie.get("availability").is_none());
}

#[tokio::test]
async fn distinguishes_waiting_from_active_radarr_download() {
    let fixture = Fixture::new().await;
    let (status, downloading) = fixture
        .send(Method::GET, "/v1/requests/movie/3", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(downloading["transferStatus"], "downloading");
    let (status, waiting) = fixture
        .send(Method::GET, "/v1/requests/movie/4", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(waiting["transferStatus"], "waiting");
}

#[tokio::test]
async fn attributes_sonarr_queue_to_its_season() {
    let fixture = Fixture::new().await;
    let (status, series) = fixture
        .send(Method::GET, "/v1/requests/series/3", json!({}))
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(series["transferStatus"], "downloading");
    assert!(series["seasons"][0].get("transferStatus").is_none());
    assert_eq!(series["seasons"][1]["transferStatus"], "downloading");
}
