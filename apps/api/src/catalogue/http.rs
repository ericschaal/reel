use axum::{
    Json, Router,
    extract::{FromRequestParts, Path, Query as AxumQuery, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::media::{SeasonNumber, TmdbId};

use super::{
    Catalogue, CatalogueManifest, CatalogueRailResponse, CatalogueResponse, Collection,
    CollectionResponse, Error, MediaKind, MovieDetailsResponse, SeasonDetailsResponse,
    SeriesDetailsResponse, Surface, TitleSummariesResponse, TitleSummary, titles::TitleKey,
};

pub(super) fn router(catalogue: Catalogue) -> Router {
    Router::new()
        .route("/v1/catalogue/discover", get(discover))
        .route("/v1/catalogue/movies", get(movies))
        .route("/v1/catalogue/series", get(series))
        .route("/v1/catalogue/{surface}/manifest", get(manifest))
        .route("/v1/catalogue/{surface}/rails/{rail}", get(rail))
        .route("/v1/titles/summaries", get(title_summaries))
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

// Keep malformed and unsupported query parameters in the API's JSON error envelope.
struct Query<T>(T);
impl<S: Send + Sync, T: DeserializeOwned + Send> FromRequestParts<S> for Query<T> {
    type Rejection = Error;
    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Error> {
        AxumQuery::<T>::from_request_parts(parts, state)
            .await
            .map(|AxumQuery(value)| Self(value))
            .map_err(|_| Error::InvalidQuery)
    }
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum TitleView {
    Summary,
    #[default]
    Detail,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum TitleInclude {
    InitialSeason,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TitleQuery {
    language: Option<String>,
    #[serde(default)]
    view: TitleView,
    include: Option<TitleInclude>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum TitleResponse {
    Summary(TitleSummary),
    Movie(MovieDetailsResponse),
    Series(Box<SeriesDetailsResponse>),
}

async fn title_view(
    catalogue: Catalogue,
    kind: MediaKind,
    tmdb_id: i64,
    query: TitleQuery,
) -> Result<Json<TitleResponse>, Error> {
    let tmdb_id = TmdbId::try_from(tmdb_id).map_err(|_| Error::MediaNotFound)?;
    if query.include.is_some() && (kind != MediaKind::Series || query.view != TitleView::Detail) {
        return Err(Error::InvalidQuery);
    }
    let language = query.language.as_deref();
    if query.view == TitleView::Summary {
        let metadata = catalogue
            .title_metadata(TitleKey {
                kind,
                id: tmdb_id,
                language: query.language,
            })
            .await?;
        return Ok(Json(TitleResponse::Summary(metadata.summary())));
    }
    match kind {
        MediaKind::Movie => catalogue
            .movie_details(tmdb_id, language)
            .await
            .map(TitleResponse::Movie)
            .map(Json),
        MediaKind::Series => {
            let details = if query.include.is_some() {
                catalogue
                    .series_details_with_initial_season(tmdb_id, language)
                    .await?
            } else {
                catalogue.series_details(tmdb_id, language).await?
            };
            Ok(Json(TitleResponse::Series(Box::new(details))))
        }
    }
}

async fn movie_details(
    State(catalogue): State<Catalogue>,
    Path(tmdb_id): Path<i64>,
    Query(query): Query<TitleQuery>,
) -> Result<Json<TitleResponse>, Error> {
    title_view(catalogue, MediaKind::Movie, tmdb_id, query).await
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummariesQuery {
    ids: String,
    language: Option<String>,
}

async fn title_summaries(
    State(catalogue): State<Catalogue>,
    Query(query): Query<SummariesQuery>,
) -> Result<Json<TitleSummariesResponse>, Error> {
    catalogue
        .title_summaries(&query.ids, query.language.as_deref())
        .await
        .map(Json)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogueQuery {
    language: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CollectionQuery {
    language: Option<String>,
    cursor: Option<String>,
}

async fn discover(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue
        .discover(query.language.as_deref())
        .await
        .map(Json)
}

async fn movies(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue.movies(query.language.as_deref()).await.map(Json)
}

async fn series(
    State(catalogue): State<Catalogue>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<CatalogueResponse>, Error> {
    catalogue.series(query.language.as_deref()).await.map(Json)
}

async fn series_details(
    State(catalogue): State<Catalogue>,
    Path(tmdb_id): Path<i64>,
    Query(query): Query<TitleQuery>,
) -> Result<Json<TitleResponse>, Error> {
    title_view(catalogue, MediaKind::Series, tmdb_id, query).await
}

async fn season_details(
    State(catalogue): State<Catalogue>,
    Path((tmdb_id, season_number)): Path<(i64, i32)>,
    Query(query): Query<CatalogueQuery>,
) -> Result<Json<SeasonDetailsResponse>, Error> {
    let tmdb_id = TmdbId::try_from(tmdb_id).map_err(|_| Error::MediaNotFound)?;
    let season_number = SeasonNumber::try_from(season_number).map_err(|_| Error::MediaNotFound)?;
    catalogue
        .season_details(tmdb_id, season_number, query.language.as_deref())
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
        .rail(surface, &rail, query.language.as_deref())
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
        .collection(
            collection,
            query.language.as_deref(),
            query.cursor.as_deref(),
        )
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
        .collection(
            collection,
            query.language.as_deref(),
            query.cursor.as_deref(),
        )
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
            Self::InvalidQuery => (
                StatusCode::BAD_REQUEST,
                "invalid_query",
                "The query parameters are invalid",
            ),
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
