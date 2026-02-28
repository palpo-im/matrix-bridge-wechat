use std::sync::Arc;
use std::collections::HashMap;
use std::time::Instant;
use tokio::sync::RwLock;
use once_cell::sync::Lazy;

pub static METRICS: Lazy<Metrics> = Lazy::new(Metrics::new);

#[derive(Debug, Clone, Default)]
pub struct Counter {
    value: Arc<RwLock<u64>>,
    labels: HashMap<String, String>,
}

impl Counter {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_labels(labels: HashMap<String, String>) -> Self {
        Self {
            value: Arc::new(RwLock::new(0)),
            labels,
        }
    }
    
    pub async fn inc(&self) {
        let mut value = self.value.write().await;
        *value += 1;
    }
    
    pub async fn inc_by(&self, delta: u64) {
        let mut value = self.value.write().await;
        *value += delta;
    }
    
    pub async fn dec(&self) {
        let mut value = self.value.write().await;
        *value = value.saturating_sub(1);
    }
    
    pub async fn get(&self) -> u64 {
        *self.value.read().await
    }
    
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

#[derive(Debug, Clone)]
pub struct Gauge {
    value: Arc<RwLock<f64>>,
    labels: HashMap<String, String>,
}

impl Default for Gauge {
    fn default() -> Self {
        Self {
            value: Arc::new(RwLock::new(0.0)),
            labels: HashMap::new(),
        }
    }
}

impl Gauge {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn with_labels(labels: HashMap<String, String>) -> Self {
        Self {
            value: Arc::new(RwLock::new(0.0)),
            labels,
        }
    }
    
    pub async fn set(&self, value: f64) {
        *self.value.write().await = value;
    }
    
    pub async fn inc(&self) {
        let mut value = self.value.write().await;
        *value += 1.0;
    }
    
    pub async fn dec(&self) {
        let mut value = self.value.write().await;
        *value -= 1.0;
    }
    
    pub async fn add(&self, delta: f64) {
        let mut value = self.value.write().await;
        *value += delta;
    }
    
    pub async fn sub(&self, delta: f64) {
        let mut value = self.value.write().await;
        *value -= delta;
    }
    
    pub async fn get(&self) -> f64 {
        *self.value.read().await
    }
    
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

#[derive(Debug, Clone)]
pub struct Histogram {
    buckets: Vec<f64>,
    counts: Arc<RwLock<Vec<u64>>>,
    sum: Arc<RwLock<f64>>,
    count: Arc<RwLock<u64>>,
    labels: HashMap<String, String>,
}

impl Histogram {
    pub fn new(buckets: Vec<f64>) -> Self {
        let bucket_count = buckets.len();
        Self {
            buckets,
            counts: Arc::new(RwLock::new(vec![0; bucket_count + 1])),
            sum: Arc::new(RwLock::new(0.0)),
            count: Arc::new(RwLock::new(0)),
            labels: HashMap::new(),
        }
    }
    
    pub fn with_labels(buckets: Vec<f64>, labels: HashMap<String, String>) -> Self {
        let bucket_count = buckets.len();
        Self {
            buckets,
            counts: Arc::new(RwLock::new(vec![0; bucket_count + 1])),
            sum: Arc::new(RwLock::new(0.0)),
            count: Arc::new(RwLock::new(0)),
            labels,
        }
    }
    
    pub fn default_buckets() -> Vec<f64> {
        vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    }
    
    pub async fn observe(&self, value: f64) {
        let mut counts = self.counts.write().await;
        let mut sum = self.sum.write().await;
        let mut count = self.count.write().await;
        
        *sum += value;
        *count += 1;
        
        for (i, &bucket) in self.buckets.iter().enumerate() {
            if value <= bucket {
                counts[i] += 1;
            }
        }
        counts[self.buckets.len()] += 1;
    }
    
    pub async fn get_counts(&self) -> Vec<u64> {
        self.counts.read().await.clone()
    }
    
    pub async fn get_sum(&self) -> f64 {
        *self.sum.read().await
    }
    
    pub async fn get_count(&self) -> u64 {
        *self.count.read().await
    }
    
    pub fn labels(&self) -> &HashMap<String, String> {
        &self.labels
    }
}

pub struct HistogramTimer {
    start: Instant,
    histogram: Histogram,
}

impl HistogramTimer {
    pub fn new(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
    
    pub async fn observe_duration(&self) {
        let duration = self.start.elapsed().as_secs_f64();
        self.histogram.observe(duration).await;
    }
}

impl Drop for HistogramTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed().as_secs_f64();
        let histogram = self.histogram.clone();
        tokio::spawn(async move {
            histogram.observe(duration).await;
        });
    }
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub messages_bridged: Counter,
    pub messages_sent: Counter,
    pub messages_received: Counter,
    pub messages_failed: Counter,
    pub messages_latency: Histogram,
    
    pub http_requests: Counter,
    pub http_errors: Counter,
    pub http_latency: Histogram,
    
    pub websocket_connections: Gauge,
    pub websocket_messages: Counter,
    
    pub database_queries: Counter,
    pub database_errors: Counter,
    pub database_latency: Histogram,
    
    pub active_users: Gauge,
    pub active_portals: Gauge,
    pub active_puppets: Gauge,
    
    pub encryption_operations: Counter,
    pub encryption_errors: Counter,
    pub encryption_latency: Histogram,
    
    pub reconnection_attempts: Counter,
    pub reconnection_success: Counter,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            messages_bridged: Counter::new(),
            messages_sent: Counter::new(),
            messages_received: Counter::new(),
            messages_failed: Counter::new(),
            messages_latency: Histogram::new(Histogram::default_buckets()),
            
            http_requests: Counter::new(),
            http_errors: Counter::new(),
            http_latency: Histogram::new(Histogram::default_buckets()),
            
            websocket_connections: Gauge::new(),
            websocket_messages: Counter::new(),
            
            database_queries: Counter::new(),
            database_errors: Counter::new(),
            database_latency: Histogram::new(Histogram::default_buckets()),
            
            active_users: Gauge::new(),
            active_portals: Gauge::new(),
            active_puppets: Gauge::new(),
            
            encryption_operations: Counter::new(),
            encryption_errors: Counter::new(),
            encryption_latency: Histogram::new(Histogram::default_buckets()),
            
            reconnection_attempts: Counter::new(),
            reconnection_success: Counter::new(),
        }
    }
    
    pub async fn to_prometheus(&self) -> String {
        let mut output = String::new();
        
        output.push_str("# HELP bridge_messages_bridged Total number of messages bridged\n");
        output.push_str("# TYPE bridge_messages_bridged counter\n");
        output.push_str(&format!("bridge_messages_bridged {}\n", self.messages_bridged.get().await));
        
        output.push_str("# HELP bridge_messages_sent Total number of messages sent\n");
        output.push_str("# TYPE bridge_messages_sent counter\n");
        output.push_str(&format!("bridge_messages_sent {}\n", self.messages_sent.get().await));
        
        output.push_str("# HELP bridge_messages_received Total number of messages received\n");
        output.push_str("# TYPE bridge_messages_received counter\n");
        output.push_str(&format!("bridge_messages_received {}\n", self.messages_received.get().await));
        
        output.push_str("# HELP bridge_messages_failed Total number of messages failed\n");
        output.push_str("# TYPE bridge_messages_failed counter\n");
        output.push_str(&format!("bridge_messages_failed {}\n", self.messages_failed.get().await));
        
        output.push_str("# HELP bridge_http_requests Total number of HTTP requests\n");
        output.push_str("# TYPE bridge_http_requests counter\n");
        output.push_str(&format!("bridge_http_requests {}\n", self.http_requests.get().await));
        
        output.push_str("# HELP bridge_http_errors Total number of HTTP errors\n");
        output.push_str("# TYPE bridge_http_errors counter\n");
        output.push_str(&format!("bridge_http_errors {}\n", self.http_errors.get().await));
        
        output.push_str("# HELP bridge_websocket_connections Current number of WebSocket connections\n");
        output.push_str("# TYPE bridge_websocket_connections gauge\n");
        output.push_str(&format!("bridge_websocket_connections {}\n", self.websocket_connections.get().await));
        
        output.push_str("# HELP bridge_database_queries Total number of database queries\n");
        output.push_str("# TYPE bridge_database_queries counter\n");
        output.push_str(&format!("bridge_database_queries {}\n", self.database_queries.get().await));
        
        output.push_str("# HELP bridge_database_errors Total number of database errors\n");
        output.push_str("# TYPE bridge_database_errors counter\n");
        output.push_str(&format!("bridge_database_errors {}\n", self.database_errors.get().await));
        
        output.push_str("# HELP bridge_active_users Current number of active users\n");
        output.push_str("# TYPE bridge_active_users gauge\n");
        output.push_str(&format!("bridge_active_users {}\n", self.active_users.get().await));
        
        output.push_str("# HELP bridge_active_portals Current number of active portals\n");
        output.push_str("# TYPE bridge_active_portals gauge\n");
        output.push_str(&format!("bridge_active_portals {}\n", self.active_portals.get().await));
        
        output.push_str("# HELP bridge_active_puppets Current number of active puppets\n");
        output.push_str("# TYPE bridge_active_puppets gauge\n");
        output.push_str(&format!("bridge_active_puppets {}\n", self.active_puppets.get().await));
        
        output.push_str("# HELP bridge_encryption_operations Total number of encryption operations\n");
        output.push_str("# TYPE bridge_encryption_operations counter\n");
        output.push_str(&format!("bridge_encryption_operations {}\n", self.encryption_operations.get().await));
        
        output.push_str("# HELP bridge_encryption_errors Total number of encryption errors\n");
        output.push_str("# TYPE bridge_encryption_errors counter\n");
        output.push_str(&format!("bridge_encryption_errors {}\n", self.encryption_errors.get().await));
        
        output.push_str("# HELP bridge_reconnection_attempts Total number of reconnection attempts\n");
        output.push_str("# TYPE bridge_reconnection_attempts counter\n");
        output.push_str(&format!("bridge_reconnection_attempts {}\n", self.reconnection_attempts.get().await));
        
        output.push_str("# HELP bridge_reconnection_success Total number of successful reconnections\n");
        output.push_str("# TYPE bridge_reconnection_success counter\n");
        output.push_str(&format!("bridge_reconnection_success {}\n", self.reconnection_success.get().await));
        
        output
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

pub fn metrics() -> &'static Metrics {
    &METRICS
}
