use std::path::Path;

use reel_api::aiostreams::{AioStreams, SearchTarget};

fn client() -> AioStreams {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env.local"));
    AioStreams::new(
        std::env::var("AIOSTREAMS_BASE_URL")
            .expect("AIOSTREAMS_BASE_URL must be set for AIOStreams E2E tests"),
        std::env::var("AIOSTREAMS_UUID")
            .expect("AIOSTREAMS_UUID must be set for AIOStreams E2E tests"),
        std::env::var("AIOSTREAMS_PASSWORD")
            .expect("AIOSTREAMS_PASSWORD must be set for AIOStreams E2E tests"),
    )
    .expect("create AIOStreams client")
}

#[tokio::test]
#[ignore = "requires apps/api/.env.local and performs a live direct-media range probe"]
async fn discovers_direct_movies_and_exact_episodes_on_the_real_instance() {
    let aiostreams = client();
    let movie = aiostreams
        .search(SearchTarget::Movie {
            tmdb_id: 550.try_into().unwrap(),
            imdb_id: Some("tt0137523".parse().unwrap()),
        })
        .await
        .expect("search the configured AIOStreams instance for a movie");
    assert!(
        !movie.streams.is_empty(),
        "the configured AIOStreams user should return a direct movie source"
    );
    assert!(movie.streams.iter().all(|stream| {
        matches!(stream.url.scheme(), "http" | "https")
            && stream.url.username().is_empty()
            && stream.url.password().is_none()
    }));

    let episode = aiostreams
        .search(SearchTarget::Episode {
            series_tmdb_id: 5920.try_into().unwrap(),
            imdb_id: Some("tt1196946".parse().unwrap()),
            season_number: 1.try_into().unwrap(),
            episode_number: 1.try_into().unwrap(),
        })
        .await
        .expect("search the configured AIOStreams instance for an exact episode");
    assert!(
        episode
            .streams
            .iter()
            .all(|stream| matches!(stream.url.scheme(), "http" | "https"))
    );
    assert!(
        !episode.streams.is_empty(),
        "the reported episode should return direct sources through its IMDb identifier"
    );
    let first = &movie.streams[0];
    let media = client()
        .media_response(first.url.clone(), Some("bytes=0-0"), &first.request_headers)
        .await;
    assert!(
        media.is_ok(),
        "the selected direct source should accept a backend range probe"
    );
    assert!(
        media.unwrap().status().is_success(),
        "the direct source should return a successful HTTP status"
    );
}
