use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, debug, error};

use super::backoff::{BackoffConfig, ExponentialBackoff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Failed,
}

pub struct ReconnectionManager {
    state: Arc<RwLock<ConnectionState>>,
    backoff: Arc<RwLock<ExponentialBackoff>>,
    stop_signal: Arc<RwLock<bool>>,
}

impl ReconnectionManager {
    pub fn new(config: BackoffConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            backoff: Arc::new(RwLock::new(ExponentialBackoff::new(config))),
            stop_signal: Arc::new(RwLock::new(false)),
        }
    }
    
    pub fn default_manager() -> Self {
        Self::new(BackoffConfig {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            multiplier: 2.0,
            max_retries: 100,
            jitter: true,
        })
    }
    
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }
    
    pub async fn set_state(&self, state: ConnectionState) {
        let mut current = self.state.write().await;
        if *current != state {
            info!("Connection state changed: {:?} -> {:?}", *current, state);
            *current = state;
        }
    }
    
    pub async fn is_connected(&self) -> bool {
        *self.state.read().await == ConnectionState::Connected
    }
    
    pub async fn wait_for_reconnect_delay(&self) -> bool {
        let mut backoff = self.backoff.write().await;
        
        if let Some(delay) = backoff.next_delay() {
            debug!("Waiting {:?} before reconnection attempt {}", delay, backoff.retry_count());
            
            let stop_signal = self.stop_signal.clone();
            tokio::select! {
                _ = tokio::time::sleep(delay) => {
                    true
                }
                _ = async {
                    while !*stop_signal.read().await {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                } => {
                    false
                }
            }
        } else {
            warn!("Max reconnection attempts exhausted");
            self.set_state(ConnectionState::Failed).await;
            false
        }
    }
    
    pub async fn on_connected(&self) {
        self.set_state(ConnectionState::Connected).await;
        self.backoff.write().await.reset();
    }
    
    pub async fn on_disconnected(&self) {
        let mut state = self.state.write().await;
        if *state == ConnectionState::Connected {
            *state = ConnectionState::Disconnected;
        }
    }
    
    pub async fn on_connecting(&self) {
        self.set_state(ConnectionState::Connecting).await;
    }
    
    pub async fn on_reconnecting(&self) {
        self.set_state(ConnectionState::Reconnecting).await;
    }
    
    pub async fn on_failed(&self) {
        self.set_state(ConnectionState::Failed).await;
    }
    
    pub async fn stop(&self) {
        *self.stop_signal.write().await = true;
    }
    
    pub async fn reset(&self) {
        self.backoff.write().await.reset();
        *self.stop_signal.write().await = false;
        *self.state.write().await = ConnectionState::Disconnected;
    }
    
    pub async fn retry_count(&self) -> u32 {
        self.backoff.read().await.retry_count()
    }
}

#[derive(Debug, Clone)]
pub enum ReconnectEvent {
    Disconnected,
    Connected,
    Reconnecting { attempt: u32 },
    Failed,
    Stop,
}

pub struct ReconnectEventEmitter {
    tx: mpsc::Sender<ReconnectEvent>,
}

impl ReconnectEventEmitter {
    pub fn new(tx: mpsc::Sender<ReconnectEvent>) -> Self {
        Self { tx }
    }
    
    pub async fn emit(&self, event: ReconnectEvent) {
        if let Err(e) = self.tx.send(event).await {
            warn!("Failed to emit reconnect event: {:?}", e);
        }
    }
}

pub struct ReconnectEventReceiver {
    rx: mpsc::Receiver<ReconnectEvent>,
}

impl ReconnectEventReceiver {
    pub fn new(rx: mpsc::Receiver<ReconnectEvent>) -> Self {
        Self { rx }
    }
    
    pub async fn recv(&mut self) -> Option<ReconnectEvent> {
        self.rx.recv().await
    }
}

pub fn create_reconnect_channel() -> (ReconnectEventEmitter, ReconnectEventReceiver) {
    let (tx, rx) = mpsc::channel(16);
    (ReconnectEventEmitter::new(tx), ReconnectEventReceiver::new(rx))
}

pub struct ConnectionGuard {
    manager: ReconnectionManager,
}

impl ConnectionGuard {
    pub fn new(manager: ReconnectionManager) -> Self {
        Self { manager }
    }
    
    pub async fn protect<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        if !self.manager.is_connected().await {
            warn!("Connection not established, operation may fail");
        }
        
        f().await
    }
}

impl Clone for ReconnectionManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            backoff: self.backoff.clone(),
            stop_signal: self.stop_signal.clone(),
        }
    }
}
