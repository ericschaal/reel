mod curated;
mod http;
mod media;
mod navigation;
mod titles;
mod types;

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::Router;
use thiserror::Error as ThisError;
use tokio::sync::{Mutex, Semaphore};

use crate::{
    integration::Integration,
    jellyfin::Jellyfin,
    media::{SeasonNumber, TmdbId},
    seerr::{
        DiscoverMoviesQuery, DiscoverResponse, DiscoverSeriesQuery, GenreSliderItem, Seerr,
        TimeWindow, TrendingMediaType, TrendingQuery,
    },
};
use curated::{CuratedCategory, NETWORKS, STUDIOS};

pub use types::*;

const POPULARITY_DESCENDING: &str = "popularity.desc";
const LOCAL_MOVIES_CACHE_TTL: Duration = Duration::from_secs(30);

/// The deep module that turns provider-shaped discovery data into Reel's catalogue.
#[derive(Clone)]
pub struct Catalogue {
    seerr: Seerr,
    jellyfin: Jellyfin,
    local_movies: Arc<LocalMoviesCache>,
    titles: Arc<titles::TitleCache>,
}

struct LocalMoviesCache {
    value: Mutex<Option<CachedLocalMovies>>,
    refresh: Semaphore,
}

struct CachedLocalMovies {
    loaded_at: Instant,
    movies: Arc<HashMap<TmdbId, LocalCopy>>,
}

impl Catalogue {
    #[must_use]
    pub fn new(seerr: Seerr, jellyfin: Jellyfin) -> Self {
        Self {
            seerr,
            jellyfin,
            titles: Arc::new(titles::TitleCache::new()),
            local_movies: Arc::new(LocalMoviesCache {
                value: Mutex::new(None),
                refresh: Semaphore::new(1),
            }),
        }
    }

    pub async fn discover(&self, language: Option<&str>) -> Result<CatalogueResponse, Error> {
        let trending_query = trending_query(TrendingMediaType::All, 1, language);
        let movies_query = popular_movies_query(1, language);
        let series_query = popular_series_query(1, language);
        let (trending, movies, series, movie_genres, series_genres, local_movies) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.movies(&movies_query),
            self.seerr.series(&series_query),
            self.seerr.movie_genres(language),
            self.seerr.series_genres(language),
            self.local_movies(),
        );

        let mut builder = SurfaceBuilder::new(Surface::Discover, language);
        builder.local_movies(local_movies);
        builder.media(Collection::Trending, trending);
        builder.media(Collection::PopularMovies, movies);
        builder.media(Collection::PopularSeries, series);
        builder.genres(MediaKind::Movie, movie_genres);
        builder.curated(CategoryKind::Studio, STUDIOS);
        builder.genres(MediaKind::Series, series_genres);
        builder.curated(CategoryKind::Network, NETWORKS);
        builder.finish(self).await
    }

    pub async fn movies(&self, language: Option<&str>) -> Result<CatalogueResponse, Error> {
        let trending_query = trending_query(TrendingMediaType::Movie, 1, language);
        let popular_query = popular_movies_query(1, language);
        let (trending, popular, genres, local_movies) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.movies(&popular_query),
            self.seerr.movie_genres(language),
            self.local_movies(),
        );

        let mut builder = SurfaceBuilder::new(Surface::Movies, language);
        builder.local_movies(local_movies);
        builder.media(Collection::TrendingMovies, trending);
        builder.media(Collection::PopularMovies, popular);
        builder.genres(MediaKind::Movie, genres);
        builder.curated(CategoryKind::Studio, STUDIOS);
        builder.finish(self).await
    }

    pub async fn series(&self, language: Option<&str>) -> Result<CatalogueResponse, Error> {
        // A Jellyfin series item proves library presence, not that every episode is playable.
        // Episode-level availability belongs on the series hierarchy, not catalogue cards.
        let trending_query = trending_query(TrendingMediaType::Tv, 1, language);
        let popular_query = popular_series_query(1, language);
        let (trending, popular, genres) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.series(&popular_query),
            self.seerr.series_genres(language),
        );

        let mut builder = SurfaceBuilder::new(Surface::Series, language);
        builder.media(Collection::TrendingSeries, trending);
        builder.media(Collection::PopularSeries, popular);
        builder.genres(MediaKind::Series, genres);
        builder.curated(CategoryKind::Network, NETWORKS);
        builder.finish(self).await
    }

    pub async fn series_details(
        &self,
        tmdb_id: TmdbId,
        language: Option<&str>,
    ) -> Result<SeriesDetailsResponse, Error> {
        let metadata = self
            .title_metadata(titles::TitleKey {
                kind: MediaKind::Series,
                id: tmdb_id,
                language: language.map(str::to_owned),
            })
            .await?;
        let titles::TitleMetadata::Series(details) = metadata.as_ref() else {
            unreachable!("series metadata key")
        };
        Ok(media::normalize_series_details(details.clone()))
    }

    pub async fn series_details_with_initial_season(
        &self,
        tmdb_id: TmdbId,
        language: Option<&str>,
    ) -> Result<SeriesDetailsResponse, Error> {
        let mut response = self.series_details(tmdb_id, language).await?;
        let first_season_number = media::initial_season_number(&response.seasons);

        if let Some(season_number) = first_season_number {
            match self.season_details(tmdb_id, season_number, language).await {
                Ok(season) => response.initial_season = Some(season),
                Err(error) => {
                    tracing::warn!(
                        ?error,
                        %tmdb_id,
                        %season_number,
                        "initial season guide is unavailable"
                    );
                    response
                        .issues
                        .push(CatalogueIssue::upstream(Integration::Seerr, None));
                }
            }
        }
        Ok(response)
    }

    pub async fn movie_details(
        &self,
        tmdb_id: TmdbId,
        language: Option<&str>,
    ) -> Result<MovieDetailsResponse, Error> {
        let (metadata, local_movies) = tokio::join!(
            self.title_metadata(titles::TitleKey {
                kind: MediaKind::Movie,
                id: tmdb_id,
                language: language.map(str::to_owned),
            }),
            self.local_movies(),
        );
        let metadata = metadata?;
        let titles::TitleMetadata::Movie(details) = metadata.as_ref() else {
            unreachable!("movie metadata key")
        };
        let mut issues = Vec::new();
        let local_copy = unwrap_local_movies(local_movies, &mut issues)
            .get(&tmdb_id)
            .cloned();
        let availability = Availability::for_copy(local_copy.is_some(), issues.is_empty());
        let mut response = media::normalize_movie_details(details.clone(), availability);
        response.issues = issues;
        Ok(response)
    }

    pub async fn season_details(
        &self,
        tmdb_id: TmdbId,
        season_number: SeasonNumber,
        language: Option<&str>,
    ) -> Result<SeasonDetailsResponse, Error> {
        let (details, local_episodes) = tokio::join!(
            self.seerr.season_details(tmdb_id, season_number, language),
            media::local_episodes(&self.jellyfin, tmdb_id, season_number),
        );
        let details = details.map_err(map_details_error)?;
        let mut issues = Vec::new();
        let local_episodes = local_episodes.unwrap_or_else(|error| {
            tracing::warn!(%error, %tmdb_id, %season_number, "Jellyfin episode enrichment is unavailable");
            issues.push(CatalogueIssue::upstream(Integration::Jellyfin, None));
            HashMap::new()
        });
        Ok(media::normalize_season_details(
            tmdb_id,
            details,
            local_episodes,
            issues,
        ))
    }

    async fn collection(
        &self,
        collection: Collection,
        language: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CollectionResponse, Error> {
        let page = navigation::page_from_cursor(cursor, &collection, language)?;
        let local_movies = async {
            if collection.includes_movies() {
                Some(self.local_movies().await)
            } else {
                None
            }
        };
        let (response, local_movies) = tokio::join!(
            self.fetch_collection(collection, page, language),
            local_movies,
        );
        let response = response.map_err(|error| {
            tracing::warn!(%error, collection = collection.id(), "Seerr collection is unavailable");
            Error::Unavailable
        })?;

        let mut issues = Vec::new();
        let local_movies = local_movies
            .map(|result| unwrap_local_movies(result, &mut issues))
            .unwrap_or_default();
        let next = (response.page < response.total_pages)
            .then(|| navigation::next_page(&collection, language, response.page.saturating_add(1)));

        let title = collection_title(&response).unwrap_or_else(|| collection.title().into());
        let mut items = media::items(response.results, &local_movies, issues.is_empty());
        if !self.enrich_cards(&mut items, language).await {
            issues.push(CatalogueIssue::upstream(
                Integration::Seerr,
                Some(&collection.id()),
            ));
        }
        Ok(CollectionResponse {
            id: collection.id(),
            title,
            items,
            total_results: response.total_results,
            next,
            issues,
        })
    }

    pub fn manifest(&self, surface: Surface, language: Option<&str>) -> CatalogueManifest {
        CatalogueManifest {
            surface,
            rails: rail_specs(surface)
                .iter()
                .map(|spec| CatalogueRailDescriptor {
                    id: spec.id.into(),
                    title: spec.title.into(),
                    layout: spec.layout,
                    items_href: rail_href(surface, spec.id, language),
                    item_count_hint: spec.item_count_hint,
                })
                .collect(),
        }
    }

    pub async fn rail(
        &self,
        surface: Surface,
        rail_id: &str,
        language: Option<&str>,
    ) -> Result<CatalogueRailResponse, Error> {
        if !rail_specs(surface).iter().any(|spec| spec.id == rail_id) {
            return Err(Error::NotFound);
        }

        let mut builder = SurfaceBuilder::new(surface, language);
        match rail_id {
            "trending" => {
                let query = trending_query(TrendingMediaType::All, 1, language);
                let (items, local_movies) =
                    tokio::join!(self.seerr.trending(&query), self.local_movies(),);
                builder.local_movies(local_movies);
                builder.media(Collection::Trending, items);
            }
            "trending-movies" => {
                let query = trending_query(TrendingMediaType::Movie, 1, language);
                let (items, local_movies) =
                    tokio::join!(self.seerr.trending(&query), self.local_movies(),);
                builder.local_movies(local_movies);
                builder.media(Collection::TrendingMovies, items);
            }
            "trending-series" => builder.media(
                Collection::TrendingSeries,
                self.seerr
                    .trending(&trending_query(TrendingMediaType::Tv, 1, language))
                    .await,
            ),
            "popular-movies" => {
                let query = popular_movies_query(1, language);
                let (items, local_movies) =
                    tokio::join!(self.seerr.movies(&query), self.local_movies(),);
                builder.local_movies(local_movies);
                builder.media(Collection::PopularMovies, items);
            }
            "popular-series" => builder.media(
                Collection::PopularSeries,
                self.seerr.series(&popular_series_query(1, language)).await,
            ),
            "movie-genres" => {
                builder.genres(MediaKind::Movie, self.seerr.movie_genres(language).await)
            }
            "series-genres" => {
                builder.genres(MediaKind::Series, self.seerr.series_genres(language).await)
            }
            "studios" => builder.curated(CategoryKind::Studio, STUDIOS),
            "networks" => builder.curated(CategoryKind::Network, NETWORKS),
            _ => return Err(Error::NotFound),
        }

        let response = builder.finish(self).await?;
        let section = response
            .sections
            .into_iter()
            .next()
            .ok_or(Error::Unavailable)?;
        Ok(CatalogueRailResponse {
            section,
            issues: response.issues,
        })
    }

    async fn local_movies(&self) -> crate::jellyfin::Result<Arc<HashMap<TmdbId, LocalCopy>>> {
        if let Some(movies) = self.cached_local_movies().await {
            return Ok(movies);
        }

        let _refresh = self
            .local_movies
            .refresh
            .acquire()
            .await
            .expect("the private cache semaphore is never closed");

        // A concurrent request may have populated the cache while this task waited.
        if let Some(movies) = self.cached_local_movies().await {
            return Ok(movies);
        }

        let movies = Arc::new(media::local_movies(&self.jellyfin).await?);
        *self.local_movies.value.lock().await = Some(CachedLocalMovies {
            loaded_at: Instant::now(),
            movies: movies.clone(),
        });
        Ok(movies)
    }

    async fn cached_local_movies(&self) -> Option<Arc<HashMap<TmdbId, LocalCopy>>> {
        self.local_movies
            .value
            .lock()
            .await
            .as_ref()
            .filter(|cached| cached.loaded_at.elapsed() < LOCAL_MOVIES_CACHE_TTL)
            .map(|cached| cached.movies.clone())
    }

    async fn fetch_collection(
        &self,
        collection: Collection,
        page: u32,
        language: Option<&str>,
    ) -> crate::seerr::Result<DiscoverResponse> {
        match collection {
            Collection::Trending => {
                self.seerr
                    .trending(&trending_query(TrendingMediaType::All, page, language))
                    .await
            }
            Collection::TrendingMovies => {
                self.seerr
                    .trending(&trending_query(TrendingMediaType::Movie, page, language))
                    .await
            }
            Collection::TrendingSeries => {
                self.seerr
                    .trending(&trending_query(TrendingMediaType::Tv, page, language))
                    .await
            }
            Collection::PopularMovies => {
                self.seerr
                    .movies(&popular_movies_query(page, language))
                    .await
            }
            Collection::PopularSeries => {
                self.seerr
                    .series(&popular_series_query(page, language))
                    .await
            }
            Collection::MovieGenre(id) => {
                self.seerr.movies_by_genre(id, Some(page), language).await
            }
            Collection::SeriesGenre(id) => {
                self.seerr.series_by_genre(id, Some(page), language).await
            }
            Collection::Studio(id) => self.seerr.movies_by_studio(id, Some(page), language).await,
            Collection::Network(id) => self.seerr.series_by_network(id, Some(page), language).await,
        }
    }
}

#[derive(Clone, Copy)]
struct RailSpec {
    id: &'static str,
    title: &'static str,
    layout: SectionLayout,
    item_count_hint: usize,
}

const DISCOVER_RAILS: &[RailSpec] = &[
    RailSpec::poster("trending", "Trending Now", 20),
    RailSpec::poster("popular-movies", "Popular Movies", 20),
    RailSpec::poster("popular-series", "Popular Series", 20),
    RailSpec::backdrop("movie-genres", "Movie Genres", 19),
    RailSpec::backdrop("studios", "Studios", STUDIOS.len()),
    RailSpec::backdrop("series-genres", "Series Genres", 16),
    RailSpec::backdrop("networks", "Networks", NETWORKS.len()),
];
const MOVIE_RAILS: &[RailSpec] = &[
    RailSpec::poster("trending-movies", "Trending Movies", 20),
    RailSpec::poster("popular-movies", "Popular Movies", 20),
    RailSpec::backdrop("movie-genres", "Genres", 19),
    RailSpec::backdrop("studios", "Studios", STUDIOS.len()),
];
const SERIES_RAILS: &[RailSpec] = &[
    RailSpec::poster("trending-series", "Trending Series", 20),
    RailSpec::poster("popular-series", "Popular Series", 20),
    RailSpec::backdrop("series-genres", "Genres", 16),
    RailSpec::backdrop("networks", "Networks", NETWORKS.len()),
];

impl RailSpec {
    const fn poster(id: &'static str, title: &'static str, item_count_hint: usize) -> Self {
        Self {
            id,
            title,
            layout: SectionLayout::Poster,
            item_count_hint,
        }
    }

    const fn backdrop(id: &'static str, title: &'static str, item_count_hint: usize) -> Self {
        Self {
            id,
            title,
            layout: SectionLayout::Backdrop,
            item_count_hint,
        }
    }
}

fn rail_specs(surface: Surface) -> &'static [RailSpec] {
    match surface {
        Surface::Discover => DISCOVER_RAILS,
        Surface::Movies => MOVIE_RAILS,
        Surface::Series => SERIES_RAILS,
    }
}

fn rail_href(surface: Surface, rail_id: &str, language: Option<&str>) -> String {
    let path = format!("/v1/catalogue/{}/rails/{rail_id}", surface.as_str());
    let Some(language) = language else {
        return path;
    };
    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("language", language)
        .finish();
    format!("{path}?{query}")
}

pub fn router(catalogue: Catalogue) -> Router {
    http::router(catalogue)
}

#[derive(Debug, ThisError)]
pub enum Error {
    #[error("query parameters are invalid")]
    InvalidQuery,
    #[error("continuation cursor is invalid")]
    InvalidCursor,
    #[error("catalogue resource was not found")]
    NotFound,
    #[error("media was not found")]
    MediaNotFound,
    #[error("catalogue is unavailable")]
    Unavailable,
}

fn map_details_error(error: crate::seerr::Error) -> Error {
    match error {
        crate::seerr::Error::Rejected { status, .. }
            if status == reqwest::StatusCode::NOT_FOUND =>
        {
            Error::MediaNotFound
        }
        error => {
            tracing::warn!(%error, "Seerr title details are unavailable");
            Error::Unavailable
        }
    }
}

struct SurfaceBuilder<'a> {
    surface: Surface,
    language: Option<&'a str>,
    local_movies: Arc<HashMap<TmdbId, LocalCopy>>,
    sections: Vec<CatalogueSection>,
    issues: Vec<CatalogueIssue>,
}

impl<'a> SurfaceBuilder<'a> {
    fn new(surface: Surface, language: Option<&'a str>) -> Self {
        Self {
            surface,
            language,
            local_movies: Arc::new(HashMap::new()),
            sections: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn local_movies(&mut self, result: crate::jellyfin::Result<Arc<HashMap<TmdbId, LocalCopy>>>) {
        self.local_movies = unwrap_local_movies(result, &mut self.issues);
    }

    fn media(&mut self, collection: Collection, result: crate::seerr::Result<DiscoverResponse>) {
        match result {
            Ok(response) => self.sections.push(CatalogueSection {
                id: collection.id(),
                title: collection.title().into(),
                layout: SectionLayout::Poster,
                href: Some(navigation::first_page(&collection, self.language)),
                items: media::items(
                    response.results,
                    &self.local_movies,
                    !self
                        .issues
                        .iter()
                        .any(|issue| issue.source == CatalogueSource::Jellyfin),
                ),
            }),
            Err(error) => self.seerr_issue(collection.id(), error),
        }
    }

    fn genres(
        &mut self,
        media_kind: MediaKind,
        result: crate::seerr::Result<Vec<GenreSliderItem>>,
    ) {
        let (section_id, section_title) = match (media_kind, self.surface) {
            (MediaKind::Movie, Surface::Discover) => ("movie-genres", "Movie Genres"),
            (MediaKind::Series, Surface::Discover) => ("series-genres", "Series Genres"),
            (MediaKind::Movie, _) => ("movie-genres", "Genres"),
            (MediaKind::Series, _) => ("series-genres", "Genres"),
        };
        match result {
            Ok(genres) => self.category_section(
                section_id,
                section_title,
                genres
                    .into_iter()
                    .map(|genre| CategoryCard {
                        id: genre.id,
                        title: genre.name,
                        media_kind,
                        category_kind: CategoryKind::Genre,
                        href: navigation::first_page(
                            &Collection::genre(media_kind, genre.id),
                            self.language,
                        ),
                        images: genre
                            .backdrops
                            .into_iter()
                            .map(media::backdrop_url)
                            .collect(),
                    })
                    .collect(),
            ),
            Err(error) => self.seerr_issue(section_id, error),
        }
    }

    fn curated(&mut self, kind: CategoryKind, categories: &[CuratedCategory]) {
        let (section_id, section_title, media_kind) = match kind {
            CategoryKind::Studio => ("studios", "Studios", MediaKind::Movie),
            CategoryKind::Network => ("networks", "Networks", MediaKind::Series),
            CategoryKind::Genre => unreachable!("genres are provided by Seerr"),
        };
        self.category_section(
            section_id,
            section_title,
            categories
                .iter()
                .map(|category| CategoryCard {
                    id: category.id,
                    title: category.title.into(),
                    media_kind,
                    category_kind: kind,
                    href: navigation::first_page(
                        &match kind {
                            CategoryKind::Studio => Collection::Studio(category.id),
                            CategoryKind::Network => Collection::Network(category.id),
                            CategoryKind::Genre => unreachable!("genres are provided by Seerr"),
                        },
                        self.language,
                    ),
                    images: vec![category.image.into()],
                })
                .collect(),
        );
    }

    fn category_section(&mut self, id: &str, title: &str, cards: Vec<CategoryCard>) {
        self.sections.push(CatalogueSection {
            id: id.into(),
            title: title.into(),
            layout: SectionLayout::Backdrop,
            href: None,
            items: cards.into_iter().map(CatalogueItem::Category).collect(),
        });
    }

    fn seerr_issue(&mut self, section_id: impl Into<String>, error: crate::seerr::Error) {
        let section_id = section_id.into();
        tracing::warn!(%error, section_id, "Seerr catalogue section is unavailable");
        self.issues.push(CatalogueIssue::upstream(
            Integration::Seerr,
            Some(&section_id),
        ));
    }

    async fn finish(mut self, catalogue: &Catalogue) -> Result<CatalogueResponse, Error> {
        for section in &mut self.sections {
            if !catalogue
                .enrich_cards(&mut section.items, self.language)
                .await
            {
                self.issues.push(CatalogueIssue::upstream(
                    Integration::Seerr,
                    Some(&section.id),
                ));
            }
        }
        if self.sections.is_empty() {
            Err(Error::Unavailable)
        } else {
            Ok(CatalogueResponse {
                surface: self.surface,
                sections: self.sections,
                issues: self.issues,
            })
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Collection {
    Trending,
    TrendingMovies,
    TrendingSeries,
    PopularMovies,
    PopularSeries,
    MovieGenre(i64),
    SeriesGenre(i64),
    Studio(i64),
    Network(i64),
}

impl Collection {
    fn named(id: &str) -> Option<Self> {
        match id {
            "trending" => Some(Self::Trending),
            "trending-movies" => Some(Self::TrendingMovies),
            "trending-series" => Some(Self::TrendingSeries),
            "popular-movies" => Some(Self::PopularMovies),
            "popular-series" => Some(Self::PopularSeries),
            _ => None,
        }
    }

    fn category(kind: &str, id: i64) -> Option<Self> {
        match kind {
            "movie-genres" => Some(Self::MovieGenre(id)),
            "series-genres" => Some(Self::SeriesGenre(id)),
            "studios" => Some(Self::Studio(id)),
            "networks" => Some(Self::Network(id)),
            _ => None,
        }
    }

    const fn genre(media_kind: MediaKind, id: i64) -> Self {
        match media_kind {
            MediaKind::Movie => Self::MovieGenre(id),
            MediaKind::Series => Self::SeriesGenre(id),
        }
    }

    fn id(self) -> String {
        match self {
            Self::Trending => "trending".into(),
            Self::TrendingMovies => "trending-movies".into(),
            Self::TrendingSeries => "trending-series".into(),
            Self::PopularMovies => "popular-movies".into(),
            Self::PopularSeries => "popular-series".into(),
            Self::MovieGenre(id) => format!("movie-genres/{id}"),
            Self::SeriesGenre(id) => format!("series-genres/{id}"),
            Self::Studio(id) => format!("studios/{id}"),
            Self::Network(id) => format!("networks/{id}"),
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Trending => "Trending Now",
            Self::TrendingMovies => "Trending Movies",
            Self::TrendingSeries => "Trending Series",
            Self::PopularMovies => "Popular Movies",
            Self::PopularSeries => "Popular Series",
            Self::MovieGenre(_) => "Movie Genre",
            Self::SeriesGenre(_) => "Series Genre",
            Self::Studio(_) => "Studio",
            Self::Network(_) => "Network",
        }
    }

    const fn media_kind(self) -> MediaKind {
        match self {
            Self::TrendingSeries
            | Self::PopularSeries
            | Self::SeriesGenre(_)
            | Self::Network(_) => MediaKind::Series,
            _ => MediaKind::Movie,
        }
    }

    const fn includes_movies(self) -> bool {
        !matches!(self.media_kind(), MediaKind::Series) || matches!(self, Self::Trending)
    }
}

fn popular_movies_query(page: u32, language: Option<&str>) -> DiscoverMoviesQuery {
    DiscoverMoviesQuery {
        page: Some(page),
        sort_by: Some(POPULARITY_DESCENDING.into()),
        language: language.map(str::to_owned),
        ..DiscoverMoviesQuery::default()
    }
}

fn popular_series_query(page: u32, language: Option<&str>) -> DiscoverSeriesQuery {
    DiscoverSeriesQuery {
        page: Some(page),
        sort_by: Some(POPULARITY_DESCENDING.into()),
        language: language.map(str::to_owned),
        ..DiscoverSeriesQuery::default()
    }
}

fn trending_query(
    media_type: TrendingMediaType,
    page: u32,
    language: Option<&str>,
) -> TrendingQuery {
    TrendingQuery {
        page: Some(page),
        media_type: Some(media_type),
        time_window: Some(TimeWindow::Week),
        language: language.map(str::to_owned),
    }
}

fn unwrap_local_movies(
    result: crate::jellyfin::Result<Arc<HashMap<TmdbId, LocalCopy>>>,
    issues: &mut Vec<CatalogueIssue>,
) -> Arc<HashMap<TmdbId, LocalCopy>> {
    result.unwrap_or_else(|error| {
        tracing::warn!(%error, "Jellyfin catalogue enrichment is unavailable");
        issues.push(CatalogueIssue::upstream(Integration::Jellyfin, None));
        Arc::new(HashMap::new())
    })
}

fn collection_title(response: &DiscoverResponse) -> Option<String> {
    response
        .genre
        .as_ref()
        .map(|genre| genre.name.clone())
        .or_else(|| response.studio.as_ref().map(|studio| studio.name.clone()))
        .or_else(|| {
            response
                .network
                .as_ref()
                .map(|network| network.name.clone())
        })
}
