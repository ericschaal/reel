use std::{error::Error as StdError, fmt};

use reqwest::StatusCode;

#[derive(Debug)]
pub enum Error {
    InvalidBaseUrl(url::ParseError),
    InvalidAuthorizationHeader(reqwest::header::InvalidHeaderValue),
    Transport(reqwest::Error),
    Api { status: StatusCode, body: String },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl(error) => write!(formatter, "invalid Jellyfin base URL: {error}"),
            Self::InvalidAuthorizationHeader(error) => {
                write!(formatter, "invalid Jellyfin authentication value: {error}")
            }
            Self::Transport(error) => write!(formatter, "Jellyfin request failed: {error}"),
            Self::Api { status, body } if body.is_empty() => {
                write!(formatter, "Jellyfin returned {status}")
            }
            Self::Api { status, body } => write!(formatter, "Jellyfin returned {status}: {body}"),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidBaseUrl(error) => Some(error),
            Self::InvalidAuthorizationHeader(error) => Some(error),
            Self::Transport(error) => Some(error),
            Self::Api { .. } => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::Transport(error)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
