mod types;

use std::fmt::Write as _;

use reqwest::{
    Url,
    header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue},
};

use crate::integration::{Integration, JsonClient, parse_base_url};

pub use crate::integration::{Error, Result};
pub use types::*;

const ITEM_FIELDS: &str = "Overview,ProviderIds,MediaStreams";
const IMAGE_TYPES: &str = "Primary,Backdrop,Logo,Thumb";

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub name: String,
    pub device: String,
    pub device_id: String,
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "Reel".into(),
            device: "Reel API".into(),
            device_id: "reel-api".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

#[derive(Clone)]
pub struct Jellyfin {
    http: JsonClient,
}

impl Jellyfin {
    pub fn new(base_url: impl AsRef<str>, access_token: impl AsRef<str>) -> Result<Self> {
        Self::with_client_info(base_url, Some(access_token.as_ref()), ClientInfo::default())
    }

    pub fn unauthenticated(base_url: impl AsRef<str>) -> Result<Self> {
        Self::with_client_info(base_url, None, ClientInfo::default())
    }

    pub fn with_client_info(
        base_url: impl AsRef<str>,
        access_token: Option<&str>,
        client_info: ClientInfo,
    ) -> Result<Self> {
        let base_url = parse_base_url(Integration::Jellyfin, base_url.as_ref())?;

        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut authorization = format!(
            "MediaBrowser Client=\"{}\", Device=\"{}\", DeviceId=\"{}\", Version=\"{}\"",
            client_info.name, client_info.device, client_info.device_id, client_info.version
        );
        if let Some(token) = access_token.filter(|token| !token.is_empty()) {
            let _ = write!(authorization, ", Token=\"{token}\"");
        }
        let mut authorization = HeaderValue::from_str(&authorization).map_err(|source| {
            Error::invalid_authentication_header(Integration::Jellyfin, source)
        })?;
        authorization.set_sensitive(true);
        headers.insert(AUTHORIZATION, authorization);

        let http = JsonClient::new(Integration::Jellyfin, base_url, headers)?;
        Ok(Self { http })
    }

    pub async fn public_system_info(&self) -> Result<PublicSystemInfo> {
        self.http.get("System/Info/Public").await
    }

    pub async fn system_info(&self) -> Result<PublicSystemInfo> {
        self.http.get("System/Info").await
    }

    pub async fn users(&self) -> Result<Vec<User>> {
        self.http.get("Users").await
    }

    pub async fn items(&self, query: &ItemsQuery) -> Result<ItemQueryResult> {
        let mut parameters = common_item_parameters(query.include_item_types.as_slice());
        push_optional(&mut parameters, "parentId", query.parent_id.as_deref());
        push_joined(&mut parameters, "ids", &query.ids);
        push_optional(&mut parameters, "searchTerm", query.search_term.as_deref());
        push_bool(&mut parameters, "recursive", query.recursive);
        push_number(&mut parameters, "startIndex", query.start_index);
        push_number(&mut parameters, "limit", query.limit);
        push_bool(&mut parameters, "isPlayed", query.is_played);

        if !query.sort_by.is_empty() {
            parameters.push((
                "sortBy",
                query
                    .sort_by
                    .iter()
                    .map(|value| value.as_str())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if let Some(order) = query.sort_order {
            parameters.push(("sortOrder", order.as_str().into()));
        }

        self.http.get_with_query("Items", &parameters).await
    }

    pub async fn search(&self, term: &str, limit: u32) -> Result<ItemQueryResult> {
        self.items(&ItemsQuery {
            search_term: Some(term.to_owned()),
            include_item_types: vec![ItemType::Movie, ItemType::Series, ItemType::Episode],
            recursive: Some(true),
            limit: Some(limit),
            ..ItemsQuery::default()
        })
        .await
    }

    pub async fn item(&self, item_id: &str, user_id: &str) -> Result<Item> {
        self.http
            .get_with_query(
                &format!("Items/{item_id}"),
                &[("userId", user_id.to_owned())],
            )
            .await
    }

    pub async fn latest(&self, user_id: &str, limit: u32) -> Result<Vec<Item>> {
        let mut parameters =
            common_item_parameters(&[ItemType::Movie, ItemType::Series, ItemType::Episode]);
        parameters.push(("userId", user_id.to_owned()));
        push_number(&mut parameters, "limit", Some(limit));
        self.http.get_with_query("Items/Latest", &parameters).await
    }

    pub async fn seasons(&self, series_id: &str) -> Result<ItemQueryResult> {
        let parameters = common_item_parameters(&[]);
        self.http
            .get_with_query(&format!("Shows/{series_id}/Seasons"), &parameters)
            .await
    }

    pub async fn episodes(
        &self,
        series_id: &str,
        season_id: Option<&str>,
    ) -> Result<ItemQueryResult> {
        let mut parameters = common_item_parameters(&[]);
        push_optional(&mut parameters, "seasonId", season_id);
        self.http
            .get_with_query(&format!("Shows/{series_id}/Episodes"), &parameters)
            .await
    }

    pub async fn playback_info(
        &self,
        item_id: &str,
        user_id: &str,
        request: &PlaybackInfoRequest,
    ) -> Result<PlaybackInfoResponse> {
        let mut request = request.clone();
        request.user_id = Some(user_id.to_owned());
        self.http
            .post_json(&format!("Items/{item_id}/PlaybackInfo"), &request)
            .await
    }

    pub fn image_url(
        &self,
        item_id: &str,
        image_type: ImageType,
        options: &ImageOptions,
    ) -> Result<Url> {
        let mut url = self
            .http
            .endpoint(&format!("Items/{item_id}/Images/{}", image_type.as_str()))?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(value) = options.max_width {
                query.append_pair("maxWidth", &value.to_string());
            }
            if let Some(value) = options.max_height {
                query.append_pair("maxHeight", &value.to_string());
            }
            if let Some(value) = options.quality {
                query.append_pair("quality", &value.min(100).to_string());
            }
            if let Some(value) = options.tag.as_deref() {
                query.append_pair("tag", value);
            }
            if let Some(value) = options.image_index {
                query.append_pair("imageIndex", &value.to_string());
            }
        }
        Ok(url)
    }

    pub fn direct_play_url(
        &self,
        item_id: &str,
        container: Option<&str>,
        media_source_id: &str,
        play_session_id: Option<&str>,
    ) -> Result<Url> {
        let path = match container {
            Some(container) => format!("Videos/{item_id}/stream.{container}"),
            None => format!("Videos/{item_id}/stream"),
        };
        let mut url = self.http.endpoint(&path)?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("static", "true");
            query.append_pair("mediaSourceId", media_source_id);
            if let Some(value) = play_session_id {
                query.append_pair("playSessionId", value);
            }
        }
        Ok(url)
    }

    pub fn resolve_url(&self, path_or_url: &str) -> Result<Url> {
        self.http.resolve_url(path_or_url)
    }

    pub fn has_same_origin(&self, url: &Url) -> bool {
        self.http.has_same_origin(url)
    }

    pub async fn media_response(&self, url: Url, range: Option<&str>) -> Result<reqwest::Response> {
        self.http.get_response(url, range).await
    }
}

fn common_item_parameters(item_types: &[ItemType]) -> Vec<(&'static str, String)> {
    let mut parameters = vec![
        ("fields", ITEM_FIELDS.into()),
        ("enableImages", "true".into()),
        ("enableImageTypes", IMAGE_TYPES.into()),
    ];
    if !item_types.is_empty() {
        parameters.push((
            "includeItemTypes",
            item_types
                .iter()
                .map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(","),
        ));
    }
    parameters
}

fn push_optional(
    parameters: &mut Vec<(&'static str, String)>,
    name: &'static str,
    value: Option<&str>,
) {
    if let Some(value) = value {
        parameters.push((name, value.to_owned()));
    }
}

fn push_joined(
    parameters: &mut Vec<(&'static str, String)>,
    name: &'static str,
    values: &[String],
) {
    if !values.is_empty() {
        parameters.push((name, values.join(",")));
    }
}

fn push_bool(
    parameters: &mut Vec<(&'static str, String)>,
    name: &'static str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        parameters.push((name, value.to_string()));
    }
}

fn push_number<T: ToString>(
    parameters: &mut Vec<(&'static str, String)>,
    name: &'static str,
    value: Option<T>,
) {
    if let Some(value) = value {
        parameters.push((name, value.to_string()));
    }
}
