//! Media acquisition through Seerr, independent of Jellyfin availability.

use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{
    integration::Error as UpstreamError,
    media::TmdbId,
    seerr::{
        CreateMediaRequest, MediaInfo, MediaRequest, MediaSeason, QueueItem, RequestMediaType,
        Seerr, ServiceProfile,
    },
};

pub fn router(seerr: Seerr) -> Router {
    Router::new()
        .route("/v1/requests", post(create))
        .route("/v1/requests/profiles/{kind}", get(profiles))
        .route("/v1/requests/{kind}/{tmdb_id}", get(status))
        .with_state(seerr)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Kind {
    Movie,
    Series,
}

impl Kind {
    fn media_type(self) -> RequestMediaType {
        match self {
            Self::Movie => RequestMediaType::Movie,
            Self::Series => RequestMediaType::Tv,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateRequest {
    kind: Kind,
    tmdb_id: TmdbId,
    season_numbers: Option<Vec<i32>>,
    profile_id: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileResponse {
    server_id: i64,
    server_name: String,
    default_profile_id: i64,
    profiles: Vec<ServiceProfile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatedRequest {
    id: i64,
    request_status: RequestStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    kind: Kind,
    tmdb_id: TmdbId,
    request_status: RequestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_at: Option<String>,
    acquisition_status: AcquisitionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_status: Option<TransferStatus>,
    seasons: Vec<SeasonStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SeasonStatus {
    season_number: i32,
    request_status: RequestStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    requested_at: Option<String>,
    acquisition_status: AcquisitionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    transfer_status: Option<TransferStatus>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum RequestStatus {
    None,
    Pending,
    Approved,
    Declined,
    Failed,
    Completed,
    Unknown,
}

impl RequestStatus {
    fn from_seerr(status: u8) -> Self {
        match status {
            1 => Self::Pending,
            2 => Self::Approved,
            3 => Self::Declined,
            4 => Self::Failed,
            5 => Self::Completed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
enum AcquisitionStatus {
    Unknown,
    Pending,
    Processing,
    PartiallyAvailable,
    Available,
    Blocklisted,
    Deleted,
}

impl AcquisitionStatus {
    fn from_seerr(status: Option<u8>) -> Self {
        match status {
            Some(2) => Self::Pending,
            Some(3) => Self::Processing,
            Some(4) => Self::PartiallyAvailable,
            Some(5) => Self::Available,
            Some(6) => Self::Blocklisted,
            Some(7) => Self::Deleted,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
enum TransferStatus {
    Waiting,
    Queued,
    Downloading,
    Importing,
    Attention,
}

impl TransferStatus {
    fn from_queue<'a>(items: impl Iterator<Item = &'a QueueItem>) -> Self {
        items.fold(Self::Waiting, |current, item| {
            let status = item.status.to_ascii_lowercase();
            let tracked_status = item.tracked_download_status.to_ascii_lowercase();
            let state = item.tracked_download_state.to_ascii_lowercase();
            let next = if tracked_status == "warning"
                || tracked_status == "error"
                || state.contains("failed")
            {
                Self::Attention
            } else if state.starts_with("import") || status == "completed" {
                Self::Importing
            } else if state == "downloading" || status == "downloading" {
                Self::Downloading
            } else {
                Self::Queued
            };
            current.max(next)
        })
    }
}

async fn create(
    State(seerr): State<Seerr>,
    request: Result<Json<CreateRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<CreatedRequest>), ApiError> {
    let Json(request) = request.map_err(|_| ApiError::InvalidRequest)?;
    let seasons = match (request.kind, request.season_numbers) {
        (Kind::Movie, None) => None,
        (Kind::Series, Some(mut numbers))
            if !numbers.is_empty() && numbers.iter().all(|number| *number > 0) =>
        {
            numbers.sort_unstable();
            numbers.dedup();
            Some(numbers)
        }
        _ => return Err(ApiError::InvalidRequest),
    };
    if request.profile_id <= 0 {
        return Err(ApiError::InvalidProfile);
    }
    let available_profiles = load_profiles(&seerr, request.kind).await?;
    if !available_profiles
        .profiles
        .iter()
        .any(|profile| profile.id == request.profile_id)
    {
        return Err(ApiError::InvalidProfile);
    }
    let body = CreateMediaRequest {
        media_type: request.kind.media_type(),
        media_id: request.tmdb_id,
        seasons,
        server_id: available_profiles.server_id,
        profile_id: request.profile_id,
    };
    let created = seerr.request_media(&body).await.map_err(ApiError::from)?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedRequest {
            id: created.id,
            request_status: RequestStatus::from_seerr(created.status),
        }),
    ))
}

async fn profiles(
    State(seerr): State<Seerr>,
    Path(kind): Path<String>,
) -> Result<Json<ProfileResponse>, ApiError> {
    let kind = match kind.as_str() {
        "movie" => Kind::Movie,
        "series" => Kind::Series,
        _ => return Err(ApiError::MediaNotFound),
    };
    load_profiles(&seerr, kind).await.map(Json)
}

async fn load_profiles(seerr: &Seerr, kind: Kind) -> Result<ProfileResponse, ApiError> {
    let media_type = kind.media_type();
    let servers = seerr
        .service_servers(media_type)
        .await
        .map_err(|_| ApiError::ProfilesUnavailable)?;
    let server = servers
        .into_iter()
        .find(|server| server.is_default && !server.is4k)
        .ok_or(ApiError::ProfilesUnavailable)?;
    let details = seerr
        .service_details(media_type, server.id)
        .await
        .map_err(|_| ApiError::ProfilesUnavailable)?;
    if details.profiles.is_empty() || details.server.id != server.id {
        return Err(ApiError::ProfilesUnavailable);
    }
    let default_profile_id = if details
        .profiles
        .iter()
        .any(|profile| profile.id == server.active_profile_id)
    {
        server.active_profile_id
    } else {
        details.profiles[0].id
    };
    Ok(ProfileResponse {
        server_id: server.id,
        server_name: server.name,
        default_profile_id,
        profiles: details.profiles,
    })
}

async fn status(
    State(seerr): State<Seerr>,
    Path((kind, id)): Path<(String, i64)>,
) -> Result<Json<StatusResponse>, ApiError> {
    let kind = match kind.as_str() {
        "movie" => Kind::Movie,
        "series" => Kind::Series,
        _ => return Err(ApiError::MediaNotFound),
    };
    let tmdb_id = TmdbId::try_from(id).map_err(|_| ApiError::MediaNotFound)?;
    let (media, seasons) = match kind {
        Kind::Movie => {
            let details = seerr.movie(tmdb_id, None).await.map_err(ApiError::from)?;
            (details.media_info, Vec::new())
        }
        Kind::Series => {
            let details = seerr
                .series_details(tmdb_id, None)
                .await
                .map_err(ApiError::from)?;
            let numbers = details
                .seasons
                .into_iter()
                .map(|season| season.season_number.get())
                .filter(|number| *number > 0)
                .collect();
            (details.media_info, numbers)
        }
    };
    let (request_status, requested_at) = latest_request(media.as_ref(), None);
    let acquisition_status = AcquisitionStatus::from_seerr(media.as_ref().and_then(|m| m.status));
    let queue = if matches!(acquisition_status, AcquisitionStatus::Processing) {
        if let Some((server_id, external_id)) = media
            .as_ref()
            .and_then(|media| media.service_id.zip(media.external_service_id))
        {
            tokio::time::timeout(
                Duration::from_secs(5),
                seerr.queue_details(kind.media_type(), server_id, external_id),
            )
            .await
            .ok()
            .and_then(Result::ok)
        } else {
            None
        }
    } else {
        None
    };
    let transfer_status = queue
        .as_ref()
        .map(|queue| TransferStatus::from_queue(queue.iter()));
    let seasons = seasons
        .into_iter()
        .map(|season_number| {
            let (request_status, requested_at) =
                latest_request(media.as_ref(), Some(season_number));
            SeasonStatus {
                season_number,
                request_status,
                requested_at,
                acquisition_status: AcquisitionStatus::from_seerr(
                    media
                        .as_ref()
                        .and_then(|m| m.seasons.iter().find(|s| s.season_number == season_number))
                        .map(|season: &MediaSeason| season.status),
                ),
                transfer_status: queue.as_ref().and_then(|queue| {
                    if queue.is_empty() {
                        return Some(TransferStatus::Waiting);
                    }
                    let matching = queue.iter().filter(|item| {
                        item.season_number == Some(season_number)
                            || item.season_numbers.contains(&season_number)
                            || item
                                .episodes
                                .iter()
                                .any(|episode| episode.season_number == season_number)
                    });
                    let matching: Vec<_> = matching.collect();
                    (!matching.is_empty()).then(|| TransferStatus::from_queue(matching.into_iter()))
                }),
            }
        })
        .collect();
    Ok(Json(StatusResponse {
        kind,
        tmdb_id,
        request_status,
        requested_at,
        acquisition_status,
        transfer_status,
        seasons,
    }))
}

fn latest_request(
    media: Option<&MediaInfo>,
    season_number: Option<i32>,
) -> (RequestStatus, Option<String>) {
    let request = media
        .into_iter()
        .flat_map(|media| &media.requests)
        .filter(|request| !request.is4k)
        .filter(|request| {
            season_number.is_none_or(|number| {
                request
                    .seasons
                    .iter()
                    .any(|season| season.season_number == number)
            })
        })
        .max_by_key(|request: &&MediaRequest| request.id);
    request.map_or((RequestStatus::None, None), |request| {
        let status = season_number.and_then(|number| {
            request
                .seasons
                .iter()
                .find(|season| season.season_number == number)
                .map(|season| season.status)
        });
        (
            RequestStatus::from_seerr(status.unwrap_or(request.status)),
            request.created_at.clone(),
        )
    })
}

enum ApiError {
    InvalidRequest,
    InvalidProfile,
    ProfilesUnavailable,
    MediaNotFound,
    Duplicate,
    NoSeasons,
    Quota,
    Forbidden,
    Blocklisted,
    Unavailable,
}

impl From<UpstreamError> for ApiError {
    fn from(error: UpstreamError) -> Self {
        match error {
            UpstreamError::Rejected { status, body, .. } => match status {
                StatusCode::NOT_FOUND => Self::MediaNotFound,
                StatusCode::CONFLICT => Self::Duplicate,
                StatusCode::ACCEPTED => Self::NoSeasons,
                StatusCode::FORBIDDEN if body.to_ascii_lowercase().contains("quota") => Self::Quota,
                StatusCode::FORBIDDEN if body.to_ascii_lowercase().contains("blocklist") => {
                    Self::Blocklisted
                }
                StatusCode::FORBIDDEN => Self::Forbidden,
                _ => Self::Unavailable,
            },
            _ => Self::Unavailable,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match self {
            Self::InvalidRequest => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Choose a movie or at least one regular TV season",
            ),
            Self::InvalidProfile => (
                StatusCode::BAD_REQUEST,
                "invalid_profile",
                "Choose a current Seerr quality profile",
            ),
            Self::ProfilesUnavailable => (
                StatusCode::BAD_GATEWAY,
                "profiles_unavailable",
                "Seerr could not load profiles from its default Radarr or Sonarr server",
            ),
            Self::MediaNotFound => (
                StatusCode::NOT_FOUND,
                "media_not_found",
                "This title was not found in Seerr",
            ),
            Self::Duplicate => (
                StatusCode::CONFLICT,
                "request_exists",
                "This title already has a request in Seerr",
            ),
            Self::NoSeasons => (
                StatusCode::CONFLICT,
                "no_seasons_available",
                "None of the selected seasons can be requested in Seerr",
            ),
            Self::Quota => (
                StatusCode::FORBIDDEN,
                "request_quota_exceeded",
                "The Seerr request quota has been reached",
            ),
            Self::Forbidden => (
                StatusCode::FORBIDDEN,
                "request_forbidden",
                "Seerr does not allow this request with the configured API key",
            ),
            Self::Blocklisted => (
                StatusCode::FORBIDDEN,
                "media_blocklisted",
                "This title is blocklisted in Seerr",
            ),
            Self::Unavailable => (
                StatusCode::BAD_GATEWAY,
                "seerr_unavailable",
                "Seerr could not complete this request",
            ),
        };
        (
            status,
            Json(serde_json::json!({"error": {"code": code, "message": message}})),
        )
            .into_response()
    }
}
