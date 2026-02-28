use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use crate::crypto::types::*;
use crate::error::{CryptoResult};

#[async_trait]
pub trait CryptoStore: Send + Sync {
    async fn load_account(&self) -> CryptoResult<Option<AccountInfo>>;
    async fn save_account(&self, account: &AccountInfo) -> CryptoResult<()>;
    
    async fn get_session(&self, sender_key: &str, session_id: &str) -> CryptoResult<Option<OlmSession>>;
    async fn save_session(&self, session: &OlmSession) -> CryptoResult<()>;
    async fn get_sessions(&self, sender_key: &str) -> CryptoResult<Vec<OlmSession>>;
    async fn delete_session(&self, sender_key: &str, session_id: &str) -> CryptoResult<()>;
    
    async fn get_inbound_group_session(&self, room_id: &str, session_id: &str) -> CryptoResult<Option<MegolmSession>>;
    async fn save_inbound_group_session(&self, session: &MegolmSession) -> CryptoResult<()>;
    async fn get_inbound_group_sessions_for_room(&self, room_id: &str) -> CryptoResult<Vec<MegolmSession>>;
    async fn delete_inbound_group_session(&self, room_id: &str, session_id: &str) -> CryptoResult<()>;
    
    async fn get_outbound_group_session(&self, room_id: &str) -> CryptoResult<Option<MegolmSession>>;
    async fn save_outbound_group_session(&self, session: &MegolmSession) -> CryptoResult<()>;
    async fn delete_outbound_group_session(&self, room_id: &str) -> CryptoResult<()>;
    
    async fn get_device_keys(&self, user_id: &str, device_id: &str) -> CryptoResult<Option<DeviceKeys>>;
    async fn save_device_keys(&self, keys: &DeviceKeys) -> CryptoResult<()>;
    async fn get_device_keys_for_user(&self, user_id: &str) -> CryptoResult<Vec<DeviceKeys>>;
    async fn delete_device_keys(&self, user_id: &str, device_id: &str) -> CryptoResult<()>;
    
    async fn get_cross_signing_key(&self, user_id: &str, key_type: &str) -> CryptoResult<Option<CrossSigningKey>>;
    async fn save_cross_signing_key(&self, key: &CrossSigningKey) -> CryptoResult<()>;
    async fn delete_cross_signing_key(&self, user_id: &str) -> CryptoResult<()>;
    
    async fn is_device_verified(&self, user_id: &str, device_id: &str) -> CryptoResult<bool>;
    async fn set_device_verified(&self, user_id: &str, device_id: &str, verified: bool) -> CryptoResult<()>;
    
    async fn save_value(&self, key: &str, value: &str) -> CryptoResult<()>;
    async fn get_value(&self, key: &str) -> CryptoResult<Option<String>>;
    async fn delete_value(&self, key: &str) -> CryptoResult<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub user_id: String,
    pub device_id: String,
    pub pickle: String,
    pub shared: bool,
    pub uploaded_key_count: u64,
    pub identity_keys: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct MemoryCryptoStore {
    inner: Arc<RwLock<MemoryCryptoStoreInner>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryCryptoStoreInner {
    account: Option<AccountInfo>,
    sessions: HashMap<String, Vec<OlmSession>>,
    inbound_group_sessions: HashMap<String, HashMap<String, MegolmSession>>,
    outbound_group_sessions: HashMap<String, MegolmSession>,
    device_keys: HashMap<String, HashMap<String, DeviceKeys>>,
    cross_signing_keys: HashMap<String, HashMap<String, CrossSigningKey>>,
    verified_devices: HashMap<String, HashMap<String, bool>>,
    values: HashMap<String, String>,
}

impl MemoryCryptoStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CryptoStore for MemoryCryptoStore {
    async fn load_account(&self) -> CryptoResult<Option<AccountInfo>> {
        let inner = self.inner.read().await;
        Ok(inner.account.clone())
    }
    
    async fn save_account(&self, account: &AccountInfo) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.account = Some(account.clone());
        Ok(())
    }
    
    async fn get_session(&self, sender_key: &str, session_id: &str) -> CryptoResult<Option<OlmSession>> {
        let inner = self.inner.read().await;
        Ok(inner
            .sessions
            .get(sender_key)
            .and_then(|sessions| sessions.iter().find(|s| s.session_id == session_id))
            .cloned())
    }
    
    async fn save_session(&self, session: &OlmSession) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        let sessions = inner.sessions.entry(session.sender_key.clone()).or_default();
        if let Some(pos) = sessions.iter().position(|s| s.session_id == session.session_id) {
            sessions[pos] = session.clone();
        } else {
            sessions.push(session.clone());
        }
        Ok(())
    }
    
    async fn get_sessions(&self, sender_key: &str) -> CryptoResult<Vec<OlmSession>> {
        let inner = self.inner.read().await;
        Ok(inner.sessions.get(sender_key).cloned().unwrap_or_default())
    }
    
    async fn delete_session(&self, sender_key: &str, session_id: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        if let Some(sessions) = inner.sessions.get_mut(sender_key) {
            sessions.retain(|s| s.session_id != session_id);
        }
        Ok(())
    }
    
    async fn get_inbound_group_session(&self, room_id: &str, session_id: &str) -> CryptoResult<Option<MegolmSession>> {
        let inner = self.inner.read().await;
        Ok(inner
            .inbound_group_sessions
            .get(room_id)
            .and_then(|sessions| sessions.get(session_id))
            .cloned())
    }
    
    async fn save_inbound_group_session(&self, session: &MegolmSession) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        let room_sessions = inner.inbound_group_sessions.entry(session.room_id.clone()).or_default();
        room_sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }
    
    async fn get_inbound_group_sessions_for_room(&self, room_id: &str) -> CryptoResult<Vec<MegolmSession>> {
        let inner = self.inner.read().await;
        Ok(inner
            .inbound_group_sessions
            .get(room_id)
            .map(|sessions| sessions.values().cloned().collect())
            .unwrap_or_default())
    }
    
    async fn delete_inbound_group_session(&self, room_id: &str, session_id: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        if let Some(sessions) = inner.inbound_group_sessions.get_mut(room_id) {
            sessions.remove(session_id);
        }
        Ok(())
    }
    
    async fn get_outbound_group_session(&self, room_id: &str) -> CryptoResult<Option<MegolmSession>> {
        let inner = self.inner.read().await;
        Ok(inner.outbound_group_sessions.get(room_id).cloned())
    }
    
    async fn save_outbound_group_session(&self, session: &MegolmSession) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.outbound_group_sessions.insert(session.room_id.clone(), session.clone());
        Ok(())
    }
    
    async fn delete_outbound_group_session(&self, room_id: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.outbound_group_sessions.remove(room_id);
        Ok(())
    }
    
    async fn get_device_keys(&self, user_id: &str, device_id: &str) -> CryptoResult<Option<DeviceKeys>> {
        let inner = self.inner.read().await;
        Ok(inner
            .device_keys
            .get(user_id)
            .and_then(|devices| devices.get(device_id))
            .cloned())
    }
    
    async fn save_device_keys(&self, keys: &DeviceKeys) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        let user_devices = inner.device_keys.entry(keys.user_id.clone()).or_default();
        user_devices.insert(keys.device_id.clone(), keys.clone());
        Ok(())
    }
    
    async fn get_device_keys_for_user(&self, user_id: &str) -> CryptoResult<Vec<DeviceKeys>> {
        let inner = self.inner.read().await;
        Ok(inner
            .device_keys
            .get(user_id)
            .map(|devices| devices.values().cloned().collect())
            .unwrap_or_default())
    }
    
    async fn delete_device_keys(&self, user_id: &str, device_id: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        if let Some(devices) = inner.device_keys.get_mut(user_id) {
            devices.remove(device_id);
        }
        Ok(())
    }
    
    async fn get_cross_signing_key(&self, user_id: &str, key_type: &str) -> CryptoResult<Option<CrossSigningKey>> {
        let inner = self.inner.read().await;
        Ok(inner
            .cross_signing_keys
            .get(user_id)
            .and_then(|keys| keys.get(key_type))
            .cloned())
    }
    
    async fn save_cross_signing_key(&self, key: &CrossSigningKey) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        let user_keys = inner.cross_signing_keys.entry(key.user_id.clone()).or_default();
        for usage in &key.usage {
            user_keys.insert(usage.clone(), key.clone());
        }
        Ok(())
    }
    
    async fn delete_cross_signing_key(&self, user_id: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.cross_signing_keys.remove(user_id);
        Ok(())
    }
    
    async fn is_device_verified(&self, user_id: &str, device_id: &str) -> CryptoResult<bool> {
        let inner = self.inner.read().await;
        Ok(inner
            .verified_devices
            .get(user_id)
            .and_then(|devices| devices.get(device_id))
            .copied()
            .unwrap_or(false))
    }
    
    async fn set_device_verified(&self, user_id: &str, device_id: &str, verified: bool) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        let user_devices = inner.verified_devices.entry(user_id.to_string()).or_default();
        user_devices.insert(device_id.to_string(), verified);
        Ok(())
    }
    
    async fn save_value(&self, key: &str, value: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.values.insert(key.to_string(), value.to_string());
        Ok(())
    }
    
    async fn get_value(&self, key: &str) -> CryptoResult<Option<String>> {
        let inner = self.inner.read().await;
        Ok(inner.values.get(key).cloned())
    }
    
    async fn delete_value(&self, key: &str) -> CryptoResult<()> {
        let mut inner = self.inner.write().await;
        inner.values.remove(key);
        Ok(())
    }
}
