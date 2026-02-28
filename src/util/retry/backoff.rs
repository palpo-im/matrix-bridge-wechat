use std::time::Duration;

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub max_retries: u32,
    pub jitter: bool,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_retries: 10,
            jitter: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExponentialBackoff {
    config: BackoffConfig,
    current_delay: Duration,
    retry_count: u32,
}

impl ExponentialBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            current_delay: config.initial_delay,
            config,
            retry_count: 0,
        }
    }

    pub fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= self.config.max_retries {
            return None;
        }

        let delay = self.current_delay;
        self.retry_count += 1;

        let mut next = (self.current_delay.as_secs_f64() * self.config.multiplier)
            .min(self.config.max_delay.as_secs_f64());

        if self.config.jitter {
            next = self.add_jitter(next);
        }

        self.current_delay = Duration::from_secs_f64(next);
        Some(delay)
    }

    fn add_jitter(&self, delay: f64) -> f64 {
        let jitter = (delay * 0.1) * (rand_factor() * 2.0 - 1.0);
        (delay + jitter).max(0.0)
    }

    pub fn reset(&mut self) {
        self.current_delay = self.config.initial_delay;
        self.retry_count = 0;
    }

    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    pub fn is_exhausted(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }
}

fn rand_factor() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ((nanos % 1000) as f64) / 1000.0
}

#[derive(Debug, Clone, Copy)]
pub enum BackoffStrategy {
    Exponential,
    Linear,
    Fixed,
    Immediate,
}

impl BackoffStrategy {
    pub fn create_backoff(&self, config: BackoffConfig) -> Box<dyn Backoff + Send + Sync> {
        match self {
            BackoffStrategy::Exponential => Box::new(ExponentialBackoff::new(config)),
            BackoffStrategy::Linear => Box::new(LinearBackoff::new(config)),
            BackoffStrategy::Fixed => Box::new(FixedBackoff::new(config)),
            BackoffStrategy::Immediate => Box::new(ImmediateBackoff::new(config)),
        }
    }
}

pub trait Backoff {
    fn next_delay(&mut self) -> Option<Duration>;
    fn reset(&mut self);
    fn retry_count(&self) -> u32;
    fn is_exhausted(&self) -> bool;
}

impl Backoff for ExponentialBackoff {
    fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= self.config.max_retries {
            return None;
        }

        let delay = self.current_delay;
        self.retry_count += 1;

        let mut next = (self.current_delay.as_secs_f64() * self.config.multiplier)
            .min(self.config.max_delay.as_secs_f64());

        if self.config.jitter {
            next = self.add_jitter(next);
        }

        self.current_delay = Duration::from_secs_f64(next);
        Some(delay)
    }

    fn reset(&mut self) {
        self.current_delay = self.config.initial_delay;
        self.retry_count = 0;
    }

    fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn is_exhausted(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }
}

#[derive(Debug, Clone)]
pub struct LinearBackoff {
    config: BackoffConfig,
    current_delay: Duration,
    retry_count: u32,
}

impl LinearBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            current_delay: config.initial_delay,
            config,
            retry_count: 0,
        }
    }
}

impl Backoff for LinearBackoff {
    fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= self.config.max_retries {
            return None;
        }

        let delay = self.current_delay;
        self.retry_count += 1;

        let next = (self.current_delay.as_secs_f64() + self.config.initial_delay.as_secs_f64())
            .min(self.config.max_delay.as_secs_f64());
        self.current_delay = Duration::from_secs_f64(next);

        Some(delay)
    }

    fn reset(&mut self) {
        self.current_delay = self.config.initial_delay;
        self.retry_count = 0;
    }

    fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn is_exhausted(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }
}

#[derive(Debug, Clone)]
pub struct FixedBackoff {
    config: BackoffConfig,
    retry_count: u32,
}

impl FixedBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            config,
            retry_count: 0,
        }
    }
}

impl Backoff for FixedBackoff {
    fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= self.config.max_retries {
            return None;
        }

        self.retry_count += 1;
        Some(self.config.initial_delay)
    }

    fn reset(&mut self) {
        self.retry_count = 0;
    }

    fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn is_exhausted(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }
}

#[derive(Debug, Clone)]
pub struct ImmediateBackoff {
    config: BackoffConfig,
    retry_count: u32,
}

impl ImmediateBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            config,
            retry_count: 0,
        }
    }
}

impl Backoff for ImmediateBackoff {
    fn next_delay(&mut self) -> Option<Duration> {
        if self.retry_count >= self.config.max_retries {
            return None;
        }

        self.retry_count += 1;
        Some(Duration::ZERO)
    }

    fn reset(&mut self) {
        self.retry_count = 0;
    }

    fn retry_count(&self) -> u32 {
        self.retry_count
    }

    fn is_exhausted(&self) -> bool {
        self.retry_count >= self.config.max_retries
    }
}
