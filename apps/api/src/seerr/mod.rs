mod types;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{
    Url,
    header::{ACCEPT, HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;

use crate::{
    integration::{Integration, JsonClient, parse_base_url},
    media::{SeasonNumber, TmdbId},
};

pub use crate::integration::{Error, Result};
pub use types::*;

const API_KEY: HeaderName = HeaderName::from_static("x-api-key");

#[derive(Clone)]
pub struct Seerr {
    http: JsonClient,
}

impl Seerr {
    /// Creates an authenticated Seerr client.
    ///
    /// # Errors
    ///
    /// Returns an error when the URL or API-key header is invalid, or when the
    /// HTTP client cannot be created.
    pub fn new(base_url: impl AsRef<str>, api_key: impl AsRef<str>) -> Result<Self> {
        let base_url = api_base_url(base_url.as_ref())?;
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut api_key = HeaderValue::from_str(api_key.as_ref())
            .map_err(|source| Error::invalid_authentication_header(Integration::Seerr, source))?;
        api_key.set_sensitive(true);
        headers.insert(API_KEY, api_key);

        let http = JsonClient::new(Integration::Seerr, base_url, headers)?;
        Ok(Self { http })
    }

    /// Fetches trending media.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn trending(&self, query: &TrendingQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/trending", query).await
    }

    /// Discovers movies matching `query`.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn movies(&self, query: &DiscoverMoviesQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/movies", query).await
    }

    /// Discovers series matching `query`.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn series(&self, query: &DiscoverSeriesQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/tv", query).await
    }

    /// Fetches movie genres in the requested language.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn movie_genres(&self, language: Option<&str>) -> Result<Vec<GenreSliderItem>> {
        self.http
            .get_with_query("discover/genreslider/movie", &LanguageQuery { language })
            .await
    }

    /// Fetches series genres in the requested language.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn series_genres(&self, language: Option<&str>) -> Result<Vec<GenreSliderItem>> {
        self.http
            .get_with_query("discover/genreslider/tv", &LanguageQuery { language })
            .await
    }

    /// Discovers movies produced by a studio.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn movies_by_studio(
        &self,
        studio_id: i64,
        page: Option<u32>,
        language: Option<&str>,
    ) -> Result<DiscoverResponse> {
        self.http
            .get_with_query(
                &format!("discover/movies/studio/{studio_id}"),
                &PageQuery { page, language },
            )
            .await
    }

    /// Discovers series carried by a network.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn series_by_network(
        &self,
        network_id: i64,
        page: Option<u32>,
        language: Option<&str>,
    ) -> Result<DiscoverResponse> {
        self.http
            .get_with_query(
                &format!("discover/tv/network/{network_id}"),
                &PageQuery { page, language },
            )
            .await
    }

    /// Discovers movies in a genre.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn movies_by_genre(
        &self,
        genre_id: i64,
        page: Option<u32>,
        language: Option<&str>,
    ) -> Result<DiscoverResponse> {
        self.http
            .get_with_query(
                &format!("discover/movies/genre/{genre_id}"),
                &PageQuery { page, language },
            )
            .await
    }

    /// Discovers series in a genre.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn series_by_genre(
        &self,
        genre_id: i64,
        page: Option<u32>,
        language: Option<&str>,
    ) -> Result<DiscoverResponse> {
        self.http
            .get_with_query(
                &format!("discover/tv/genre/{genre_id}"),
                &PageQuery { page, language },
            )
            .await
    }

    /// Searches for movies and series.
    ///
    /// # Errors
    ///
    /// Returns an error when the endpoint URL cannot be built, the request
    /// fails, or the response is invalid.
    pub async fn search(&self, query: &SearchQuery) -> Result<DiscoverResponse> {
        // Seerr rejects form encoding (`+` for spaces) here and requires every
        // reserved character in these values to use percent encoding.
        let mut parameters = vec![format!(
            "query={}",
            utf8_percent_encode(&query.query, NON_ALPHANUMERIC)
        )];
        if let Some(page) = query.page {
            parameters.push(format!("page={page}"));
        }
        if let Some(language) = query.language.as_deref() {
            parameters.push(format!(
                "language={}",
                utf8_percent_encode(language, NON_ALPHANUMERIC)
            ));
        }

        let mut url = self.http.endpoint("search")?;
        url.set_query(Some(&parameters.join("&")));
        self.http.get_url(url).await
    }

    /// Fetches details for a movie.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn movie(&self, tmdb_id: TmdbId, language: Option<&str>) -> Result<MovieDetails> {
        self.http
            .get_with_query(&format!("movie/{tmdb_id}"), &LanguageQuery { language })
            .await
    }

    /// Fetches details for a series.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn series_details(
        &self,
        tmdb_id: TmdbId,
        language: Option<&str>,
    ) -> Result<SeriesDetails> {
        self.http
            .get_with_query(&format!("tv/{tmdb_id}"), &LanguageQuery { language })
            .await
    }

    /// Fetches details for a season.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the response is invalid.
    pub async fn season_details(
        &self,
        tmdb_id: TmdbId,
        season_number: SeasonNumber,
        language: Option<&str>,
    ) -> Result<SeasonDetails> {
        self.http
            .get_with_query(
                &format!("tv/{tmdb_id}/season/{season_number}"),
                &LanguageQuery { language },
            )
            .await
    }

    /// Submits a movie or season request using Seerr's configured defaults.
    ///
    /// # Errors
    ///
    /// Returns an error when Seerr rejects the request or does not create one.
    pub async fn request_media(&self, body: &CreateMediaRequest) -> Result<MediaRequest> {
        self.http.post_json_created("request", body).await
    }

    /// Lists the Radarr or Sonarr servers configured in Seerr.
    ///
    /// # Errors
    ///
    /// Returns an error when Seerr cannot return the list.
    pub async fn service_servers(
        &self,
        media_type: RequestMediaType,
    ) -> Result<Vec<ServiceServer>> {
        self.http.get(media_type.service_path()).await
    }

    /// Loads quality profiles for a configured Radarr or Sonarr server.
    ///
    /// # Errors
    ///
    /// Returns an error when Seerr or the backing service is unavailable.
    pub async fn service_details(
        &self,
        media_type: RequestMediaType,
        server_id: i64,
    ) -> Result<ServiceDetails> {
        self.http
            .get(&format!("{}/{server_id}", media_type.service_path()))
            .await
    }

    /// Reads the linked Radarr or Sonarr download queue without exposing its credentials.
    ///
    /// # Errors
    ///
    /// Returns an error if Seerr settings or the backing service cannot be read.
    pub async fn queue_details(
        &self,
        media_type: RequestMediaType,
        server_id: i64,
        external_id: i64,
    ) -> Result<Vec<QueueItem>> {
        let integration = media_type.integration();
        let connections: Vec<ServiceConnection> = self.http.get(media_type.settings_path()).await?;
        let connection = connections
            .into_iter()
            .find(|connection| connection.id == server_id)
            .ok_or_else(|| Error::Configuration {
                integration,
                source: crate::integration::ConfigurationError::InvalidEndpointPath(
                    "configured service not found".to_owned(),
                ),
            })?;
        let scheme = if connection.use_ssl { "https" } else { "http" };
        let mut url = Url::parse(&format!("{scheme}://{}/", connection.hostname))
            .map_err(|source| Error::invalid_base_url(integration, source))?;
        url.set_port(Some(connection.port))
            .map_err(|()| Error::Configuration {
                integration,
                source: crate::integration::ConfigurationError::InvalidEndpointPath(
                    "invalid service port".to_owned(),
                ),
            })?;
        let base_path = connection.base_url.trim_matches('/');
        url.set_path(&if base_path.is_empty() {
            "/api/v3/".to_owned()
        } else {
            format!("/{base_path}/api/v3/")
        });
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        let mut api_key = HeaderValue::from_str(&connection.api_key)
            .map_err(|source| Error::invalid_authentication_header(integration, source))?;
        api_key.set_sensitive(true);
        headers.insert(API_KEY, api_key);
        let http = JsonClient::new(integration, url, headers)?;
        let id_field = match media_type {
            RequestMediaType::Movie => "movieId",
            RequestMediaType::Tv => "seriesId",
        };
        http.get(&format!("queue/details?{id_field}={external_id}"))
            .await
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateMediaRequest {
    pub media_type: RequestMediaType,
    pub media_id: TmdbId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<Vec<i32>>,
    pub server_id: i64,
    pub profile_id: i64,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RequestMediaType {
    Movie,
    Tv,
}

impl RequestMediaType {
    fn service_path(self) -> &'static str {
        match self {
            Self::Movie => "service/radarr",
            Self::Tv => "service/sonarr",
        }
    }

    fn settings_path(self) -> &'static str {
        match self {
            Self::Movie => "settings/radarr",
            Self::Tv => "settings/sonarr",
        }
    }

    fn integration(self) -> Integration {
        match self {
            Self::Movie => Integration::Radarr,
            Self::Tv => Integration::Sonarr,
        }
    }
}

#[derive(Serialize)]
struct LanguageQuery<'a> {
    language: Option<&'a str>,
}

#[derive(Serialize)]
struct PageQuery<'a> {
    page: Option<u32>,
    language: Option<&'a str>,
}

fn api_base_url(base_url: &str) -> Result<Url> {
    let base_url = parse_base_url(Integration::Seerr, base_url)?;

    if base_url.path().ends_with("/api/v1/") {
        Ok(base_url)
    } else {
        base_url
            .join("api/v1/")
            .map_err(|source| Error::invalid_base_url(Integration::Seerr, source))
    }
}
