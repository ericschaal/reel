mod curated;
mod http;
mod media;
mod navigation;
mod types;

use std::collections::HashMap;

use axum::Router;

use crate::{
    integration::Integration,
    jellyfin::Jellyfin,
    seerr::{
        DiscoverMoviesQuery, DiscoverResponse, DiscoverSeriesQuery, GenreSliderItem, Seerr,
        TimeWindow, TrendingMediaType, TrendingQuery,
    },
};
use curated::{CuratedCategory, NETWORKS, STUDIOS};

pub use types::*;

const POPULARITY_DESCENDING: &str = "popularity.desc";

/// The deep module that turns provider-shaped discovery data into Reel's catalogue.
#[derive(Clone)]
pub struct Catalogue {
    seerr: Seerr,
    jellyfin: Jellyfin,
}

impl Catalogue {
    pub fn new(seerr: Seerr, jellyfin: Jellyfin) -> Self {
        Self { seerr, jellyfin }
    }

    pub async fn discover(&self, language: Option<String>) -> Result<CatalogueResponse, Error> {
        let trending_query = trending_query(TrendingMediaType::All, 1, language.clone());
        let movies_query = popular_movies_query(1, language.clone());
        let series_query = popular_series_query(1, language.clone());
        let (trending, movies, series, movie_genres, series_genres, local_movies) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.movies(&movies_query),
            self.seerr.series(&series_query),
            self.seerr.movie_genres(language.as_deref()),
            self.seerr.series_genres(language.as_deref()),
            media::local_movies(&self.jellyfin),
        );

        let mut builder = SurfaceBuilder::new(Surface::Discover, language.as_deref());
        builder.local_movies(local_movies);
        builder.media(Collection::Trending, trending);
        builder.media(Collection::PopularMovies, movies);
        builder.media(Collection::PopularSeries, series);
        builder.genres(MediaKind::Movie, movie_genres);
        builder.curated(CategoryKind::Studio, STUDIOS);
        builder.genres(MediaKind::Series, series_genres);
        builder.curated(CategoryKind::Network, NETWORKS);
        builder.finish()
    }

    pub async fn movies(&self, language: Option<String>) -> Result<CatalogueResponse, Error> {
        let trending_query = trending_query(TrendingMediaType::Movie, 1, language.clone());
        let popular_query = popular_movies_query(1, language.clone());
        let (trending, popular, genres, local_movies) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.movies(&popular_query),
            self.seerr.movie_genres(language.as_deref()),
            media::local_movies(&self.jellyfin),
        );

        let mut builder = SurfaceBuilder::new(Surface::Movies, language.as_deref());
        builder.local_movies(local_movies);
        builder.media(Collection::TrendingMovies, trending);
        builder.media(Collection::PopularMovies, popular);
        builder.genres(MediaKind::Movie, genres);
        builder.curated(CategoryKind::Studio, STUDIOS);
        builder.finish()
    }

    pub async fn series(&self, language: Option<String>) -> Result<CatalogueResponse, Error> {
        // A Jellyfin series item proves library presence, not that every episode is playable.
        // Episode-level availability belongs on the series hierarchy, not catalogue cards.
        let trending_query = trending_query(TrendingMediaType::Tv, 1, language.clone());
        let popular_query = popular_series_query(1, language.clone());
        let (trending, popular, genres) = tokio::join!(
            self.seerr.trending(&trending_query),
            self.seerr.series(&popular_query),
            self.seerr.series_genres(language.as_deref()),
        );

        let mut builder = SurfaceBuilder::new(Surface::Series, language.as_deref());
        builder.media(Collection::TrendingSeries, trending);
        builder.media(Collection::PopularSeries, popular);
        builder.genres(MediaKind::Series, genres);
        builder.curated(CategoryKind::Network, NETWORKS);
        builder.finish()
    }

    pub async fn series_details(
        &self,
        tmdb_id: i64,
        language: Option<String>,
    ) -> Result<SeriesDetailsResponse, Error> {
        let details = self
            .seerr
            .series_details(tmdb_id, language.as_deref())
            .await
            .map_err(map_details_error)?;
        Ok(media::normalize_series_details(details))
    }

    pub async fn season_details(
        &self,
        tmdb_id: i64,
        season_number: i32,
        language: Option<String>,
    ) -> Result<SeasonDetailsResponse, Error> {
        let (details, local_episodes) = tokio::join!(
            self.seerr
                .season_details(tmdb_id, season_number, language.as_deref()),
            media::local_episodes(&self.jellyfin, tmdb_id, season_number),
        );
        let details = details.map_err(map_details_error)?;
        let mut issues = Vec::new();
        let local_episodes = local_episodes.unwrap_or_else(|error| {
            tracing::warn!(%error, tmdb_id, season_number, "Jellyfin episode enrichment is unavailable");
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
        language: Option<String>,
        cursor: Option<String>,
    ) -> Result<CollectionResponse, Error> {
        let page =
            navigation::page_from_cursor(cursor.as_deref(), &collection, language.as_deref())?;
        let local_movies = async {
            if collection.includes_movies() {
                Some(media::local_movies(&self.jellyfin).await)
            } else {
                None
            }
        };
        let (response, local_movies) = tokio::join!(
            self.fetch_collection(collection, page, language.as_deref()),
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
        let next = (response.page < response.total_pages).then(|| {
            navigation::next_page(
                &collection,
                language.as_deref(),
                response.page.saturating_add(1),
            )
        });

        Ok(CollectionResponse {
            id: collection.id(),
            title: collection_title(&response).unwrap_or_else(|| collection.title().into()),
            items: media::items(response.results, &local_movies),
            total_results: response.total_results,
            next,
            issues,
        })
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
                    .trending(&trending_query(
                        TrendingMediaType::All,
                        page,
                        language.map(str::to_owned),
                    ))
                    .await
            }
            Collection::TrendingMovies => {
                self.seerr
                    .trending(&trending_query(
                        TrendingMediaType::Movie,
                        page,
                        language.map(str::to_owned),
                    ))
                    .await
            }
            Collection::TrendingSeries => {
                self.seerr
                    .trending(&trending_query(
                        TrendingMediaType::Tv,
                        page,
                        language.map(str::to_owned),
                    ))
                    .await
            }
            Collection::PopularMovies => {
                self.seerr
                    .movies(&popular_movies_query(page, language.map(str::to_owned)))
                    .await
            }
            Collection::PopularSeries => {
                self.seerr
                    .series(&popular_series_query(page, language.map(str::to_owned)))
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

pub fn router(catalogue: Catalogue) -> Router {
    http::router(catalogue)
}

#[derive(Debug)]
pub enum Error {
    InvalidCursor,
    NotFound,
    MediaNotFound,
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
    local_movies: HashMap<i64, LocalCopy>,
    sections: Vec<CatalogueSection>,
    issues: Vec<CatalogueIssue>,
}

impl<'a> SurfaceBuilder<'a> {
    fn new(surface: Surface, language: Option<&'a str>) -> Self {
        Self {
            surface,
            language,
            local_movies: HashMap::new(),
            sections: Vec::new(),
            issues: Vec::new(),
        }
    }

    fn local_movies(&mut self, result: crate::jellyfin::Result<HashMap<i64, LocalCopy>>) {
        self.local_movies = unwrap_local_movies(result, &mut self.issues);
    }

    fn media(&mut self, collection: Collection, result: crate::seerr::Result<DiscoverResponse>) {
        match result {
            Ok(response) => self.sections.push(CatalogueSection {
                id: collection.id(),
                title: collection.title().into(),
                layout: SectionLayout::Poster,
                href: Some(navigation::first_page(&collection, self.language)),
                items: media::items(response.results, &self.local_movies),
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

    fn finish(self) -> Result<CatalogueResponse, Error> {
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

fn popular_movies_query(page: u32, language: Option<String>) -> DiscoverMoviesQuery {
    DiscoverMoviesQuery {
        page: Some(page),
        sort_by: Some(POPULARITY_DESCENDING.into()),
        language,
        ..DiscoverMoviesQuery::default()
    }
}

fn popular_series_query(page: u32, language: Option<String>) -> DiscoverSeriesQuery {
    DiscoverSeriesQuery {
        page: Some(page),
        sort_by: Some(POPULARITY_DESCENDING.into()),
        language,
        ..DiscoverSeriesQuery::default()
    }
}

fn trending_query(
    media_type: TrendingMediaType,
    page: u32,
    language: Option<String>,
) -> TrendingQuery {
    TrendingQuery {
        page: Some(page),
        media_type: Some(media_type),
        time_window: Some(TimeWindow::Week),
        language,
    }
}

fn unwrap_local_movies(
    result: crate::jellyfin::Result<HashMap<i64, LocalCopy>>,
    issues: &mut Vec<CatalogueIssue>,
) -> HashMap<i64, LocalCopy> {
    result.unwrap_or_else(|error| {
        tracing::warn!(%error, "Jellyfin catalogue enrichment is unavailable");
        issues.push(CatalogueIssue::upstream(Integration::Jellyfin, None));
        HashMap::new()
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
