use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    Movie,
    Tv,
    Person,
    Collection,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResult {
    pub id: i64,
    pub media_type: MediaType,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub name: Option<String>,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub profile_path: Option<String>,
    pub release_date: Option<String>,
    pub first_air_date: Option<String>,
    pub original_language: Option<String>,
    #[serde(default)]
    pub origin_country: Vec<String>,
    #[serde(default)]
    pub genre_ids: Vec<i64>,
    pub popularity: Option<f64>,
    pub vote_count: Option<u64>,
    pub vote_average: Option<f64>,
    pub adult: Option<bool>,
    pub media_info: Option<MediaInfo>,
}

impl DiscoverResult {
    pub fn display_title(&self) -> Option<&str> {
        self.title.as_deref().or(self.name.as_deref())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub id: i64,
    pub tmdb_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub media_type: Option<MediaType>,
    pub status: Option<u8>,
    pub status4k: Option<u8>,
    pub service_id: Option<i64>,
    pub service_id4k: Option<i64>,
    pub external_service_id: Option<i64>,
    pub external_service_id4k: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverResponse {
    pub page: u32,
    pub total_pages: u32,
    pub total_results: u64,
    #[serde(default)]
    pub results: Vec<DiscoverResult>,
    pub genre: Option<Genre>,
    pub studio: Option<Company>,
    pub network: Option<Company>,
    pub language: Option<Language>,
    #[serde(default)]
    pub keywords: Vec<Keyword>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GenreSliderItem {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub backdrops: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Genre {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub logo_path: Option<String>,
    pub origin_country: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Keyword {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Language {
    pub iso_639_1: String,
    pub english_name: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrendingQuery {
    pub page: Option<u32>,
    pub media_type: Option<TrendingMediaType>,
    pub time_window: Option<TimeWindow>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TrendingMediaType {
    All,
    Movie,
    Tv,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TimeWindow {
    Day,
    Week,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverMoviesQuery {
    pub page: Option<u32>,
    pub language: Option<String>,
    pub sort_by: Option<String>,
    pub primary_release_date_gte: Option<String>,
    pub primary_release_date_lte: Option<String>,
    pub studio: Option<String>,
    pub genre: Option<String>,
    pub keywords: Option<String>,
    pub exclude_keywords: Option<String>,
    pub with_runtime_gte: Option<u32>,
    pub with_runtime_lte: Option<u32>,
    pub vote_average_gte: Option<f32>,
    pub vote_average_lte: Option<f32>,
    pub vote_count_gte: Option<u32>,
    pub vote_count_lte: Option<u32>,
    pub watch_providers: Option<String>,
    pub watch_region: Option<String>,
    pub certification: Option<String>,
    pub certification_gte: Option<String>,
    pub certification_lte: Option<String>,
    pub certification_country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverSeriesQuery {
    pub page: Option<u32>,
    pub language: Option<String>,
    pub sort_by: Option<String>,
    pub first_air_date_gte: Option<String>,
    pub first_air_date_lte: Option<String>,
    pub network: Option<i64>,
    pub genre: Option<String>,
    pub keywords: Option<String>,
    pub exclude_keywords: Option<String>,
    pub with_runtime_gte: Option<u32>,
    pub with_runtime_lte: Option<u32>,
    pub vote_average_gte: Option<f32>,
    pub vote_average_lte: Option<f32>,
    pub vote_count_gte: Option<u32>,
    pub vote_count_lte: Option<u32>,
    pub watch_providers: Option<String>,
    pub watch_region: Option<String>,
    pub status: Option<String>,
    pub certification: Option<String>,
    pub certification_gte: Option<String>,
    pub certification_lte: Option<String>,
    pub certification_country: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub query: String,
    pub page: Option<u32>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetails {
    pub id: i64,
    pub imdb_id: Option<String>,
    pub title: String,
    pub original_title: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub release_date: Option<String>,
    pub runtime: Option<u32>,
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub production_companies: Vec<Company>,
    pub media_info: Option<MediaInfo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeriesDetails {
    pub id: i64,
    pub name: String,
    pub original_name: Option<String>,
    pub overview: Option<String>,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub first_air_date: Option<String>,
    pub number_of_seasons: Option<u32>,
    pub number_of_episodes: Option<u32>,
    #[serde(default)]
    pub episode_run_time: Vec<u32>,
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub genres: Vec<Genre>,
    #[serde(default)]
    pub networks: Vec<Company>,
    #[serde(default)]
    pub seasons: Vec<Season>,
    pub media_info: Option<MediaInfo>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub id: i64,
    pub name: String,
    pub season_number: i32,
    pub episode_count: Option<u32>,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeasonDetails {
    pub id: i64,
    pub name: String,
    pub season_number: i32,
    pub air_date: Option<String>,
    pub poster_path: Option<String>,
    pub overview: Option<String>,
    #[serde(default)]
    pub episodes: Vec<EpisodeDetails>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeDetails {
    pub id: i64,
    pub name: String,
    pub episode_number: i32,
    pub season_number: i32,
    pub air_date: Option<String>,
    pub overview: Option<String>,
    pub still_path: Option<String>,
    pub vote_average: Option<f64>,
}
