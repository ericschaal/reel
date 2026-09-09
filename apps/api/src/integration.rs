use std::{error::Error as StdError, fmt};

use reqwest::{
    Client as HttpClient, Response, StatusCode, Url,
    header::{HeaderMap, InvalidHeaderValue},
};
use serde::{Serialize, de::DeserializeOwned};

const MAX_ERROR_BODY_LENGTH: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Integration {
    Jellyfin,
    Seerr,
}

impl fmt::Display for Integration {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jellyfin => formatter.write_str("Jellyfin"),
            Self::Seerr => formatter.write_str("Seerr"),
        }
    }
}

#[derive(Debug)]
pub enum ConfigurationError {
    InvalidBaseUrl(url::ParseError),
    InvalidAuthenticationHeader(InvalidHeaderValue),
    InvalidEndpointPath(String),
}

impl fmt::Display for ConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl(error) => write!(formatter, "invalid base URL: {error}"),
            Self::InvalidAuthenticationHeader(error) => {
                write!(formatter, "invalid authentication header: {error}")
            }
            Self::InvalidEndpointPath(path) => {
                write!(formatter, "endpoint path must be relative: {path}")
            }
        }
    }
}

impl StdError for ConfigurationError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::InvalidBaseUrl(error) => Some(error),
            Self::InvalidAuthenticationHeader(error) => Some(error),
            Self::InvalidEndpointPath(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Configuration {
        integration: Integration,
        source: ConfigurationError,
    },
    Transport {
        integration: Integration,
        source: reqwest::Error,
    },
    Rejected {
        integration: Integration,
        status: StatusCode,
        body: String,
    },
    InvalidResponse {
        integration: Integration,
        source: reqwest::Error,
    },
}

impl Error {
    pub(crate) fn invalid_base_url(integration: Integration, source: url::ParseError) -> Self {
        Self::Configuration {
            integration,
            source: ConfigurationError::InvalidBaseUrl(source),
        }
    }

    pub(crate) fn invalid_authentication_header(
        integration: Integration,
        source: InvalidHeaderValue,
    ) -> Self {
        Self::Configuration {
            integration,
            source: ConfigurationError::InvalidAuthenticationHeader(source),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration {
                integration,
                source,
            } => write!(formatter, "{integration} configuration failed: {source}"),
            Self::Transport {
                integration,
                source,
            } => write!(formatter, "{integration} request failed: {source}"),
            Self::Rejected {
                integration,
                status,
                body,
            } if body.is_empty() => write!(formatter, "{integration} returned {status}"),
            Self::Rejected {
                integration,
                status,
                body,
            } => write!(formatter, "{integration} returned {status}: {body}"),
            Self::InvalidResponse {
                integration,
                source,
            } => write!(
                formatter,
                "{integration} returned an invalid response: {source}"
            ),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Configuration { source, .. } => Some(source),
            Self::Transport { source, .. } | Self::InvalidResponse { source, .. } => Some(source),
            Self::Rejected { .. } => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub(crate) struct JsonClient {
    integration: Integration,
    base_url: Url,
    http: HttpClient,
}

impl JsonClient {
    pub(crate) fn new(
        integration: Integration,
        base_url: Url,
        default_headers: HeaderMap,
    ) -> Result<Self> {
        let http = HttpClient::builder()
            .default_headers(default_headers)
            .build()
            .map_err(|source| Error::Transport {
                integration,
                source,
            })?;
        Ok(Self {
            integration,
            base_url,
            http,
        })
    }

    pub(crate) async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.endpoint(path)?;
        self.get_url(url).await
    }

    pub(crate) async fn get_with_query<Q, T>(&self, path: &str, query: &Q) -> Result<T>
    where
        Q: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let response = self
            .http
            .get(self.endpoint(path)?)
            .query(query)
            .send()
            .await
            .map_err(|source| self.transport_error(source))?;
        self.decode_json(response).await
    }

    pub(crate) async fn get_url<T: DeserializeOwned>(&self, url: Url) -> Result<T> {
        let response = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|source| self.transport_error(source))?;
        self.decode_json(response).await
    }

    pub(crate) async fn post_json<B, T>(&self, path: &str, body: &B) -> Result<T>
    where
        B: Serialize + ?Sized,
        T: DeserializeOwned,
    {
        let response = self
            .http
            .post(self.endpoint(path)?)
            .json(body)
            .send()
            .await
            .map_err(|source| self.transport_error(source))?;
        self.decode_json(response).await
    }

    pub(crate) fn endpoint(&self, path: &str) -> Result<Url> {
        if path.starts_with('/') || Url::parse(path).is_ok() {
            return Err(Error::Configuration {
                integration: self.integration,
                source: ConfigurationError::InvalidEndpointPath(path.to_owned()),
            });
        }
        self.base_url
            .join(path)
            .map_err(|source| Error::invalid_base_url(self.integration, source))
    }

    pub(crate) fn resolve_url(&self, path_or_url: &str) -> Result<Url> {
        match Url::parse(path_or_url) {
            Ok(url) => Ok(url),
            Err(url::ParseError::RelativeUrlWithoutBase) => self
                .base_url
                .join(path_or_url)
                .map_err(|source| Error::invalid_base_url(self.integration, source)),
            Err(source) => Err(Error::invalid_base_url(self.integration, source)),
        }
    }

    async fn decode_json<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let response = self.checked(response).await?;
        response.json().await.map_err(|source| {
            if source.is_decode() {
                Error::InvalidResponse {
                    integration: self.integration,
                    source,
                }
            } else {
                self.transport_error(source)
            }
        })
    }

    async fn checked(&self, mut response: Response) -> Result<Response> {
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }

        let mut bytes = Vec::with_capacity(MAX_ERROR_BODY_LENGTH);
        while bytes.len() < MAX_ERROR_BODY_LENGTH {
            let Some(chunk) = response
                .chunk()
                .await
                .map_err(|source| self.transport_error(source))?
            else {
                break;
            };
            let remaining = MAX_ERROR_BODY_LENGTH - bytes.len();
            bytes.extend_from_slice(&chunk[..chunk.len().min(remaining)]);
        }

        Err(Error::Rejected {
            integration: self.integration,
            status,
            body: String::from_utf8_lossy(&bytes).into_owned(),
        })
    }

    fn transport_error(&self, source: reqwest::Error) -> Error {
        Error::Transport {
            integration: self.integration,
            source,
        }
    }
}

pub(crate) fn parse_base_url(integration: Integration, base_url: &str) -> Result<Url> {
    let mut base_url =
        Url::parse(base_url).map_err(|source| Error::invalid_base_url(integration, source))?;
    if !base_url.path().ends_with('/') {
        let path = format!("{}/", base_url.path());
        base_url.set_path(&path);
    }
    Ok(base_url)
}
