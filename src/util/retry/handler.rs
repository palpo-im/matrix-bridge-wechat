use std::future::Future;
use std::time::Duration;
use tracing::{debug, warn};

use super::backoff::{Backoff, BackoffConfig, ExponentialBackoff};
use crate::error::BridgeError;

pub struct RetryHandler {
    backoff: ExponentialBackoff,
}

impl RetryHandler {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            backoff: ExponentialBackoff::new(config),
        }
    }
    
    pub fn default_retry() -> Self {
        Self::new(BackoffConfig::default())
    }
    
    pub async fn execute<F, Fut, T, E>(&mut self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::fmt::Debug + IsRetryable,
    {
        loop {
            match f().await {
                Ok(result) => {
                    self.backoff.reset();
                    return Ok(result);
                }
                Err(e) => {
                    if !e.is_retryable() || self.backoff.is_exhausted() {
                        return Err(e);
                    }
                    
                    let delay = self.backoff.next_delay().unwrap_or(Duration::ZERO);
                    debug!("Retry attempt {} after {:?}: {:?}", self.backoff.retry_count(), delay, e);
                    
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }
}

pub trait IsRetryable {
    fn is_retryable(&self) -> bool;
}

impl IsRetryable for BridgeError {
    fn is_retryable(&self) -> bool {
        match self {
            BridgeError::Network(_) => true,
            BridgeError::Timeout(_) => true,
            BridgeError::Http(msg) => msg.contains("5") || msg.contains("timeout"),
            BridgeError::RateLimited(_) => true,
            _ => false,
        }
    }
}

impl IsRetryable for String {
    fn is_retryable(&self) -> bool {
        true
    }
}

impl IsRetryable for anyhow::Error {
    fn is_retryable(&self) -> bool {
        if let Some(e) = self.downcast_ref::<BridgeError>() {
            e.is_retryable()
        } else {
            false
        }
    }
}

pub async fn with_retry<F, Fut, T, E>(f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug + IsRetryable,
{
    RetryHandler::default_retry().execute(f).await
}

pub async fn with_retry_config<F, Fut, T, E>(config: BackoffConfig, f: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: std::fmt::Debug + IsRetryable,
{
    RetryHandler::new(config).execute(f).await
}

#[derive(Debug, Clone, Copy)]
pub enum RetryResult<T> {
    Success(T),
    RetryableError,
    FatalError,
}

pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            ..Default::default()
        }
    }
    
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }
    
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
    
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }
    
    pub fn into_config(self) -> BackoffConfig {
        BackoffConfig {
            initial_delay: self.initial_delay,
            max_delay: self.max_delay,
            multiplier: self.multiplier,
            max_retries: self.max_retries,
            jitter: true,
        }
    }
}
