use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, SemaphorePermit, Mutex};
use tracing::{debug, warn};

pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    name: String,
    max_permits: usize,
}

impl ConcurrencyLimiter {
    pub fn new(name: impl Into<String>, max_permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_permits)),
            name: name.into(),
            max_permits,
        }
    }
    
    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        debug!("Acquiring permit for {}", self.name);
        self.semaphore.acquire().await.expect("semaphore closed")
    }
    
    pub async fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        self.semaphore.try_acquire().ok()
    }
    
    pub async fn acquire_timeout(&self, timeout: Duration) -> Option<SemaphorePermit<'_>> {
        match tokio::time::timeout(timeout, self.semaphore.acquire()).await {
            Ok(Ok(permit)) => Some(permit),
            Ok(Err(_)) => None,
            Err(_) => {
                warn!("Timeout acquiring permit for {}", self.name);
                None
            }
        }
    }
    
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
    
    pub fn max_permits(&self) -> usize {
        self.max_permits
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Clone for ConcurrencyLimiter {
    fn clone(&self) -> Self {
        Self {
            semaphore: self.semaphore.clone(),
            name: self.name.clone(),
            max_permits: self.max_permits,
        }
    }
}

pub struct MultiLimiter {
    limiters: Vec<ConcurrencyLimiter>,
}

impl MultiLimiter {
    pub fn new(limiters: Vec<ConcurrencyLimiter>) -> Self {
        Self { limiters }
    }
    
    pub async fn acquire_all(&self) -> Vec<SemaphorePermit<'_>> {
        let mut permits = Vec::new();
        for limiter in &self.limiters {
            permits.push(limiter.acquire().await);
        }
        permits
    }
}

pub struct RateLimiter {
    permits: Arc<Mutex<u32>>,
    max_permits: u32,
    refill_rate: Duration,
    name: String,
}

impl RateLimiter {
    pub fn new(name: impl Into<String>, max_permits: u32, refill_rate: Duration) -> Self {
        Self {
            permits: Arc::new(Mutex::new(max_permits)),
            max_permits,
            refill_rate,
            name: name.into(),
        }
    }
    
    pub async fn try_acquire(&self) -> bool {
        let mut permits = self.permits.lock().await;
        if *permits > 0 {
            *permits -= 1;
            true
        } else {
            false
        }
    }
    
    pub async fn acquire(&self) {
        loop {
            if self.try_acquire().await {
                return;
            }
            tokio::time::sleep(self.refill_rate).await;
            self.refill().await;
        }
    }
    
    pub async fn acquire_timeout(&self, timeout: Duration) -> bool {
        let start = std::time::Instant::now();
        loop {
            if self.try_acquire().await {
                return true;
            }
            
            if start.elapsed() >= timeout {
                return false;
            }
            
            tokio::time::sleep(self.refill_rate).await;
            self.refill().await;
        }
    }
    
    async fn refill(&self) {
        let mut permits = self.permits.lock().await;
        *permits = self.max_permits;
    }
    
    pub async fn available(&self) -> u32 {
        *self.permits.lock().await
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            permits: self.permits.clone(),
            max_permits: self.max_permits,
            refill_rate: self.refill_rate,
            name: self.name.clone(),
        }
    }
}

pub struct TokenBucket {
    tokens: Arc<Mutex<f64>>,
    max_tokens: f64,
    refill_rate: f64,
    name: String,
    last_refill: Arc<Mutex<std::time::Instant>>,
}

impl TokenBucket {
    pub fn new(name: impl Into<String>, max_tokens: f64, refill_per_second: f64) -> Self {
        Self {
            tokens: Arc::new(Mutex::new(max_tokens)),
            max_tokens,
            refill_rate: refill_per_second,
            name: name.into(),
            last_refill: Arc::new(Mutex::new(std::time::Instant::now())),
        }
    }
    
    pub async fn try_consume(&self, tokens: f64) -> bool {
        self.refill().await;
        
        let mut available = self.tokens.lock().await;
        if *available >= tokens {
            *available -= tokens;
            true
        } else {
            false
        }
    }
    
    pub async fn consume(&self, tokens: f64) {
        while !self.try_consume(tokens).await {
            let wait_time = (tokens - self.available().await) / self.refill_rate;
            tokio::time::sleep(std::time::Duration::from_secs_f64(wait_time.max(0.01))).await;
        }
    }
    
    async fn refill(&self) {
        let mut last = self.last_refill.lock().await;
        let now = std::time::Instant::now();
        let elapsed = (now - *last).as_secs_f64();
        
        if elapsed > 0.0 {
            let mut tokens = self.tokens.lock().await;
            *tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
            *last = now;
        }
    }
    
    pub async fn available(&self) -> f64 {
        self.refill().await;
        *self.tokens.lock().await
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Clone for TokenBucket {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
            max_tokens: self.max_tokens,
            refill_rate: self.refill_rate,
            name: self.name.clone(),
            last_refill: self.last_refill.clone(),
        }
    }
}

pub struct AdaptiveLimiter {
    inner: ConcurrencyLimiter,
    success_count: Arc<Mutex<u64>>,
    failure_count: Arc<Mutex<u64>>,
    adjustment_interval: Duration,
}

impl AdaptiveLimiter {
    pub fn new(name: impl Into<String>, initial_permits: usize, max_permits: usize) -> Self {
        Self {
            inner: ConcurrencyLimiter::new(name, initial_permits),
            success_count: Arc::new(Mutex::new(0)),
            failure_count: Arc::new(Mutex::new(0)),
            adjustment_interval: Duration::from_secs(10),
        }
    }
    
    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        self.inner.acquire().await
    }
    
    pub async fn report_success(&self) {
        let mut count = self.success_count.lock().await;
        *count += 1;
    }
    
    pub async fn report_failure(&self) {
        let mut count = self.failure_count.lock().await;
        *count += 1;
    }
    
    pub async fn adjust(&self) {
        let success = *self.success_count.lock().await;
        let failure = *self.failure_count.lock().await;
        let total = success + failure;
        
        if total > 100 {
            let success_rate = success as f64 / total as f64;
            
            if success_rate > 0.95 && self.inner.available_permits() < self.inner.max_permits() {
                debug!("High success rate, increasing concurrency limit");
            } else if success_rate < 0.8 {
                warn!("Low success rate, consider reducing concurrency limit");
            }
            
            *self.success_count.lock().await = 0;
            *self.failure_count.lock().await = 0;
        }
    }
    
    pub fn name(&self) -> &str {
        self.inner.name()
    }
}

impl Clone for AdaptiveLimiter {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            success_count: self.success_count.clone(),
            failure_count: self.failure_count.clone(),
            adjustment_interval: self.adjustment_interval,
        }
    }
}
