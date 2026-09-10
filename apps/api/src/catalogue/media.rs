use std::collections::HashMap;

use crate::{
    jellyfin::{Item, ItemType, ItemsQuery, Jellyfin},
    media::{SeasonNumber, TmdbId},
    seerr::{DiscoverResult, MediaType, MovieDetails, SeasonDetails, SeriesDetails},
};

use super::{
    Availability, CatalogueIssue, CatalogueItem, Episode, Images, LocalCopy, MediaCard, MediaKind,
    MovieDetailsResponse, SeasonDetailsResponse, SeasonSummary, SeriesDetailsResponse,
};

const TMDB_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p";

pub(super) async fn local_movies(
    jellyfin: &Jellyfin,
) -> crate::jellyfin::Result<HashMap<TmdbId, LocalCopy>> {
    let response = jellyfin
        .items(&ItemsQuery {
            include_item_types: vec![ItemType::Movie],
            recursive: Some(true),
            ..ItemsQuery::default()
        })
        .await?;
    Ok(response
        .items
        .into_iter()
        .filter_map(|item| {
            let tmdb_id = item
                .provider_ids
                .iter()
                .find(|(provider, _)| provider.eq_ignore_ascii_case("tmdb"))?
                .1
                .parse::<TmdbId>()
                .ok()?;
            Some((
                tmdb_id,
                LocalCopy {
                    jellyfin_item_id: item.id,
                },
            ))
        })
        .collect())
}

#[derive(Debug, Clone)]
pub(super) struct LocalEpisode {
    pub runtime_minutes: Option<u32>,
}

pub(super) async fn local_episodes(
    jellyfin: &Jellyfin,
    tmdb_id: TmdbId,
    season_number: SeasonNumber,
) -> crate::jellyfin::Result<HashMap<i32, LocalEpisode>> {
    let series = jellyfin
        .items(&ItemsQuery {
            include_item_types: vec![ItemType::Series],
            recursive: Some(true),
            ..ItemsQuery::default()
        })
        .await?
        .items
        .into_iter()
        .find(|item| provider_id(item, "tmdb").and_then(|id| id.parse().ok()) == Some(tmdb_id));
    let Some(series) = series else {
        return Ok(HashMap::new());
    };

    let season = jellyfin
        .seasons(&series.id)
        .await?
        .items
        .into_iter()
        .find(|season| season.index_number == Some(season_number.get()));
    let Some(season) = season else {
        return Ok(HashMap::new());
    };

    Ok(jellyfin
        .episodes(&series.id, Some(&season.id))
        .await?
        .items
        .into_iter()
        .filter_map(|episode| {
            let episode_number = episode.index_number?;
            Some((
                episode_number,
                LocalEpisode {
                    runtime_minutes: episode
                        .run_time_ticks
                        .and_then(|ticks| u32::try_from(ticks / 600_000_000).ok()),
                },
            ))
        })
        .collect())
}

fn provider_id<'a>(item: &'a Item, provider: &str) -> Option<&'a str> {
    item.provider_ids
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(provider))
        .map(|(_, id)| id.as_str())
}

pub(super) fn items(
    items: Vec<DiscoverResult>,
    local_movies: &HashMap<TmdbId, LocalCopy>,
    availability_known: bool,
) -> Vec<CatalogueItem> {
    items
        .into_iter()
        .filter_map(|item| normalize(item, local_movies, availability_known))
        .collect()
}

pub(super) fn backdrop_url(path: String) -> String {
    image_url("w780", path)
}

pub(super) fn normalize_series_details(details: SeriesDetails) -> SeriesDetailsResponse {
    SeriesDetailsResponse {
        kind: MediaKind::Series,
        availability: Availability::EpisodeBased,
        id: format!("tmdb:series:{}", details.id),
        tmdb_id: details.id,
        title: details.name,
        overview: details.overview,
        year: year(details.first_air_date.as_deref()),
        rating: details.vote_average,
        number_of_seasons: details.number_of_seasons,
        number_of_episodes: details.number_of_episodes,
        images: Images {
            poster: details.poster_path.map(|path| image_url("w500", path)),
            backdrop: details.backdrop_path.map(backdrop_url),
        },
        seasons: details
            .seasons
            .into_iter()
            .map(|season| SeasonSummary {
                id: format!("tmdb:season:{}", season.id),
                season_number: season.season_number,
                title: season.name,
                overview: season.overview,
                air_date: season.air_date,
                episode_count: season.episode_count,
                poster: season.poster_path.map(|path| image_url("w500", path)),
            })
            .collect(),
        issues: Vec::new(),
        initial_season: None,
    }
}

pub(super) fn initial_season_number(seasons: &[SeasonSummary]) -> Option<SeasonNumber> {
    seasons
        .iter()
        .find(|season| season.season_number.is_regular() && season.episode_count.unwrap_or(0) > 0)
        .or_else(|| {
            seasons
                .iter()
                .find(|season| season.episode_count.unwrap_or(0) > 0)
        })
        .or_else(|| seasons.first())
        .map(|season| season.season_number)
}

pub(super) fn normalize_movie_details(
    details: MovieDetails,
    availability: Availability,
) -> MovieDetailsResponse {
    MovieDetailsResponse {
        kind: MediaKind::Movie,
        issues: Vec::new(),
        id: format!("tmdb:movie:{}", details.id),
        tmdb_id: details.id,
        title: details.title,
        overview: details.overview,
        year: year(details.release_date.as_deref()),
        rating: details.vote_average,
        runtime_minutes: details.runtime,
        images: Images {
            poster: details.poster_path.map(|path| image_url("w500", path)),
            backdrop: details.backdrop_path.map(backdrop_url),
        },
        availability,
    }
}

pub(super) fn normalize_season_details(
    series_tmdb_id: TmdbId,
    details: SeasonDetails,
    mut local_episodes: HashMap<i32, LocalEpisode>,
    issues: Vec<CatalogueIssue>,
) -> SeasonDetailsResponse {
    let availability_known = !issues
        .iter()
        .any(|issue| issue.source == super::CatalogueSource::Jellyfin);
    SeasonDetailsResponse {
        id: format!("tmdb:season:{}", details.id),
        series_tmdb_id,
        season_number: details.season_number,
        title: details.name,
        overview: details.overview,
        air_date: details.air_date,
        poster: details.poster_path.map(|path| image_url("w500", path)),
        episodes: details
            .episodes
            .into_iter()
            .map(|episode| {
                let local = local_episodes.remove(&episode.episode_number);
                Episode {
                    id: format!("tmdb:episode:{}", episode.id),
                    tmdb_id: episode.id,
                    season_number: episode.season_number,
                    episode_number: episode.episode_number,
                    title: episode.name,
                    overview: episode.overview.filter(|value| !value.trim().is_empty()),
                    air_date: episode.air_date,
                    rating: episode.vote_average,
                    still: episode.still_path.map(|path| image_url("original", path)),
                    runtime_minutes: local.as_ref().and_then(|episode| episode.runtime_minutes),
                    availability: Availability::for_copy(local.is_some(), availability_known),
                }
            })
            .collect(),
        issues,
    }
}

fn normalize(
    item: DiscoverResult,
    local_movies: &HashMap<TmdbId, LocalCopy>,
    availability_known: bool,
) -> Option<CatalogueItem> {
    let kind = match item.media_type {
        MediaType::Movie => MediaKind::Movie,
        MediaType::Tv => MediaKind::Series,
        MediaType::Person | MediaType::Collection | MediaType::Unknown => return None,
    };
    let title = item.display_title()?.to_owned();
    let year = year(
        item.release_date
            .as_deref()
            .or(item.first_air_date.as_deref()),
    );
    let card = MediaCard {
        runtime_minutes: None,
        number_of_seasons: None,
        id: format!("tmdb:{}:{}", kind.as_str(), item.id),
        tmdb_id: item.id,
        title,
        year,
        rating: item.vote_average,
        images: Images {
            poster: item.poster_path.map(|path| image_url("w500", path)),
            backdrop: item.backdrop_path.map(backdrop_url),
        },
        availability: if kind == MediaKind::Series {
            Availability::EpisodeBased
        } else {
            Availability::for_copy(local_movies.contains_key(&item.id), availability_known)
        },
    };

    Some(match kind {
        MediaKind::Movie => CatalogueItem::Movie(card),
        MediaKind::Series => CatalogueItem::Series(card),
    })
}

fn image_url(size: &str, path: String) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path;
    }
    format!(
        "{TMDB_IMAGE_BASE_URL}/{size}/{}",
        path.trim_start_matches('/')
    )
}

fn year(date: Option<&str>) -> Option<i32> {
    date.and_then(|date| date.get(..4))
        .and_then(|year| year.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seerr::EpisodeDetails;

    fn tmdb_id(value: i64) -> TmdbId {
        TmdbId::try_from(value).expect("test TMDb IDs are positive")
    }

    fn season_number(value: i32) -> SeasonNumber {
        SeasonNumber::try_from(value).expect("test season numbers are non-negative")
    }

    #[test]
    fn selects_the_first_populated_regular_season_before_specials() {
        let seasons = vec![
            SeasonSummary {
                id: "specials".into(),
                season_number: season_number(0),
                title: "Specials".into(),
                overview: None,
                air_date: None,
                episode_count: Some(3),
                poster: None,
            },
            SeasonSummary {
                id: "empty".into(),
                season_number: season_number(1),
                title: "Season 1".into(),
                overview: None,
                air_date: None,
                episode_count: Some(0),
                poster: None,
            },
            SeasonSummary {
                id: "season-two".into(),
                season_number: season_number(2),
                title: "Season 2".into(),
                overview: None,
                air_date: None,
                episode_count: Some(8),
                poster: None,
            },
        ];

        assert_eq!(initial_season_number(&seasons), Some(season_number(2)));
    }

    #[test]
    fn enriches_the_exact_episode_coordinate_with_its_local_copy() {
        let details = SeasonDetails {
            id: tmdb_id(20),
            name: "Season 2".into(),
            season_number: season_number(2),
            air_date: Some("2026-01-01".into()),
            poster_path: None,
            overview: None,
            episodes: vec![EpisodeDetails {
                id: tmdb_id(201),
                name: "A New Chapter".into(),
                episode_number: 1,
                season_number: season_number(2),
                air_date: Some("2026-01-01".into()),
                overview: Some(String::new()),
                still_path: Some("/still.jpg".into()),
                vote_average: Some(8.0),
            }],
        };
        let local_episodes = HashMap::from([(
            1,
            LocalEpisode {
                runtime_minutes: Some(52),
            },
        )]);

        let season = normalize_season_details(tmdb_id(100), details, local_episodes, Vec::new());

        assert_eq!(season.episodes[0].id, "tmdb:episode:201");
        assert_eq!(season.episodes[0].overview, None);
        assert_eq!(season.episodes[0].runtime_minutes, Some(52));
        assert_eq!(season.episodes[0].availability, Availability::Local);
    }
}
