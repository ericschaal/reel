use std::path::Path;

use reel_api::seerr::{
    DiscoverMoviesQuery, DiscoverSeriesQuery, MediaType, SearchQuery, Seerr, TimeWindow,
    TrendingMediaType, TrendingQuery,
};

fn client() -> Seerr {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env.local"));

    let base_url =
        std::env::var("SEERR_BASE_URL").expect("SEERR_BASE_URL must be set for Seerr E2E tests");
    let api_key =
        std::env::var("SEERR_API_KEY").expect("SEERR_API_KEY must be set for Seerr E2E tests");

    Seerr::new(base_url, api_key).expect("create Seerr client")
}

#[tokio::test]
async fn browses_the_real_discovery_catalog() {
    let seerr = client();

    let trending = seerr
        .trending(&TrendingQuery {
            media_type: Some(TrendingMediaType::All),
            time_window: Some(TimeWindow::Week),
            ..TrendingQuery::default()
        })
        .await
        .expect("read trending media from Seerr");
    assert!(
        !trending.results.is_empty(),
        "Seerr should return at least one trending item"
    );

    let movies = seerr
        .movies(&DiscoverMoviesQuery::default())
        .await
        .expect("read popular movies from Seerr");
    assert!(
        movies
            .results
            .iter()
            .any(|item| item.media_type == MediaType::Movie),
        "Seerr should return at least one popular movie"
    );

    let series = seerr
        .series(&DiscoverSeriesQuery::default())
        .await
        .expect("read popular series from Seerr");
    assert!(
        series
            .results
            .iter()
            .any(|item| item.media_type == MediaType::Tv),
        "Seerr should return at least one popular series"
    );
}

#[tokio::test]
async fn searches_and_reads_real_movie_and_series_details() {
    let seerr = client();
    let movies = seerr
        .movies(&DiscoverMoviesQuery::default())
        .await
        .expect("read movies to select a detail fixture");
    let movie = movies
        .results
        .into_iter()
        .find(|item| item.media_type == MediaType::Movie)
        .expect("the Seerr movie catalog must not be empty");
    let movie_title = movie
        .display_title()
        .expect("the selected movie must have a title")
        .to_owned();

    let search = seerr
        .search(&SearchQuery {
            query: movie_title,
            page: Some(1),
            language: None,
        })
        .await
        .expect("search the Seerr catalog");
    assert!(
        search
            .results
            .iter()
            .any(|item| item.id == movie.id && item.media_type == MediaType::Movie),
        "searching for a movie's exact title should find that movie"
    );

    let movie_details = seerr
        .movie(movie.id, None)
        .await
        .expect("read movie details from Seerr");
    assert_eq!(movie_details.id, movie.id);

    let series = seerr
        .series(&DiscoverSeriesQuery::default())
        .await
        .expect("read series to select a detail fixture")
        .results
        .into_iter()
        .find(|item| item.media_type == MediaType::Tv)
        .expect("the Seerr series catalog must not be empty");
    let series_details = seerr
        .series_details(series.id, None)
        .await
        .expect("read series details from Seerr");
    assert_eq!(series_details.id, series.id);
    let season = series_details
        .seasons
        .iter()
        .find(|season| season.season_number > 0 && season.episode_count.unwrap_or(0) > 0)
        .expect("the selected series should contain a regular season");
    let season_details = seerr
        .season_details(series.id, season.season_number, None)
        .await
        .expect("read season and episode details from Seerr");
    assert_eq!(season_details.season_number, season.season_number);
    assert!(
        season_details
            .episodes
            .iter()
            .all(|episode| episode.season_number == season.season_number),
        "every episode should belong to the requested season"
    );
}

#[tokio::test]
async fn browses_real_genre_studio_and_network_categories() {
    let seerr = client();

    let movie_genres = seerr
        .movie_genres(None)
        .await
        .expect("read movie genre slider from Seerr");
    assert!(!movie_genres.is_empty(), "Seerr should return movie genres");

    let series_genres = seerr
        .series_genres(None)
        .await
        .expect("read series genre slider from Seerr");
    assert!(
        !series_genres.is_empty(),
        "Seerr should return series genres"
    );

    let disney = seerr
        .movies_by_studio(2, Some(1), None)
        .await
        .expect("browse Disney movies through Seerr");
    assert_eq!(disney.studio.as_ref().map(|studio| studio.id), Some(2));

    let netflix = seerr
        .series_by_network(213, Some(1), None)
        .await
        .expect("browse Netflix series through Seerr");
    assert_eq!(
        netflix.network.as_ref().map(|network| network.id),
        Some(213)
    );
}
