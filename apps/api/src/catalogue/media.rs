use std::collections::HashMap;

use crate::{
    jellyfin::{ItemType, ItemsQuery, Jellyfin},
    seerr::{DiscoverResult, MediaType},
};

use super::{CatalogueItem, Images, LocalCopy, MediaCard, MediaKind};

const TMDB_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p";

pub(super) async fn local_movies(
    jellyfin: &Jellyfin,
) -> crate::jellyfin::Result<HashMap<i64, LocalCopy>> {
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
                .parse()
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

pub(super) fn items(
    items: Vec<DiscoverResult>,
    local_movies: &HashMap<i64, LocalCopy>,
) -> Vec<CatalogueItem> {
    items
        .into_iter()
        .filter_map(|item| normalize(item, local_movies))
        .collect()
}

pub(super) fn backdrop_url(path: String) -> String {
    image_url("w780", path)
}

fn normalize(
    item: DiscoverResult,
    local_movies: &HashMap<i64, LocalCopy>,
) -> Option<CatalogueItem> {
    let kind = match item.media_type {
        MediaType::Movie => MediaKind::Movie,
        MediaType::Tv => MediaKind::Series,
        MediaType::Person | MediaType::Collection | MediaType::Unknown => return None,
    };
    let title = item.display_title()?.to_owned();
    let year = item
        .release_date
        .as_deref()
        .or(item.first_air_date.as_deref())
        .and_then(|date| date.get(..4))
        .and_then(|year| year.parse().ok());
    let card = MediaCard {
        id: format!("tmdb:{}:{}", kind.as_str(), item.id),
        tmdb_id: item.id,
        title,
        overview: item.overview,
        year,
        rating: item.vote_average,
        images: Images {
            poster: item.poster_path.map(|path| image_url("w500", path)),
            backdrop: item.backdrop_path.map(backdrop_url),
        },
        local_copy: (kind == MediaKind::Movie)
            .then(|| local_movies.get(&item.id).cloned())
            .flatten(),
    };

    Some(match kind {
        MediaKind::Movie => CatalogueItem::Movie(card),
        MediaKind::Series => CatalogueItem::Series(card),
    })
}

fn image_url(size: &str, path: String) -> String {
    format!(
        "{TMDB_IMAGE_BASE_URL}/{size}/{}",
        path.trim_start_matches('/')
    )
}
