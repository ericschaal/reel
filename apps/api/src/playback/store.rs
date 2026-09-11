//! Bounded ephemeral storage. Expiry is enforced on reads and reclaimed on writes.
use super::session::ProxyTarget;
use super::{Error, ids::ResourceId};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

pub(super) struct Store<K, V> {
    entries: RwLock<HashMap<K, Entry<V>>>,
    ttl: Duration,
    capacity: usize,
}
struct Entry<V> {
    value: Arc<V>,
    expires_at: Instant,
    valid: Option<Arc<AtomicBool>>,
}
impl<V> Entry<V> {
    fn is_live(&self, now: Instant) -> bool {
        self.expires_at > now
            && self
                .valid
                .as_ref()
                .is_none_or(|valid| valid.load(Ordering::Relaxed))
    }
}

/// Invalidates a provisional entry synchronously if activation fails or is cancelled.
/// The next store read rejects it; the next insertion reclaims its capacity.
#[must_use = "commit the entry only after activation succeeds"]
pub(super) struct PendingEntry {
    valid: Arc<AtomicBool>,
    committed: bool,
}
impl PendingEntry {
    pub(super) fn commit(mut self) {
        self.committed = true;
    }
}
impl Drop for PendingEntry {
    fn drop(&mut self) {
        if !self.committed {
            self.valid.store(false, Ordering::Relaxed);
        }
    }
}
impl<K: Eq + Hash, V> Store<K, V> {
    pub(super) fn new(ttl: Duration, capacity: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            ttl,
            capacity,
        }
    }
    pub(super) async fn insert(&self, id: K, value: V) -> Result<(), Error> {
        self.insert_at(id, value, Instant::now()).await
    }
    pub(super) async fn insert_pending(&self, id: K, value: V) -> Result<PendingEntry, Error> {
        let pending = PendingEntry {
            valid: Arc::new(AtomicBool::new(true)),
            committed: false,
        };
        self.insert_entry(id, value, Instant::now(), Some(Arc::clone(&pending.valid)))
            .await?;
        Ok(pending)
    }
    async fn insert_at(&self, id: K, value: V, now: Instant) -> Result<(), Error> {
        self.insert_entry(id, value, now, None).await
    }
    async fn insert_entry(
        &self,
        id: K,
        value: V,
        now: Instant,
        valid: Option<Arc<AtomicBool>>,
    ) -> Result<(), Error> {
        let mut entries = self.entries.write().await;
        entries.retain(|_, entry| entry.is_live(now));
        if entries.len() >= self.capacity && !entries.contains_key(&id) {
            return Err(Error::CapacityExceeded);
        }
        entries.insert(
            id,
            Entry {
                value: Arc::new(value),
                expires_at: now + self.ttl,
                valid,
            },
        );
        Ok(())
    }
    pub(super) async fn get(&self, id: &K) -> Option<Arc<V>> {
        self.get_at(id, Instant::now()).await
    }
    async fn get_at(&self, id: &K, now: Instant) -> Option<Arc<V>> {
        {
            let entries = self.entries.read().await;
            let entry = entries.get(id)?;
            if entry.is_live(now) {
                return Some(Arc::clone(&entry.value));
            }
        }
        let mut entries = self.entries.write().await;
        if entries.get(id).is_some_and(|entry| !entry.is_live(now)) {
            entries.remove(id);
        }
        None
    }
}

#[derive(Default)]
pub(super) struct ResourceRegistry {
    by_id: HashMap<ResourceId, ProxyTarget>,
    by_url: HashMap<ProxyTarget, ResourceId>,
}
impl ResourceRegistry {
    pub(super) fn register(&mut self, url: ProxyTarget) -> Result<ResourceId, Error> {
        if let Some(id) = self.by_url.get(&url) {
            return Ok(id.clone());
        }
        // Keep existing resource URLs valid; refuse growth rather than evict active segments.
        if self.by_id.len() >= 65_536 {
            return Err(Error::CapacityExceeded);
        }
        let id = ResourceId::generate();
        self.by_id.insert(id.clone(), url.clone());
        self.by_url.insert(url, id.clone());
        Ok(id)
    }
    pub(super) fn get(&self, id: &ResourceId) -> Option<&ProxyTarget> {
        self.by_id.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn shares_snapshots_rejects_overflow_and_reclaims_expired_entries() {
        let store = Store::new(Duration::from_secs(10), 1);
        let now = Instant::now();
        store.insert_at("first", vec![1, 2], now).await.unwrap();
        let first = store.get_at(&"first", now).await.unwrap();
        assert!(Arc::ptr_eq(
            &first,
            &store.get_at(&"first", now).await.unwrap()
        ));
        assert!(matches!(
            store.insert_at("second", vec![3], now).await,
            Err(Error::CapacityExceeded)
        ));
        let expired = now + Duration::from_secs(10);
        assert!(store.get_at(&"first", expired).await.is_none());
        store.insert_at("second", vec![3], expired).await.unwrap();
        assert_eq!(*store.get_at(&"second", expired).await.unwrap(), vec![3]);
        assert_eq!(
            *first,
            vec![1, 2],
            "an in-flight reader retains its snapshot"
        );
    }
    #[tokio::test]
    async fn cancelled_activation_releases_capacity_without_async_drop() {
        let store = Arc::new(Store::new(Duration::from_secs(60), 1));
        let (inserted, ready) = tokio::sync::oneshot::channel();
        let task_store = Arc::clone(&store);
        let activation = tokio::spawn(async move {
            let _pending = task_store.insert_pending("cancelled", 1).await.unwrap();
            inserted.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        assert!(
            store.get(&"cancelled").await.is_some(),
            "converter callbacks can read a pending session"
        );
        activation.abort();
        assert!(activation.await.unwrap_err().is_cancelled());
        // Insertion must reclaim the cancelled slot even without a prior read.
        let committed = store.insert_pending("ready", 2).await.unwrap();
        committed.commit();
        assert!(store.get(&"cancelled").await.is_none());
        assert_eq!(*store.get(&"ready").await.unwrap(), 2);
    }

    #[test]
    fn repeated_resources_reuse_ids_and_keep_different_urls_distinct() {
        let mut resources = ResourceRegistry::default();
        let url = ProxyTarget::Original("https://cdn.example/segment.ts?part=1".parse().unwrap());
        let first = resources.register(url.clone()).unwrap();
        assert_eq!(resources.register(url.clone()).unwrap(), first);
        let second = resources
            .register(ProxyTarget::Original(
                "https://cdn.example/segment.ts?part=2".parse().unwrap(),
            ))
            .unwrap();
        assert_ne!(first, second);
        assert_eq!(resources.get(&first), Some(&url));
        assert_eq!(resources.by_id.len(), 2);
    }
}
