use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore, Notify};
use tracing::{debug, warn};

#[derive(Debug, Clone)]
pub struct QueueMessage<T> {
    pub id: String,
    pub data: T,
    pub priority: u8,
    pub created_at: Instant,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl<T> QueueMessage<T> {
    pub fn new(id: impl Into<String>, data: T) -> Self {
        Self {
            id: id.into(),
            data,
            priority: 0,
            created_at: Instant::now(),
            retry_count: 0,
            max_retries: 3,
        }
    }
    
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
    
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }
    
    pub fn increment_retry(&mut self) -> bool {
        if self.retry_count < self.max_retries {
            self.retry_count += 1;
            true
        } else {
            false
        }
    }
    
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }
}

pub struct MessageQueue<T> {
    queue: Arc<Mutex<VecDeque<QueueMessage<T>>>>,
    notify: Arc<Notify>,
    capacity: usize,
    concurrency_limit: Arc<Semaphore>,
}

impl<T> MessageQueue<T> {
    pub fn new(capacity: usize, max_concurrency: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            notify: Arc::new(Notify::new()),
            capacity,
            concurrency_limit: Arc::new(Semaphore::new(max_concurrency)),
        }
    }
    
    pub async fn push(&self, message: QueueMessage<T>) -> Result<(), QueueError> {
        let mut queue = self.queue.lock().await;
        
        if queue.len() >= self.capacity {
            return Err(QueueError::QueueFull);
        }
        
        queue.push_back(message);
        self.notify.notify_one();
        
        debug!("Message added to queue, current size: {}", queue.len());
        Ok(())
    }
    
    pub async fn push_front(&self, message: QueueMessage<T>) -> Result<(), QueueError> {
        let mut queue = self.queue.lock().await;
        
        if queue.len() >= self.capacity {
            return Err(QueueError::QueueFull);
        }
        
        queue.push_front(message);
        self.notify.notify_one();
        
        Ok(())
    }
    
    pub async fn pop(&self) -> Option<QueueMessage<T>> {
        let mut queue = self.queue.lock().await;
        queue.pop_front()
    }
    
    pub async fn wait_for_message(&self) -> QueueMessage<T> {
        loop {
            {
                let mut queue = self.queue.lock().await;
                if let Some(message) = queue.pop_front() {
                    return message;
                }
            }
            
            self.notify.notified().await;
        }
    }
    
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }
    
    pub async fn is_empty(&self) -> bool {
        self.queue.lock().await.is_empty()
    }
    
    pub async fn clear(&self) {
        self.queue.lock().await.clear();
    }
    
    pub fn concurrency_semaphore(&self) -> Arc<Semaphore> {
        self.concurrency_limit.clone()
    }
    
    pub async fn acquire_concurrency(&self) -> Option<tokio::sync::SemaphorePermit<'_>> {
        self.concurrency_limit.acquire().await.ok()
    }
}

impl<T> Clone for MessageQueue<T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            notify: self.notify.clone(),
            capacity: self.capacity,
            concurrency_limit: self.concurrency_limit.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Queue is full")]
    QueueFull,
    
    #[error("Queue is empty")]
    QueueEmpty,
    
    #[error("Message not found")]
    MessageNotFound,
    
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
}

pub struct PriorityQueue<T> {
    high: Arc<Mutex<VecDeque<QueueMessage<T>>>>,
    normal: Arc<Mutex<VecDeque<QueueMessage<T>>>>,
    low: Arc<Mutex<VecDeque<QueueMessage<T>>>>,
    notify: Arc<Notify>,
    capacity: usize,
}

impl<T> PriorityQueue<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            high: Arc::new(Mutex::new(VecDeque::new())),
            normal: Arc::new(Mutex::new(VecDeque::new())),
            low: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            capacity,
        }
    }
    
    pub async fn push(&self, message: QueueMessage<T>) -> Result<(), QueueError> {
        let total_len = {
            let high = self.high.lock().await;
            let normal = self.normal.lock().await;
            let low = self.low.lock().await;
            high.len() + normal.len() + low.len()
        };
        
        if total_len >= self.capacity {
            return Err(QueueError::QueueFull);
        }
        
        match message.priority {
            8..=255 => {
                self.high.lock().await.push_back(message);
            }
            4..=7 => {
                self.normal.lock().await.push_back(message);
            }
            _ => {
                self.low.lock().await.push_back(message);
            }
        }
        
        self.notify.notify_one();
        Ok(())
    }
    
    pub async fn pop(&self) -> Option<QueueMessage<T>> {
        if let Some(msg) = self.high.lock().await.pop_front() {
            return Some(msg);
        }
        if let Some(msg) = self.normal.lock().await.pop_front() {
            return Some(msg);
        }
        self.low.lock().await.pop_front()
    }
    
    pub async fn wait_for_message(&self) -> QueueMessage<T> {
        loop {
            if let Some(message) = self.pop().await {
                return message;
            }
            self.notify.notified().await;
        }
    }
    
    pub async fn len(&self) -> usize {
        let high = self.high.lock().await;
        let normal = self.normal.lock().await;
        let low = self.low.lock().await;
        high.len() + normal.len() + low.len()
    }
    
    pub async fn is_empty(&self) -> bool {
        let high = self.high.lock().await;
        let normal = self.normal.lock().await;
        let low = self.low.lock().await;
        high.is_empty() && normal.is_empty() && low.is_empty()
    }
}

impl<T> Clone for PriorityQueue<T> {
    fn clone(&self) -> Self {
        Self {
            high: self.high.clone(),
            normal: self.normal.clone(),
            low: self.low.clone(),
            notify: self.notify.clone(),
            capacity: self.capacity,
        }
    }
}

pub struct DelayedQueue<T> {
    queue: Arc<Mutex<VecDeque<(Instant, QueueMessage<T>)>>>,
    notify: Arc<Notify>,
}

impl<T> DelayedQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
        }
    }
    
    pub async fn push_after(&self, delay: Duration, message: QueueMessage<T>) {
        let execute_at = Instant::now() + delay;
        let mut queue = self.queue.lock().await;
        
        let pos = queue.iter().position(|(t, _)| *t > execute_at).unwrap_or(queue.len());
        queue.insert(pos, (execute_at, message));
        
        self.notify.notify_one();
    }
    
    pub async fn pop_ready(&self) -> Option<QueueMessage<T>> {
        let mut queue = self.queue.lock().await;
        let now = Instant::now();
        
        if let Some((execute_at, _)) = queue.front() {
            if *execute_at <= now {
                return Some(queue.pop_front().unwrap().1);
            }
        }
        
        None
    }
    
    pub async fn wait_for_ready(&self) -> QueueMessage<T> {
        loop {
            {
                let mut queue = self.queue.lock().await;
                let now = Instant::now();
                
                if let Some((execute_at, _)) = queue.front() {
                    if *execute_at <= now {
                        return queue.pop_front().unwrap().1;
                    }
                    
                    let wait_duration = *execute_at - now;
                    drop(queue);
                    tokio::time::sleep(wait_duration).await;
                    continue;
                }
            }
            
            self.notify.notified().await;
        }
    }
    
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }
}

impl<T> Default for DelayedQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for DelayedQueue<T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            notify: self.notify.clone(),
        }
    }
}
