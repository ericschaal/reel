use serde::{Deserialize, Serialize};

use crate::integration::Integration;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Surface {
    Discover,
    Movies,
    Series,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueResponse {
    pub surface: Surface,
    pub sections: Vec<CatalogueSection>,
    #[serde(default)]
    pub issues: Vec<CatalogueIssue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CollectionResponse {
    pub id: String,
    pub title: String,
    pub items: Vec<CatalogueItem>,
    pub total_results: u64,
    pub next: Option<String>,
    #[serde(default)]
    pub issues: Vec<CatalogueIssue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueSection {
    pub id: String,
    pub title: String,
    pub layout: SectionLayout,
    pub href: Option<String>,
    pub items: Vec<CatalogueItem>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SectionLayout {
    Poster,
    Backdrop,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CatalogueItem {
    Movie(MediaCard),
    Series(MediaCard),
    Category(CategoryCard),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaCard {
    pub id: String,
    pub tmdb_id: i64,
    pub title: String,
    pub overview: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub images: Images,
    pub local_copy: Option<LocalCopy>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Images {
    pub poster: Option<String>,
    pub backdrop: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalCopy {
    pub jellyfin_item_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCard {
    pub id: i64,
    pub title: String,
    pub media_kind: MediaKind,
    pub category_kind: CategoryKind,
    pub href: String,
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CategoryKind {
    Genre,
    Studio,
    Network,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Movie,
    Series,
}

impl MediaKind {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Series => "series",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueIssue {
    pub source: CatalogueSource,
    pub section_id: Option<String>,
    pub code: CatalogueIssueCode,
}

impl CatalogueIssue {
    pub(super) fn upstream(integration: Integration, section_id: Option<&str>) -> Self {
        Self {
            source: match integration {
                Integration::Jellyfin => CatalogueSource::Jellyfin,
                Integration::Seerr => CatalogueSource::Seerr,
            },
            section_id: section_id.map(Into::into),
            code: CatalogueIssueCode::UpstreamUnavailable,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CatalogueSource {
    Jellyfin,
    Seerr,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CatalogueIssueCode {
    UpstreamUnavailable,
}
