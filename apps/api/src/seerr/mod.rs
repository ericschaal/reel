mod types;

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{
    Url,
    header::{ACCEPT, HeaderMap, HeaderName, HeaderValue},
};
use serde::Serialize;

use crate::integration::{Integration, JsonClient, parse_base_url};

pub use crate::integration::{Error, Result};
pub use types::*;

const API_KEY: HeaderName = HeaderName::from_static("x-api-key");

#[derive(Clone)]
pub struct Seerr {
    http: JsonClient,
}

impl Seerr {
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

    pub async fn trending(&self, query: &TrendingQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/trending", query).await
    }

    pub async fn movies(&self, query: &DiscoverMoviesQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/movies", query).await
    }

    pub async fn series(&self, query: &DiscoverSeriesQuery) -> Result<DiscoverResponse> {
        self.http.get_with_query("discover/tv", query).await
    }

    pub async fn movie_genres(&self, language: Option<&str>) -> Result<Vec<GenreSliderItem>> {
        self.http
            .get_with_query("discover/genreslider/movie", &LanguageQuery { language })
            .await
    }

    pub async fn series_genres(&self, language: Option<&str>) -> Result<Vec<GenreSliderItem>> {
        self.http
            .get_with_query("discover/genreslider/tv", &LanguageQuery { language })
            .await
    }

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

    pub async fn search(&self, query: &SearchQuery) -> Result<DiscoverResponse> {
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

    pub async fn movie(&self, tmdb_id: i64, language: Option<&str>) -> Result<MovieDetails> {
        self.http
            .get_with_query(&format!("movie/{tmdb_id}"), &LanguageQuery { language })
            .await
    }

    pub async fn series_details(
        &self,
        tmdb_id: i64,
        language: Option<&str>,
    ) -> Result<SeriesDetails> {
        self.http
            .get_with_query(&format!("tv/{tmdb_id}"), &LanguageQuery { language })
            .await
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
