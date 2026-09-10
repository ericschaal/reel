use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::{
    Catalogue, CatalogueManifest, CatalogueRailResponse, CatalogueResponse, Collection,
    CollectionResponse, Error, MovieDetailsResponse, SeasonDetailsResponse, SeriesDetailsResponse,
    Surface,
};

pub(super) fn router(catalogue: Catalogue) -> Router {
    Router::new()
        .route("/v1/catalogue/discover", get(discover))
        .route("/v1/catalogue/movies", get(movies))
        .route("/v1/catalogue/series", get(series))
        .route("/v1/catalogue/{surface}/manifest", get(manifest))
        .route("/v1/catalogue/{surface}/rails/{rail}", get(rail))
        .route("/v1/titles/movie/{tmdb_id}", get(movie_details))
        .route("/v1/titles/series/{tmdb_id}", get(series_details))
        .route(
            "/v1/titles/series/{tmdb_id}/seasons/{season_number}",
            get(season_details),
        )
        .route("/v1/catalogue/collections/{collection}", get(collection))
        .route(
            "/v1/catalogue/collections/{collection}/{category_id}",
            get(category_collection),
        )
        .with_state(catalogue)
}

async fn movie_details(
    State(catalogue): State<Catalogue>,
    Path(tmdb_id): Path<i64>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<MovieDetailsResponse>, Error> {
    if tmdb_id <= 0 {
        return Err(Error::MediaNotFound);
    }
    catalogue
        .movie_details(tmdb_id, query.language)
        .await
        .map(Json)
}

#[derive(Debug, Deserialize)]
struct CatalogueQuery {
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CollectionQuery {
    language: Option<String>,
    cursor: Option<String>,
}

async fn discover(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue.discover(query.language).await.map(Json)
}

async fn movies(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue.movies(query.language).await.map(Json)
}

async fn series(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue.series(query.language).await.map(Json)
}

async fn series_details(
    State(catalogue): State<Catalogue>,
    Path(tmdb_id): Path<i64>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<SeriesDetailsResponse>, Error> {
    if tmdb_id <= 0 {
        return Err(Error::MediaNotFound);
    }
    catalogue
        .series_details(tmdb_id, query.language)
        .await
        .map(Json)
}

async fn season_details(
    State(catalogue): State<Catalogue>,
    Path((tmdb_id, season_number)): Path<(i64, i32)>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<SeasonDetailsResponse>, Error> {
    if tmdb_id <= 0 || season_number < 0 {
        return Err(Error::MediaNotFound);
    }
    catalogue
        .season_details(tmdb_id, season_number, query.language)
        .await
        .map(Json)
}

async fn manifest(
    State(catalogue): State<Catalogue>,
    Path(surface): Path<String>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueManifest>, Error> {
    let surface = parse_surface(&surface).ok_or(Error::NotFound)?;
    Ok(Json(catalogue.manifest(surface, query.language.as_deref())))
}

async fn rail(
    State(catalogue): State<Catalogue>,
    Path((surface, rail)): Path<(String, String)>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueRailResponse>, Error> {
    let surface = parse_surface(&surface).ok_or(Error::NotFound)?;
    catalogue
        .rail(surface, &rail, query.language)
        .await
        .map(Json)
}

async fn collection(
    State(catalogue): State<Catalogue>,
    Path(collection): Path<String>,
    Query(query): Query<CollectionQuery>,
) -> Result<Json<CollectionResponse>, Error> {
    let collection = Collection::named(&collection).ok_or(Error::NotFound)?;
    catalogue
        .collection(collection, query.language, query.cursor)
        .await
        .map(Json)
}

async fn category_collection(
    State(catalogue): State<Catalogue>,
    Path((collection, category_id)): Path<(String, i64)>,
    Query(query): Query<CollectionQuery>,
) -> Result<Json<CollectionResponse>, Error> {
    let collection = Collection::category(&collection, category_id).ok_or(Error::NotFound)?;
    catalogue
        .collection(collection, query.language, query.cursor)
        .await
        .map(Json)
}

fn parse_surface(surface: &str) -> Option<Surface> {
    match surface {
        "discover" => Some(Surface::Discover),
        "movies" => Some(Surface::Movies),
        "series" => Some(Surface::Series),
        _ => None,
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidCursor => (
                StatusCode::BAD_REQUEST,
                "invalid_cursor",
                "The continuation cursor is invalid",
            ),
            Self::NotFound => (
                StatusCode::NOT_FOUND,
                "collection_not_found",
                "The catalogue collection was not found",
            ),
            Self::MediaNotFound => (
                StatusCode::NOT_FOUND,
                "media_not_found",
                "The requested media was not found",
            ),
            Self::Unavailable => (
                StatusCode::BAD_GATEWAY,
                "catalogue_unavailable",
                "The catalogue is temporarily unavailable",
            ),
        };
        (
            status,
            Json(ErrorResponse {
                error: ApiError { code, message },
            }),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Debug, Serialize)]
struct ApiError {
    code: &'static str,
    message: &'static str,
}
