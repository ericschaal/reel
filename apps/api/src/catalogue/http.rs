use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};

use super::{Catalogue, CatalogueResponse, Collection, CollectionResponse, Error};

pub(super) fn router(catalogue: Catalogue) -> Router {
    Router::new()
        .route("/v1/catalogue/discover", get(discover))
        .route("/v1/catalogue/movies", get(movies))
        .route("/v1/catalogue/series", get(series))
        .route("/v1/catalogue/collections/{collection}", get(collection))
        .route(
            "/v1/catalogue/collections/{collection}/{category_id}",
            get(category_collection),
        )
        .with_state(catalogue)
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
