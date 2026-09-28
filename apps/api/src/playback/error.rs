use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error as ThisError;

#[derive(Debug, Clone, Copy)]
pub enum PlaybackUpstream {
    Jellyfin,
    AioStreams,
    Stremio,
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("the playback request is invalid")]
    InvalidRequest(#[source] axum::extract::rejection::JsonRejection),
    #[error("playback storage capacity is exhausted")]
    CapacityExceeded,
    #[error("the configured Jellyfin playback user was not found")]
    UserNotFound,
    #[error("the requested media is not available locally")]
    NotLocal,
    #[error("no compatible playback source is available")]
    NoCompatibleSource,
    #[error("the playback session was not found or has expired")]
    SessionNotFound,
    #[error("the playback resource is invalid")]
    InvalidResource,
    #[error("the playback start position is invalid")]
    InvalidStartPosition,
    #[error("the source discovery was not found or has expired")]
    DiscoveryNotFound,
    #[error("the selected playback candidate was not found")]
    CandidateNotFound,
    #[error("AIOStreams is not configured")]
    AioStreamsNotConfigured,
    #[error("Stremio streaming is unavailable")]
    StremioUnavailable,
    #[error("Stremio media request failed")]
    StremioTransport(#[source] reqwest::Error),
    #[error("Stremio needs a reachable Reel input proxy for this source's request headers")]
    StremioInputUnavailable,
    #[error("AIOStreams returned no compatible direct sources")]
    NoRemoteSources,
    #[error("AIOStreams providers failed before returning a direct source")]
    RemoteProvidersUnavailable,
    #[error("{upstream:?} returned an invalid playback response")]
    InvalidUpstreamResponse { upstream: PlaybackUpstream },
    #[error("{upstream:?} media body could not be read")]
    InvalidUpstreamBody {
        upstream: PlaybackUpstream,
        #[source]
        source: reqwest::Error,
    },
    #[error(transparent)]
    Jellyfin(#[from] crate::jellyfin::Error),
    #[error(transparent)]
    AioStreams(#[from] crate::aiostreams::Error),
}

type ResponseDetails = (StatusCode, &'static str, &'static str);

const CAPACITY_EXCEEDED: ResponseDetails = (
    StatusCode::SERVICE_UNAVAILABLE,
    "playback_capacity_exceeded",
    "Playback is busy. Try again shortly",
);
const NOT_LOCAL: ResponseDetails = (
    StatusCode::NOT_FOUND,
    "not_local",
    "The requested media is not available in Jellyfin",
);
const NO_COMPATIBLE_SOURCE: ResponseDetails = (
    StatusCode::UNPROCESSABLE_ENTITY,
    "no_compatible_source",
    "No compatible playback source is available",
);
const SESSION_NOT_FOUND: ResponseDetails = (
    StatusCode::NOT_FOUND,
    "playback_session_not_found",
    "The playback session was not found or has expired",
);
const INVALID_RESOURCE: ResponseDetails = (
    StatusCode::BAD_REQUEST,
    "invalid_playback_resource",
    "The playback resource is invalid",
);
const INVALID_POSITION: ResponseDetails = (
    StatusCode::BAD_REQUEST,
    "invalid_playback_position",
    "The playback start position must be a finite non-negative number",
);
const DISCOVERY_NOT_FOUND: ResponseDetails = (
    StatusCode::NOT_FOUND,
    "source_discovery_not_found",
    "The source list was not found or has expired",
);
const CANDIDATE_NOT_FOUND: ResponseDetails = (
    StatusCode::BAD_REQUEST,
    "playback_candidate_not_found",
    "The selected playback source is invalid",
);

impl Error {
    fn response_details(&self) -> ResponseDetails {
        match self {
            Self::InvalidRequest(rejection) => (
                rejection.status(),
                "invalid_playback_request",
                "The playback request is invalid. Check the target, position, track selection, and capabilities",
            ),
            Self::CapacityExceeded => CAPACITY_EXCEEDED,
            Self::NotLocal => NOT_LOCAL,
            Self::NoCompatibleSource => NO_COMPATIBLE_SOURCE,
            Self::SessionNotFound => SESSION_NOT_FOUND,
            Self::InvalidResource => INVALID_RESOURCE,
            Self::InvalidStartPosition => INVALID_POSITION,
            Self::DiscoveryNotFound => DISCOVERY_NOT_FOUND,
            Self::CandidateNotFound => CANDIDATE_NOT_FOUND,
            Self::AioStreamsNotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "aiostreams_not_configured",
                "Remote playback is not configured",
            ),
            Self::NoRemoteSources => (
                StatusCode::NOT_FOUND,
                "no_remote_sources",
                "No compatible direct streams are available",
            ),
            Self::RemoteProvidersUnavailable => (
                StatusCode::BAD_GATEWAY,
                "aiostreams_unavailable",
                "Streaming providers failed before returning a direct stream. Check AIOStreams provider and debrid connections, then try again",
            ),
            Self::StremioUnavailable
            | Self::StremioTransport(_)
            | Self::InvalidUpstreamBody {
                upstream: PlaybackUpstream::Stremio,
                ..
            }
            | Self::InvalidUpstreamResponse {
                upstream: PlaybackUpstream::Stremio,
            } => (
                StatusCode::BAD_GATEWAY,
                "stremio_unavailable",
                "The Stremio streaming server could not prepare this source",
            ),
            Self::StremioInputUnavailable => (
                StatusCode::BAD_GATEWAY,
                "stremio_input_unavailable",
                "This source needs request headers. Configure REEL_STREAMING_BASE_URL to an API address reachable by Stremio, or choose another source",
            ),
            Self::AioStreams(crate::aiostreams::Error::ProviderErrorVideo) => (
                StatusCode::BAD_GATEWAY,
                "source_not_ready",
                "This provider has not made the episode available. Choose another source",
            ),
            Self::AioStreams(_)
            | Self::InvalidUpstreamBody {
                upstream: PlaybackUpstream::AioStreams,
                ..
            }
            | Self::InvalidUpstreamResponse {
                upstream: PlaybackUpstream::AioStreams,
            } => (
                StatusCode::BAD_GATEWAY,
                "aiostreams_unavailable",
                "Remote playback is temporarily unavailable",
            ),
            Self::UserNotFound
            | Self::InvalidUpstreamResponse {
                upstream: PlaybackUpstream::Jellyfin,
            }
            | Self::InvalidUpstreamBody {
                upstream: PlaybackUpstream::Jellyfin,
                ..
            }
            | Self::Jellyfin(_) => (
                StatusCode::BAD_GATEWAY,
                "jellyfin_unavailable",
                "Jellyfin playback is temporarily unavailable",
            ),
        }
    }

    pub(super) fn log_failure(&self, operation: &'static str, candidate_index: Option<usize>) {
        let (_, code, _) = self.response_details();
        // Never format provider errors: reqwest errors may contain signed URLs.
        let diagnostic = match self {
            Self::StremioTransport(source)
            | Self::InvalidUpstreamBody { source, .. }
            | Self::AioStreams(crate::aiostreams::Error::MediaTransport(source)) => {
                transport_diagnostic(source)
            }
            Self::Jellyfin(error)
            | Self::AioStreams(crate::aiostreams::Error::Integration(error)) => match error {
                crate::integration::Error::Transport { source, .. }
                | crate::integration::Error::InvalidResponse { source, .. } => {
                    transport_diagnostic(source)
                }
                crate::integration::Error::Rejected { status, .. } => {
                    ("http_status", Some(status.as_u16()))
                }
                crate::integration::Error::Configuration { .. } => ("configuration", None),
            },
            Self::AioStreams(crate::aiostreams::Error::Dns(_)) => ("dns", None),
            Self::AioStreams(crate::aiostreams::Error::UnsafeMediaDestination) => {
                ("unsafe_destination", None)
            }
            _ => (code, None),
        };
        tracing::warn!(
            operation,
            candidate_index,
            code,
            cause = diagnostic.0,
            upstream_status = diagnostic.1,
            "playback operation failed"
        );
    }
}

fn transport_diagnostic(error: &reqwest::Error) -> (&'static str, Option<u16>) {
    let kind = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connection"
    } else if error.is_decode() {
        "decode"
    } else {
        "transport"
    };
    (kind, error.status().map(|status| status.as_u16()))
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.log_failure("http_request", None);
        let (status, code, message) = self.response_details();
        (
            status,
            Json(serde_json::json!({"error": {"code": code, "message": message}})),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_errors_identify_the_upstream_that_failed() {
        for (upstream, expected) in [
            (PlaybackUpstream::Jellyfin, "jellyfin_unavailable"),
            (PlaybackUpstream::AioStreams, "aiostreams_unavailable"),
            (PlaybackUpstream::Stremio, "stremio_unavailable"),
        ] {
            let error = Error::InvalidUpstreamResponse { upstream };
            assert_eq!(error.response_details().1, expected);
        }
    }
}
