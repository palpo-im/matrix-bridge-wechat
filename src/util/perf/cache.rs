use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct CacheEntry<V> {
    pub value: V,
    pub created_at: Instant,
    pub expires_at: Option<Instant>,
    pub hits: u64,
}

impl<V> CacheEntry<V> {
    pub fn new(value: V, ttl: Option<Duration>) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            expires_at: ttl.map(|t| now + t),
            hits: 0,
        }
    }
    
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| Instant::now() > e).unwrap_or(false)
    }
    
    pub fn hit(&mut self) {
        self.hits += 1;
    }
    
    pub fn age(&self) -> Duration {
        Instant::now() - self.created_at
    }
}

pub struct Cache<K, V> {
    inner: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    default_ttl: Option<Duration>,
    max_size: usize,
}

impl<K, V> Cache<K, V>
where
    K: Eq + Hash + Clone + std::fmt::Debug,
    V: Clone,
{
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: None,
            max_size,
        }
    }
    
    pub fn with_ttl(max_size: usize, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            default_ttl: Some(ttl),
            max_size,
        }
    }
    
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.write().await;
        
        if let Some(entry) = inner.get_mut(key) {
            if entry.is_expired() {
                inner.remove(key);
                return None;
            }
            entry.hit();
            return Some(entry.value.clone());
        }
        
        None
    }
    
    pub async fn get_entry(&self, key: &K) -> Option<CacheEntry<V>> {
        let mut inner = self.inner.write().await;
        
        if let Some(entry) = inner.get_mut(key) {
            if entry.is_expired() {
                inner.remove(key);
                return None;
            }
            entry.hit();
            return Some(entry.clone());
        }
        
        None
    }
    
    pub async fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl).await;
    }
    
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let mut inner = self.inner.write().await;
        
        if inner.len() >= self.max_size && !inner.contains_key(&key) {
            self.evict_oldest(&mut inner);
        }
        
        let entry = CacheEntry::new(value, ttl);
        inner.insert(key, entry);
    }
    
    fn evict_oldest(&self, inner: &mut HashMap<K, CacheEntry<V>>) {
        if let Some(oldest_key) = inner
            .iter()
            .min_by_key(|(_, e)| e.created_at)
            .map(|(k, _)| k.clone())
        {
            debug!("Evicting oldest cache entry: {:?}", oldest_key);
            inner.remove(&oldest_key);
        }
    }
    
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut inner = self.inner.write().await;
        inner.remove(key).map(|e| e.value)
    }
    
    pub async fn contains(&self, key: &K) -> bool {
        let inner = self.inner.read().await;
        inner.get(key).map(|e| !e.is_expired()).unwrap_or(false)
    }
    
    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
    
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
    
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
    
    pub async fn cleanup_expired(&self) {
        let mut inner = self.inner.write().await;
        let now = Instant::now();
        inner.retain(|_, entry| {
            entry.expires_at.map(|e| e > now).unwrap_or(true)
        });
    }
    
    pub async fn stats(&self) -> CacheStats {
        let inner = self.inner.read().await;
        let total_entries = inner.len();
        let total_hits: u64 = inner.values().map(|e| e.hits).sum();
        let expired_count = inner.values().filter(|e| e.is_expired()).count();
        
        CacheStats {
            total_entries,
            total_hits,
            expired_count,
        }
    }
}

impl<K, V> Clone for Cache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            default_ttl: self.default_ttl,
            max_size: self.max_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_hits: u64,
    pub expired_count: usize,
}

pub struct LruCache<K, V> {
    inner: Arc<RwLock<lru::LruCache<K, CacheEntry<V>>>>,
    default_ttl: Option<Duration>,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            default_ttl: None,
        }
    }
    
    pub fn with_ttl(capacity: usize, ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(lru::LruCache::new(
                std::num::NonZeroUsize::new(capacity).unwrap(),
            ))),
            default_ttl: Some(ttl),
        }
    }
    
    pub async fn get(&self, key: &K) -> Option<V>
    where
        K: std::borrow::Borrow<K> + ToOwned<Owned = K>,
    {
        let mut inner = self.inner.write().await;
        
        if let Some(entry) = inner.get_mut(key) {
            if entry.is_expired() {
                inner.pop(key);
                return None;
            }
            entry.hit();
            return Some(entry.value.clone());
        }
        
        None
    }
    
    pub async fn insert(&self, key: K, value: V) {
        let entry = CacheEntry::new(value, self.default_ttl);
        self.inner.write().await.put(key, entry);
    }
    
    pub async fn insert_with_ttl(&self, key: K, value: V, ttl: Option<Duration>) {
        let entry = CacheEntry::new(value, ttl);
        self.inner.write().await.put(key, entry);
    }
    
    pub async fn remove(&self, key: &K) -> Option<V>
    where
        K: std::borrow::Borrow<K> + ToOwned<Owned = K>,
    {
        self.inner.write().await.pop(key).map(|(_, e)| e.value)
    }
    
    pub async fn contains(&self, key: &K) -> bool
    where
        K: std::borrow::Borrow<K>,
    {
        let inner = self.inner.read().await;
        inner.contains(key)
    }
    
    pub async fn clear(&self) {
        self.inner.write().await.clear();
    }
    
    pub async fn len(&self) -> usize {
        self.inner.read().await.len()
    }
    
    pub async fn is_empty(&self) -> bool {
        self.inner.read().await.is_empty()
    }
}

impl<K, V> Clone for LruCache<K, V> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            default_ttl: self.default_ttl,
        }
    }
}

pub trait ToOwned {
    type Owned;
    fn to_owned(&self) -> Self::Owned;
}

impl<T: Clone> ToOwned for T {
    type Owned = T;
    fn to_owned(&self) -> T {
        self.clone()
    }
}

mod lru {
    use std::collections::HashMap;
    use std::hash::Hash;
    
    pub struct LruCache<K, V> {
        capacity: usize,
        map: HashMap<K, V>,
        order: Vec<K>,
    }
    
    impl<K: Eq + Hash + Clone, V> LruCache<K, V> {
        pub fn new(capacity: std::num::NonZeroUsize) -> Self {
            Self {
                capacity: capacity.get(),
                map: HashMap::new(),
                order: Vec::new(),
            }
        }
        
        pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
        where
            K: std::borrow::Borrow<Q>,
            Q: Hash + Eq + ?Sized,
        {
            if self.map.contains_key(key) {
                if let Some(k) = self.order.iter().find(|k| k.borrow() == key).cloned() {
                    self.order.retain(|x| x.borrow() != key);
                    self.order.push(k);
                }
            }
            self.map.get_mut(key)
        }
        
        pub fn put(&mut self, key: K, value: V) {
            if self.map.contains_key(&key) {
                self.order.retain(|k| k != &key);
            } else if self.map.len() >= self.capacity {
                if let Some(old_key) = self.order.first().cloned() {
                    self.map.remove(&old_key);
                    self.order.remove(0);
                }
            }
            
            self.order.push(key.clone());
            self.map.insert(key, value);
        }
        
        pub fn pop<Q>(&mut self, key: &Q) -> Option<(K, V)>
        where
            K: std::borrow::Borrow<Q>,
            Q: Hash + Eq + ?Sized,
        {
            if let Some(value) = self.map.remove(key) {
                self.order.retain(|k| k.borrow() != key);
                if let Some(k) = self.order.iter().find(|k| k.borrow() == key).cloned() {
                    return Some((k, value));
                }
            }
            None
        }
        
        pub fn contains<Q>(&self, key: &Q) -> bool
        where
            K: std::borrow::Borrow<Q>,
            Q: Hash + Eq + ?Sized,
        {
            self.map.contains_key(key)
        }
        
        pub fn clear(&mut self) {
            self.map.clear();
            self.order.clear();
        }
        
        pub fn len(&self) -> usize {
            self.map.len()
        }
        
        pub fn is_empty(&self) -> bool {
            self.map.is_empty()
        }
    }
}
