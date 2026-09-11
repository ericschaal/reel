//! Title metadata is shared by compact summaries and detail views. Availability
//! and episode guides are deliberately loaded separately from this cache.
use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, Semaphore};

use super::{
    Catalogue, Error, MediaKind, TitleSummariesResponse, TitleSummary, TitleSummaryIssue,
    TitleSummaryIssueCode, map_details_error,
};
use crate::{
    media::{CatalogueId, TmdbId},
    seerr::{MovieDetails, SeriesDetails},
};

pub(super) const MAX_SUMMARIES: usize = 40;
const CACHE_CAPACITY: usize = 512;
const CACHE_TTL: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct TitleKey {
    pub kind: MediaKind,
    pub id: TmdbId,
    pub language: Option<String>,
}

impl TitleKey {
    pub fn parse(value: &str, language: Option<&str>) -> Result<Self, Error> {
        let (kind, id) = match value.parse().map_err(|_| Error::InvalidQuery)? {
            CatalogueId::Movie(id) => (MediaKind::Movie, id),
            CatalogueId::Series(id) => (MediaKind::Series, id),
            CatalogueId::Season(_) | CatalogueId::Episode(_) => return Err(Error::InvalidQuery),
        };
        Ok(Self {
            kind,
            id,
            language: language.map(str::to_owned),
        })
    }

    fn canonical_id(&self) -> CatalogueId {
        match self.kind {
            MediaKind::Movie => CatalogueId::Movie(self.id),
            MediaKind::Series => CatalogueId::Series(self.id),
        }
    }
}

#[derive(Clone)]
pub(super) enum TitleMetadata {
    Movie(MovieDetails),
    Series(SeriesDetails),
}

impl TitleMetadata {
    pub fn summary(&self) -> TitleSummary {
        match self {
            Self::Movie(movie) => TitleSummary::Movie {
                id: CatalogueId::Movie(movie.id),
                runtime_minutes: movie.runtime,
            },
            Self::Series(series) => TitleSummary::Series {
                id: CatalogueId::Series(series.id),
                number_of_seasons: series.number_of_seasons,
            },
        }
    }
}

struct CachedTitle {
    loaded_at: Instant,
    metadata: Arc<TitleMetadata>,
}

pub(super) struct TitleCache {
    values: Mutex<HashMap<TitleKey, CachedTitle>>,
    // Fixed-size locks coalesce concurrent misses without an unbounded lock map.
    refresh: [Mutex<()>; 64],
    requests: Semaphore,
}

impl TitleCache {
    pub fn new() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            refresh: std::array::from_fn(|_| Mutex::new(())),
            requests: Semaphore::new(8),
        }
    }

    async fn get(&self, key: &TitleKey) -> Option<Arc<TitleMetadata>> {
        self.values
            .lock()
            .await
            .get(key)
            .filter(|cached| cached.loaded_at.elapsed() < CACHE_TTL)
            .map(|cached| Arc::clone(&cached.metadata))
    }
}

impl Catalogue {
    pub(super) async fn title_metadata(&self, key: TitleKey) -> Result<Arc<TitleMetadata>, Error> {
        let cache = &self.titles;
        if let Some(metadata) = cache.get(&key).await {
            return Ok(metadata);
        }
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let slot = (hasher.finish() % cache.refresh.len() as u64) as usize;
        let _refresh = cache.refresh[slot].lock().await;
        if let Some(metadata) = cache.get(&key).await {
            return Ok(metadata);
        }
        let _request = cache
            .requests
            .acquire()
            .await
            .expect("private semaphore is never closed");
        let metadata = Arc::new(match key.kind {
            MediaKind::Movie => TitleMetadata::Movie(
                self.seerr
                    .movie(key.id, key.language.as_deref())
                    .await
                    .map_err(map_details_error)?,
            ),
            MediaKind::Series => TitleMetadata::Series(
                self.seerr
                    .series_details(key.id, key.language.as_deref())
                    .await
                    .map_err(map_details_error)?,
            ),
        });
        let mut values = cache.values.lock().await;
        values.retain(|_, cached| cached.loaded_at.elapsed() < CACHE_TTL);
        if values.len() >= CACHE_CAPACITY {
            let oldest = values
                .iter()
                .min_by_key(|(_, cached)| cached.loaded_at)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                values.remove(&oldest);
            }
        }
        values.insert(
            key,
            CachedTitle {
                loaded_at: Instant::now(),
                metadata: Arc::clone(&metadata),
            },
        );
        Ok(metadata)
    }

    /// Complete the card view before it crosses the API boundary. A failed
    /// metadata lookup keeps the discovery card, with unknown facts.
    pub(super) async fn enrich_cards(
        &self,
        items: &mut [super::CatalogueItem],
        language: Option<&str>,
    ) -> bool {
        let mut tasks = tokio::task::JoinSet::new();
        for (index, item) in items.iter().enumerate() {
            let (kind, card) = match item {
                super::CatalogueItem::Movie(card) => (MediaKind::Movie, card),
                super::CatalogueItem::Series(card) => (MediaKind::Series, card),
                super::CatalogueItem::Category(_) => continue,
            };
            let key = TitleKey {
                kind,
                id: card.tmdb_id,
                language: language.map(str::to_owned),
            };
            let catalogue = self.clone();
            tasks.spawn(async move { (index, catalogue.title_metadata(key).await) });
        }
        let mut complete = true;
        while let Some(result) = tasks.join_next().await {
            match result {
                Ok((index, Ok(metadata))) => match (&mut items[index], metadata.as_ref()) {
                    (super::CatalogueItem::Movie(card), TitleMetadata::Movie(movie)) => {
                        card.runtime_minutes = movie.runtime
                    }
                    (super::CatalogueItem::Series(card), TitleMetadata::Series(series)) => {
                        card.number_of_seasons = series.number_of_seasons
                    }
                    _ => unreachable!("metadata kind matches card key"),
                },
                Ok((_, Err(_))) => complete = false,
                Err(error) => {
                    tracing::error!(%error, "card enrichment task failed");
                    complete = false;
                }
            }
        }
        complete
    }

    pub(super) async fn title_summaries(
        &self,
        ids: &str,
        language: Option<&str>,
    ) -> Result<TitleSummariesResponse, Error> {
        // Validate the whole batch before doing any I/O, and count duplicates
        // against the request limit so payload size is bounded as well as work.
        let ids: Vec<_> = ids.split(',').collect();
        if ids.len() > MAX_SUMMARIES {
            return Err(Error::InvalidQuery);
        }
        let mut keys = Vec::with_capacity(ids.len());
        for id in ids {
            let key = TitleKey::parse(id, language)?;
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        let mut tasks = tokio::task::JoinSet::new();
        for (index, key) in keys.into_iter().enumerate() {
            let catalogue = self.clone();
            tasks.spawn(async move {
                let id = key.canonical_id();
                (index, id, catalogue.title_metadata(key).await)
            });
        }
        let mut results = Vec::new();
        while let Some(result) = tasks.join_next().await {
            results.push(result.map_err(|error| {
                tracing::error!(%error, "title summary task failed");
                Error::Unavailable
            })?);
        }
        results.sort_by_key(|(index, _, _)| *index);
        let mut response = TitleSummariesResponse {
            items: Vec::new(),
            issues: Vec::new(),
        };
        for (_, id, result) in results {
            match result {
                Ok(metadata) => response.items.push(metadata.summary()),
                Err(error) => response.issues.push(TitleSummaryIssue {
                    id,
                    code: if matches!(error, Error::MediaNotFound) {
                        TitleSummaryIssueCode::MediaNotFound
                    } else {
                        TitleSummaryIssueCode::CatalogueUnavailable
                    },
                }),
            }
        }
        Ok(response)
    }
}
