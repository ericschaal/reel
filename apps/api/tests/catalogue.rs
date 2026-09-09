use std::path::Path;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use reel_api::{
    app,
    catalogue::{Catalogue, CatalogueItem, CatalogueResponse, CollectionResponse, Surface},
    jellyfin::Jellyfin,
    seerr::Seerr,
};
use tower::ServiceExt;

fn clients() -> (Seerr, Jellyfin) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let _ = dotenvy::from_path(manifest_dir.join(".env.local"));

    let seerr = Seerr::new(
        std::env::var("SEERR_BASE_URL").expect("SEERR_BASE_URL must be set for catalogue tests"),
        std::env::var("SEERR_API_KEY").expect("SEERR_API_KEY must be set for catalogue tests"),
    )
    .expect("create Seerr client");
    let jellyfin = Jellyfin::new(
        std::env::var("JELLYFIN_BASE_URL")
            .expect("JELLYFIN_BASE_URL must be set for catalogue tests"),
        std::env::var("JELLYFIN_API_KEY")
            .expect("JELLYFIN_API_KEY must be set for catalogue tests"),
    )
    .expect("create Jellyfin client");

    (seerr, jellyfin)
}

async fn get(application: axum::Router, path: &str) -> axum::response::Response {
    application
        .oneshot(
            Request::builder()
                .uri(path)
                .body(Body::empty())
                .expect("build catalogue request"),
        )
        .await
        .expect("serve catalogue request")
}

async fn get_catalogue(path: &str) -> CatalogueResponse {
    let (seerr, jellyfin) = clients();
    let response = get(app(Catalogue::new(seerr, jellyfin)), path).await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read catalogue response");
    serde_json::from_slice(&body).expect("decode Reel catalogue response")
}

#[tokio::test]
async fn serves_seerr_catalogue_when_jellyfin_enrichment_is_unavailable() {
    let (seerr, _) = clients();
    let unavailable_jellyfin =
        Jellyfin::new("http://127.0.0.1:1", "unused").expect("create unavailable Jellyfin client");
    let response = get(
        app(Catalogue::new(seerr, unavailable_jellyfin)),
        "/v1/catalogue/movies",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read catalogue response");
    let catalogue: CatalogueResponse =
        serde_json::from_slice(&body).expect("decode Reel catalogue response");
    assert!(
        catalogue
            .issues
            .iter()
            .any(|issue| issue.source == reel_api::catalogue::CatalogueSource::Jellyfin),
        "the response should disclose unavailable Jellyfin enrichment"
    );
    assert_surface(&catalogue, Surface::Movies);
}

#[tokio::test]
async fn serves_normalized_catalogue_surfaces_from_real_integrations() {
    let (discover, movies, series) = tokio::join!(
        get_catalogue("/v1/catalogue/discover?language=en"),
        get_catalogue("/v1/catalogue/movies?language=en"),
        get_catalogue("/v1/catalogue/series?language=en"),
    );

    assert_surface(&discover, Surface::Discover);
    assert_surface(&movies, Surface::Movies);
    assert_surface(&series, Surface::Series);
}

#[tokio::test]
async fn catalogue_previews_link_to_reel_collections() {
    let (discover, movies, series) = tokio::join!(
        get_catalogue("/v1/catalogue/discover?language=en"),
        get_catalogue("/v1/catalogue/movies?language=en"),
        get_catalogue("/v1/catalogue/series?language=en"),
    );
    let sections = discover
        .sections
        .iter()
        .chain(&movies.sections)
        .chain(&series.sections);
    let links: std::collections::HashSet<_> = sections
        .filter_map(|section| section.href.as_deref())
        .collect();

    for id in [
        "trending",
        "trending-movies",
        "trending-series",
        "popular-movies",
        "popular-series",
    ] {
        assert!(
            links.contains(format!("/v1/catalogue/collections/{id}?language=en").as_str()),
            "missing collection link for {id}"
        );
    }
}

#[tokio::test]
async fn collection_links_start_at_page_one_and_continue_with_an_opaque_cursor() {
    let catalogue = get_catalogue("/v1/catalogue/movies?language=en").await;
    let popular = catalogue
        .sections
        .iter()
        .find(|section| section.id == "popular-movies")
        .expect("movies catalogue should include popular movies");
    let href = popular
        .href
        .as_deref()
        .expect("popular movies collection link");

    let first: CollectionResponse = get_json(href).await;
    assert_eq!(first.id, "popular-movies");
    assert_eq!(first.items, popular.items);
    assert!(first.total_results >= first.items.len() as u64);

    let next = first
        .next
        .as_deref()
        .expect("popular movies should have another page");
    assert!(next.starts_with("/v1/catalogue/collections/popular-movies?"));
    assert!(next.contains("language=en"));
    assert!(next.contains("cursor="));
    assert!(
        !next.contains("page="),
        "Seerr page numbers must remain private"
    );

    let second: CollectionResponse = get_json(next).await;
    assert_eq!(second.id, first.id);
    assert_ne!(second.items, first.items);
}

#[tokio::test]
async fn genre_cards_link_to_paginated_movie_collections() {
    let catalogue = get_catalogue("/v1/catalogue/movies?language=en").await;
    let genre = catalogue
        .sections
        .iter()
        .find(|section| section.id == "movie-genres")
        .and_then(|section| section.items.first())
        .and_then(|item| match item {
            CatalogueItem::Category(genre) => Some(genre),
            _ => None,
        })
        .expect("movies catalogue should contain a genre card");

    assert!(
        genre
            .href
            .starts_with("/v1/catalogue/collections/movie-genres/")
    );
    assert!(genre.href.ends_with("?language=en"));
    let collection: CollectionResponse = get_json(&genre.href).await;
    assert_eq!(collection.title, genre.title);
    assert!(!collection.items.is_empty());
    assert!(
        collection
            .items
            .iter()
            .all(|item| matches!(item, CatalogueItem::Movie(_)))
    );
    let second: CollectionResponse = get_json(
        collection
            .next
            .as_deref()
            .expect("movie genre should have another page"),
    )
    .await;
    assert_ne!(second.items, collection.items);
}

#[tokio::test]
async fn genre_cards_link_to_paginated_series_collections() {
    let catalogue = get_catalogue("/v1/catalogue/series?language=fr-FR").await;
    let genre = catalogue
        .sections
        .iter()
        .find(|section| section.id == "series-genres")
        .and_then(|section| section.items.first())
        .and_then(|item| match item {
            CatalogueItem::Category(genre) => Some(genre),
            _ => None,
        })
        .expect("series catalogue should contain a genre card");

    assert!(
        genre
            .href
            .starts_with("/v1/catalogue/collections/series-genres/")
    );
    assert!(genre.href.ends_with("?language=fr-FR"));
    let collection: CollectionResponse = get_json(&genre.href).await;
    assert!(!collection.items.is_empty());
    assert!(
        collection
            .items
            .iter()
            .all(|item| matches!(item, CatalogueItem::Series(_)))
    );
    assert!(
        collection
            .next
            .as_deref()
            .is_some_and(|next| next.contains("language=fr-FR"))
    );
}

#[tokio::test]
async fn rejects_invalid_collection_cursors_without_contacting_upstreams() {
    let (seerr, jellyfin) = clients();
    let response = get(
        app(Catalogue::new(seerr, jellyfin)),
        "/v1/catalogue/collections/popular-movies?language=en&cursor=tampered",
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read error response");
    let body: serde_json::Value = serde_json::from_slice(&body).expect("decode error response");
    assert_eq!(body["error"]["code"], "invalid_cursor");
    assert_eq!(
        body["error"]["message"],
        "The continuation cursor is invalid"
    );
}

#[tokio::test]
async fn studio_cards_open_paginated_movie_collections() {
    let catalogue = get_catalogue("/v1/catalogue/movies?language=en").await;
    let studios = category_cards(&catalogue, "studios");
    assert_eq!(
        studios.iter().map(|studio| studio.id).collect::<Vec<_>>(),
        [2, 127928, 34, 174, 33, 4, 3, 521, 420, 9993, 41077]
    );
    assert_eq!(
        studios
            .iter()
            .map(|studio| studio.title.as_str())
            .collect::<Vec<_>>(),
        [
            "Disney",
            "20th Century Studios",
            "Sony Pictures",
            "Warner Bros. Pictures",
            "Universal",
            "Paramount",
            "Pixar",
            "Dreamworks",
            "Marvel Studios",
            "DC",
            "A24",
        ]
    );
    let studio = category_card(&catalogue, "studios", "Disney");

    assert_eq!(
        studio.href,
        "/v1/catalogue/collections/studios/2?language=en"
    );
    assert_eq!(
        studio.images,
        [
            "https://image.tmdb.org/t/p/w780_filter(duotone,ffffff,bababa)/wdrCwmRnLFJhEoH8GSfymY85KHT.png"
        ]
    );
    let collection: CollectionResponse = get_json(&studio.href).await;
    assert!(!collection.title.is_empty());
    assert!(
        collection
            .items
            .iter()
            .all(|item| matches!(item, CatalogueItem::Movie(_)))
    );
    assert!(
        collection.next.is_some(),
        "Disney movies should be pageable"
    );
}

#[tokio::test]
async fn network_cards_open_paginated_series_collections() {
    let catalogue = get_catalogue("/v1/catalogue/series?language=en").await;
    let networks = category_cards(&catalogue, "networks");
    assert_eq!(
        networks
            .iter()
            .map(|network| network.id)
            .collect::<Vec<_>>(),
        [
            213, 2739, 1024, 2552, 453, 49, 4353, 2, 19, 359, 174, 67, 318, 71, 6, 16, 4330, 4, 56,
            80, 13, 3353,
        ]
    );
    assert_eq!(
        networks
            .iter()
            .map(|network| network.title.as_str())
            .collect::<Vec<_>>(),
        [
            "Netflix",
            "Disney+",
            "Prime Video",
            "Apple TV+",
            "Hulu",
            "HBO",
            "Discovery+",
            "ABC",
            "FOX",
            "Cinemax",
            "AMC",
            "Showtime",
            "Starz",
            "The CW",
            "NBC",
            "CBS",
            "Paramount+",
            "BBC One",
            "Cartoon Network",
            "Adult Swim",
            "Nickelodeon",
            "Peacock",
        ]
    );
    let network = category_card(&catalogue, "networks", "Netflix");

    assert_eq!(
        network.href,
        "/v1/catalogue/collections/networks/213?language=en"
    );
    let collection: CollectionResponse = get_json(&network.href).await;
    assert!(!collection.title.is_empty());
    assert!(
        collection
            .items
            .iter()
            .all(|item| matches!(item, CatalogueItem::Series(_)))
    );
    assert!(
        collection.next.is_some(),
        "Netflix series should be pageable"
    );
}

fn category_card<'a>(
    catalogue: &'a CatalogueResponse,
    section_id: &str,
    title: &str,
) -> &'a reel_api::catalogue::CategoryCard {
    category_cards(catalogue, section_id)
        .into_iter()
        .find(|category| category.title == title)
        .expect("catalogue should contain the requested category card")
}

fn category_cards<'a>(
    catalogue: &'a CatalogueResponse,
    section_id: &str,
) -> Vec<&'a reel_api::catalogue::CategoryCard> {
    catalogue
        .sections
        .iter()
        .find(|section| section.id == section_id)
        .map(|section| {
            section
                .items
                .iter()
                .filter_map(|item| match item {
                    CatalogueItem::Category(category) => Some(category),
                    _ => None,
                })
                .collect()
        })
        .expect("catalogue should contain the requested category section")
}

async fn get_json<T: serde::de::DeserializeOwned>(path: &str) -> T {
    let (seerr, jellyfin) = clients();
    let response = get(app(Catalogue::new(seerr, jellyfin)), path).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("read JSON response");
    serde_json::from_slice(&body).expect("decode JSON response")
}

fn assert_surface(catalogue: &CatalogueResponse, expected: Surface) {
    assert_eq!(catalogue.surface, expected);
    assert!(
        !catalogue.sections.is_empty(),
        "a catalogue surface should contain at least one available section"
    );
    assert!(
        catalogue
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| matches!(item, CatalogueItem::Movie(_) | CatalogueItem::Series(_))),
        "a catalogue surface should contain normalized media cards"
    );
}
