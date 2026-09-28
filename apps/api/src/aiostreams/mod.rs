use crate::media::{EpisodeNumber, ImdbTitleId, SeasonNumber, TmdbId};
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{
    Client as HttpClient, Response, Url,
    header::{
        ACCEPT, AUTHORIZATION, CONTENT_LENGTH, HOST, HeaderMap, HeaderName, HeaderValue, LOCATION,
        PROXY_AUTHORIZATION, RANGE,
    },
    redirect::Policy,
};
use serde::Deserialize;
use thiserror::Error as ThisError;
use url::Host;

use crate::integration::{Integration, JsonClient, parse_base_url};

const MEDIA_TIMEOUT: Duration = Duration::from_mins(1);
const MAX_REDIRECTS: usize = 5;
const MAX_REQUEST_HEADERS: usize = 32;
const MAX_HEADER_VALUE_LENGTH: usize = 8 * 1024;

#[derive(Clone)]
pub struct AioStreams {
    http: JsonClient,
    base_url: Url,
    authorization: HeaderValue,
}

impl AioStreams {
    /// Creates a client configured for one `AIOStreams` account.
    ///
    /// # Errors
    ///
    /// Returns an error when the base URL or authentication header is invalid,
    /// or when the underlying HTTP client cannot be created.
    pub fn new(
        base_url: impl AsRef<str>,
        uuid: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> Result<Self> {
        let base_url = parse_base_url(Integration::AioStreams, base_url.as_ref())?;
        let encoded = STANDARD.encode(format!("{}:{}", uuid.as_ref(), password.as_ref()));
        let mut authorization =
            HeaderValue::from_str(&format!("Basic {encoded}")).map_err(|source| {
                crate::integration::Error::invalid_authentication_header(
                    Integration::AioStreams,
                    source,
                )
            })?;
        authorization.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        headers.insert(AUTHORIZATION, authorization.clone());
        let http = JsonClient::new(Integration::AioStreams, base_url.clone(), headers)?;
        Ok(Self {
            http,
            base_url,
            authorization,
        })
    }

    /// Searches `AIOStreams` for direct streams matching `target`.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails, `AIOStreams` rejects it, or the
    /// response does not contain a valid result payload.
    pub async fn search(&self, target: SearchTarget) -> Result<SearchOutcome> {
        let (content_type, id) = target.request_parameters();
        let envelope: SearchEnvelope = self
            .http
            .get_with_query(
                "api/v1/search",
                &[
                    ("type", content_type),
                    ("id", id.as_str()),
                    ("requiredFields", "url"),
                    ("format", "true"),
                ],
            )
            .await?;
        if !envelope.success {
            return Err(Error::SearchRejected {
                code: envelope
                    .error
                    .as_ref()
                    .map(|error| compact_text(&error.code, 64))
                    .filter(|code| !code.is_empty())
                    .unwrap_or_else(|| "UNKNOWN".into()),
            });
        }
        let data = envelope.data.ok_or(Error::InvalidSearchResponse)?;
        let mut streams = Vec::with_capacity(data.results.len());
        for (index, result) in data.results.into_iter().enumerate() {
            let Some(raw_url) = result.url.as_deref() else {
                continue;
            };
            let Ok(url) = parse_direct_url(raw_url) else {
                continue;
            };
            if !self.has_same_origin(&url) && !has_safe_literal_destination(&url) {
                continue;
            }
            let Ok(request_headers) = safe_request_headers(result.request_headers) else {
                continue;
            };
            let parsed = result.parsed_file.unwrap_or_default();
            let container = parsed
                .container
                .or(parsed.extension)
                .map(|value| compact_text(&value, 24))
                .filter(|value| !value.is_empty());
            let resolution = parsed
                .resolution
                .map(|value| compact_text(&value, 24))
                .filter(|value| !value.is_empty());
            let quality = parsed
                .quality
                .map(|value| compact_text(&value, 48))
                .filter(|value| !value.is_empty());
            let video_codec = parsed
                .encode
                .map(|value| compact_text(&value, 24))
                .filter(|value| !value.is_empty());
            let addon = result
                .addon
                .map(|value| compact_text(&value, 80))
                .filter(|value| !value.is_empty());
            let service = result
                .service
                .map(|value| compact_text(&value, 48))
                .filter(|value| !value.is_empty());
            let label = source_label(
                index,
                resolution.as_deref(),
                quality.as_deref(),
                result.name.as_deref(),
            );
            let description = source_description(
                result.cached,
                result.size,
                addon.as_deref(),
                service.as_deref(),
            );
            streams.push(DirectStream {
                url,
                request_headers,
                label,
                description,
                addon,
                service,
                cached: result.cached,
                resolution,
                quality,
                container,
                video_codec,
                size_bytes: finite_u64(result.size),
                web_ready: result.not_web_ready != Some(true),
                duration_seconds: finite_u64(result.duration),
            });
        }
        Ok(SearchOutcome {
            streams,
            upstream_error_count: data.errors.len(),
        })
    }

    /// Fetches a validated direct-media URL, following safe redirects.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe destination, a failed DNS or HTTP
    /// request, or an invalid or excessive redirect chain.
    pub async fn media_response(
        &self,
        mut url: Url,
        range: Option<&str>,
        request_headers: &HeaderMap,
    ) -> Result<Response> {
        for redirect_count in 0..=MAX_REDIRECTS {
            // Provider failures can redirect to an HTTP 200 error video.
            if url.host_str() == Some("slate.elfhosted.com") {
                return Err(Error::ProviderErrorVideo);
            }
            let client = self.media_client(&url).await?;
            let mut headers = request_headers.clone();
            headers.remove(RANGE);
            headers.remove(HOST);
            headers.remove(CONTENT_LENGTH);
            if self.has_same_origin(&url) && !headers.contains_key(AUTHORIZATION) {
                headers.insert(AUTHORIZATION, self.authorization.clone());
            }
            let mut request = client.get(url.clone()).headers(headers);
            if let Some(range) = range {
                request = request.header(RANGE, range);
            }
            let response = request.send().await.map_err(Error::MediaTransport)?;
            if !response.status().is_redirection() {
                return Ok(response);
            }
            if redirect_count == MAX_REDIRECTS {
                return Err(Error::TooManyRedirects);
            }
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or(Error::InvalidRedirect)?;
            url = parse_direct_url(
                url.join(location)
                    .map_err(|_| Error::InvalidRedirect)?
                    .as_str(),
            )?;
        }
        Err(Error::TooManyRedirects)
    }

    fn has_same_origin(&self, url: &Url) -> bool {
        self.base_url.scheme() == url.scheme()
            && self.base_url.host_str() == url.host_str()
            && self.base_url.port_or_known_default() == url.port_or_known_default()
    }

    async fn media_client(&self, url: &Url) -> Result<HttpClient> {
        let host = url.host_str().ok_or(Error::UnsafeMediaDestination)?;
        let mut builder = HttpClient::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(MEDIA_TIMEOUT);
        if !self.has_same_origin(url) {
            if is_local_hostname(host) {
                return Err(Error::UnsafeMediaDestination);
            }
            let port = url
                .port_or_known_default()
                .ok_or(Error::UnsafeMediaDestination)?;
            let addresses = tokio::net::lookup_host((host, port))
                .await
                .map_err(Error::Dns)?
                .collect::<Vec<_>>();
            if addresses.is_empty() || addresses.iter().any(|address| !is_public_ip(address.ip())) {
                return Err(Error::UnsafeMediaDestination);
            }
            builder = builder.resolve_to_addrs(host, &addresses);
        }
        builder.build().map_err(Error::MediaTransport)
    }
}

#[derive(Debug, Clone)]
pub enum SearchTarget {
    Movie {
        tmdb_id: TmdbId,
        imdb_id: Option<ImdbTitleId>,
    },
    Episode {
        series_tmdb_id: TmdbId,
        imdb_id: Option<ImdbTitleId>,
        season_number: SeasonNumber,
        episode_number: EpisodeNumber,
    },
}

impl SearchTarget {
    fn request_parameters(&self) -> (&'static str, String) {
        match self {
            SearchTarget::Movie { tmdb_id, imdb_id } => (
                "movie",
                imdb_id
                    .as_ref()
                    .map_or_else(|| format!("tmdb:{tmdb_id}"), ToString::to_string),
            ),
            SearchTarget::Episode {
                series_tmdb_id,
                imdb_id,
                season_number,
                episode_number,
            } => (
                "series",
                imdb_id.as_ref().map_or_else(
                    || format!("tmdb:{series_tmdb_id}:{season_number}:{episode_number}"),
                    |id| format!("{id}:{season_number}:{episode_number}"),
                ),
            ),
        }
    }
}

#[derive(Clone)]
pub struct DirectStream {
    pub url: Url,
    pub request_headers: HeaderMap,
    pub label: String,
    pub description: Option<String>,
    pub addon: Option<String>,
    pub service: Option<String>,
    pub cached: Option<bool>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub size_bytes: Option<u64>,
    pub web_ready: bool,
    pub duration_seconds: Option<u64>,
}

pub struct SearchOutcome {
    pub streams: Vec<DirectStream>,
    pub upstream_error_count: usize,
}

#[derive(Debug, Deserialize)]
struct SearchEnvelope {
    success: bool,
    error: Option<SearchApiError>,
    data: Option<SearchData>,
}

#[derive(Debug, Deserialize)]
struct SearchApiError {
    code: String,
}

#[derive(Debug, Deserialize)]
struct SearchData {
    #[serde(default)]
    results: Vec<SearchResult>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResult {
    url: Option<String>,
    #[serde(default)]
    request_headers: HashMap<String, String>,
    parsed_file: Option<ParsedFile>,
    addon: Option<String>,
    service: Option<String>,
    cached: Option<bool>,
    size: Option<f64>,
    duration: Option<f64>,
    not_web_ready: Option<bool>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ParsedFile {
    container: Option<String>,
    extension: Option<String>,
    encode: Option<String>,
    resolution: Option<String>,
    quality: Option<String>,
}

fn parse_direct_url(value: &str) -> Result<Url> {
    let mut url = Url::parse(value).map_err(|_| Error::InvalidMediaUrl)?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(Error::InvalidMediaUrl);
    }
    url.set_fragment(None);
    Ok(url)
}

fn safe_request_headers(values: HashMap<String, String>) -> Result<HeaderMap> {
    if values.len() > MAX_REQUEST_HEADERS {
        return Err(Error::InvalidRequestHeaders);
    }
    let mut headers = HeaderMap::new();
    for (name, value) in values {
        if value.len() > MAX_HEADER_VALUE_LENGTH {
            return Err(Error::InvalidRequestHeaders);
        }
        let name = HeaderName::try_from(name).map_err(|_| Error::InvalidRequestHeaders)?;
        if is_forbidden_request_header(&name) {
            continue;
        }
        let mut value = HeaderValue::try_from(value).map_err(|_| Error::InvalidRequestHeaders)?;
        if name == AUTHORIZATION || name == PROXY_AUTHORIZATION || name.as_str() == "cookie" {
            value.set_sensitive(true);
        }
        headers.insert(name, value);
    }
    Ok(headers)
}

fn is_forbidden_request_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "connection"
            | "content-length"
            | "host"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "range"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn source_label(
    index: usize,
    resolution: Option<&str>,
    quality: Option<&str>,
    formatted_name: Option<&str>,
) -> String {
    let pieces = [resolution, quality]
        .into_iter()
        .flatten()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if !pieces.is_empty() {
        return pieces.join(" · ");
    }
    formatted_name
        .map(|value| compact_text(value, 96))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("AIOStreams source {}", index + 1))
}

fn source_description(
    cached: Option<bool>,
    size: Option<f64>,
    addon: Option<&str>,
    service: Option<&str>,
) -> Option<String> {
    let mut pieces = Vec::new();
    if cached == Some(true) {
        pieces.push("Cached".to_owned());
    }
    if let Some(size) = finite_u64(size) {
        pieces.push(format_size(size));
    }
    if let Some(addon) = addon {
        pieces.push(addon.to_owned());
    }
    if let Some(service) = service {
        pieces.push(service.to_owned());
    }
    (!pieces.is_empty()).then(|| pieces.join(" · "))
}

fn finite_u64(value: Option<f64>) -> Option<u64> {
    value
        .filter(|value| value.is_finite() && *value >= 0.0)
        .and_then(|value| value.trunc().to_string().parse().ok())
}

fn format_size(bytes: u64) -> String {
    const GIB: u128 = 1024 * 1024 * 1024;
    const MIB: u128 = 1024 * 1024;
    let bytes = u128::from(bytes);
    if bytes >= GIB {
        let tenths = (bytes * 10 + GIB / 2) / GIB;
        format!("{}.{:01} GB", tenths / 10, tenths % 10)
    } else {
        let megabytes = (bytes + MIB / 2) / MIB;
        format!("{megabytes} MB")
    }
}

fn compact_text(value: &str, max_characters: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_characters)
        .collect()
}

fn is_local_hostname(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let mut labels = host.rsplit('.');
    let last = labels.next();
    let penultimate = labels.next();
    host == "localhost"
        || matches!(last, Some("localhost" | "local" | "internal"))
        || matches!((penultimate, last), (Some("home"), Some("arpa")))
}

fn has_safe_literal_destination(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(host)) => !is_local_hostname(host),
        Some(Host::Ipv4(ip)) => is_public_ipv4(ip),
        Some(Host::Ipv6(ip)) => is_public_ipv6(ip),
        None => false,
    }
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_ipv4(ip),
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map_or_else(|| is_public_ipv6(ip), is_public_ipv4),
    }
}

fn is_public_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && b == 168)
        || (a == 192 && b == 0 && c == 0)
        || (a == 192 && b == 0 && c == 2)
        || (a == 198 && (b == 18 || b == 19))
        || (a == 198 && b == 51 && c == 100)
        || (a == 203 && b == 0 && c == 113)
        || a >= 224)
}

fn is_public_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    !(ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8))
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error(transparent)]
    Integration(#[from] crate::integration::Error),
    #[error("AIOStreams rejected the search ({code})")]
    SearchRejected { code: String },
    #[error("AIOStreams returned an invalid search response")]
    InvalidSearchResponse,
    #[error("AIOStreams returned an invalid direct media URL")]
    InvalidMediaUrl,
    #[error("AIOStreams returned invalid media request headers")]
    InvalidRequestHeaders,
    #[error("the media URL resolved to an unsafe network destination")]
    UnsafeMediaDestination,
    #[error("the media destination could not be resolved: {0}")]
    Dns(std::io::Error),
    #[error("the direct media request failed: {0}")]
    MediaTransport(reqwest::Error),
    #[error("the direct media redirect was invalid")]
    InvalidRedirect,
    #[error("the direct media response redirected too many times")]
    TooManyRedirects,
    #[error("the provider returned an error video instead of the selected media")]
    ProviderErrorVideo,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_provider_search_ids_without_catalogue_kind_prefixes() {
        let tmdb_id = TmdbId::new(42).unwrap();
        let imdb_id = "tt0123456".parse::<ImdbTitleId>().unwrap();
        assert_eq!(
            SearchTarget::Movie {
                tmdb_id,
                imdb_id: None
            }
            .request_parameters(),
            ("movie", "tmdb:42".into())
        );
        assert_eq!(
            SearchTarget::Movie {
                tmdb_id,
                imdb_id: Some(imdb_id.clone())
            }
            .request_parameters(),
            ("movie", "tt0123456".into())
        );
        let episode = |imdb_id| SearchTarget::Episode {
            series_tmdb_id: tmdb_id,
            imdb_id,
            season_number: SeasonNumber::new(0).unwrap(),
            episode_number: 2.try_into().unwrap(),
        };
        assert_eq!(
            episode(None).request_parameters(),
            ("series", "tmdb:42:0:2".into())
        );
        assert_eq!(
            episode(Some(imdb_id)).request_parameters(),
            ("series", "tt0123456:0:2".into())
        );
    }

    #[test]
    fn accepts_only_credential_free_http_urls() {
        assert!(parse_direct_url("https://media.example/video.mp4#fragment").is_ok());
        assert!(parse_direct_url("http://media.example/video.m3u8").is_ok());
        assert!(parse_direct_url("file:///etc/passwd").is_err());
        assert!(parse_direct_url("https://user:pass@media.example/video").is_err());
    }

    #[test]
    fn rejects_private_and_special_network_ranges() {
        for address in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "::1",
            "fd00::1",
            "fe80::1",
            "2001:db8::1",
        ] {
            assert!(!is_public_ip(address.parse().unwrap()), "{address}");
        }
        assert!(is_public_ip("1.1.1.1".parse().unwrap()));
        assert!(is_public_ip("2606:4700:4700::1111".parse().unwrap()));
        assert!(!has_safe_literal_destination(
            &Url::parse("http://127.0.0.1:8080/video").unwrap()
        ));
        assert!(!has_safe_literal_destination(
            &Url::parse("http://metadata.internal/video").unwrap()
        ));
    }

    #[test]
    fn drops_transport_headers_and_marks_credentials_sensitive() {
        let headers = safe_request_headers(HashMap::from([
            ("Authorization".into(), "Bearer token".into()),
            ("Range".into(), "bytes=0-99".into()),
            ("Referer".into(), "https://example.com".into()),
        ]))
        .unwrap();
        assert!(!headers.contains_key(RANGE));
        assert_eq!(headers["referer"], "https://example.com");
        assert!(headers[AUTHORIZATION].is_sensitive());
    }

    #[test]
    fn labels_use_structured_metadata_before_formatted_names() {
        assert_eq!(
            source_label(0, Some("2160p"), Some("BluRay REMUX"), Some("fallback")),
            "2160p · BluRay REMUX"
        );
        assert_eq!(source_label(1, None, None, None), "AIOStreams source 2");
    }
}
