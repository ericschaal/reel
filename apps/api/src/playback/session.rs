use super::{ids::CandidateId, store::ResourceRegistry, types::PlaybackTarget};
use crate::{aiostreams::DirectStream, jellyfin::JellyfinItemId};
use axum::http::HeaderMap;
use reqwest::Url;
use tokio::sync::RwLock;

pub(super) struct PlaybackSession {
    pub(super) source: SessionSource,
    pub(super) source_url: Url,
    pub(super) delivery: SessionDelivery,
    pub(super) resources: RwLock<ResourceRegistry>,
}

#[derive(Clone)]
pub(super) enum SessionDelivery {
    Original,
    Converted { playlist_url: Url },
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub(super) enum ProxyTarget {
    Original(Url),
    Converted(Url),
}

impl PlaybackSession {
    pub(super) fn output(&self) -> ProxyTarget {
        match &self.delivery {
            SessionDelivery::Original => ProxyTarget::Original(self.source_url.clone()),
            SessionDelivery::Converted { playlist_url } => {
                ProxyTarget::Converted(playlist_url.clone())
            }
        }
    }
}

#[derive(Clone)]
pub(super) enum SessionSource {
    Jellyfin {
        item_id: JellyfinItemId,
    },
    AioStreams {
        request_headers: HeaderMap,
        content_type: Option<&'static str>,
    },
}

pub(super) struct Discovery {
    pub(super) target: PlaybackTarget,
    pub(super) candidates: Vec<DiscoveredCandidate>,
    pub(super) upstream_error_count: usize,
}

pub(super) struct DiscoveredCandidate {
    pub(super) id: CandidateId,
    pub(super) stream: DirectStream,
}

impl ProxyTarget {
    pub(super) fn url(&self) -> &Url {
        match self {
            Self::Original(url) | Self::Converted(url) => url,
        }
    }
    pub(super) fn with_url(&self, url: Url) -> Self {
        match self {
            Self::Original(_) => Self::Original(url),
            Self::Converted(_) => Self::Converted(url),
        }
    }
    pub(super) fn upstream(&self, source: &SessionSource) -> super::PlaybackUpstream {
        match self {
            Self::Converted(_) => super::PlaybackUpstream::Stremio,
            Self::Original(_) => match source {
                SessionSource::Jellyfin { .. } => super::PlaybackUpstream::Jellyfin,
                SessionSource::AioStreams { .. } => super::PlaybackUpstream::AioStreams,
            },
        }
    }
}
