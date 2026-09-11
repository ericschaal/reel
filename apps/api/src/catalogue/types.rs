use serde::{Deserialize, Serialize};

use crate::{
    integration::Integration,
    media::{SeasonNumber, TmdbId},
};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Surface {
    Discover,
    Movies,
    Series,
}

impl Surface {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Discover => "discover",
            Self::Movies => "movies",
            Self::Series => "series",
        }
    }
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
pub struct CatalogueManifest {
    pub surface: Surface,
    pub rails: Vec<CatalogueRailDescriptor>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueRailDescriptor {
    pub id: String,
    pub title: String,
    pub layout: SectionLayout,
    pub items_href: String,
    pub item_count_hint: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogueRailResponse {
    pub section: CatalogueSection,
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
    pub runtime_minutes: Option<u32>,
    pub number_of_seasons: Option<u32>,
    pub id: String,
    pub tmdb_id: TmdbId,
    pub title: String,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub images: Images,
    pub availability: Availability,
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

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetailsResponse {
    pub kind: MediaKind,
    #[serde(default)]
    pub issues: Vec<CatalogueIssue>,
    pub id: String,
    pub tmdb_id: TmdbId,
    pub imdb_id: Option<String>,
    pub title: String,
    pub overview: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub runtime_minutes: Option<u32>,
    pub images: Images,
    pub availability: Availability,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetailsResponse {
    pub kind: MediaKind,
    pub availability: Availability,
    pub id: String,
    pub tmdb_id: TmdbId,
    pub imdb_id: Option<String>,
    pub title: String,
    pub overview: Option<String>,
    pub year: Option<i32>,
    pub rating: Option<f64>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    pub images: Images,
    pub seasons: Vec<SeasonSummary>,
    #[serde(default)]
    pub issues: Vec<CatalogueIssue>,
    pub initial_season: Option<SeasonDetailsResponse>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonSummary {
    pub id: String,
    pub season_number: SeasonNumber,
    pub title: String,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub episode_count: Option<u32>,
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonDetailsResponse {
    pub id: String,
    pub series_tmdb_id: TmdbId,
    pub season_number: SeasonNumber,
    pub title: String,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub poster: Option<String>,
    pub episodes: Vec<Episode>,
    #[serde(default)]
    pub issues: Vec<CatalogueIssue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub id: String,
    pub tmdb_id: TmdbId,
    pub season_number: SeasonNumber,
    pub episode_number: i32,
    pub title: String,
    pub overview: Option<String>,
    pub air_date: Option<String>,
    pub rating: Option<f64>,
    pub still: Option<String>,
    pub runtime_minutes: Option<u32>,
    pub availability: Availability,
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

#[derive(Debug, Clone, Copy, Hash, Deserialize, Serialize, PartialEq, Eq)]
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
                Integration::AioStreams => {
                    unreachable!("AIOStreams does not produce catalogue issues")
                }
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

/// Compact card facts. No artwork, synopsis, availability or episode guide.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TitleSummary {
    Movie {
        id: String,
        runtime_minutes: Option<u32>,
    },
    Series {
        id: String,
        number_of_seasons: Option<u32>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TitleSummariesResponse {
    pub items: Vec<TitleSummary>,
    pub issues: Vec<TitleSummaryIssue>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TitleSummaryIssue {
    pub id: String,
    pub code: TitleSummaryIssueCode,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TitleSummaryIssueCode {
    MediaNotFound,
    CatalogueUnavailable,
}

/// Local-library knowledge, independent of remote playback and acquisition.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    Local,
    NotLocal,
    Unknown,
    EpisodeBased,
}

impl Availability {
    pub(super) fn for_copy(present: bool, known: bool) -> Self {
        if present {
            Self::Local
        } else if known {
            Self::NotLocal
        } else {
            Self::Unknown
        }
    }
}
