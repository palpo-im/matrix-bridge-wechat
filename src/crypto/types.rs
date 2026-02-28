use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OlmSession {
    pub session_id: String,
    pub sender_key: String,
    pub created_at: u64,
    pub last_used: u64,
    pub pickle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MegolmSession {
    pub session_id: String,
    pub sender_key: String,
    pub room_id: String,
    pub created_at: u64,
    pub last_used: u64,
    pub pickle: String,
    pub message_index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSigningKey {
    pub user_id: String,
    pub key_id: String,
    pub key: String,
    pub usage: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKey {
    pub user_id: String,
    pub device_id: String,
    pub key_id: String,
    pub key: String,
    pub algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    pub user_id: String,
    pub device_id: String,
    pub algorithms: Vec<String>,
    pub keys: HashMap<String, String>,
    pub signatures: HashMap<String, HashMap<String, String>>,
    pub unsigned: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomKey {
    pub room_id: String,
    pub session_id: String,
    pub algorithm: String,
    pub key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipRequest {
    pub request_id: String,
    pub room_id: String,
    pub session_id: String,
    pub sender_key: String,
    pub from_device: String,
    pub recipients: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct CryptoSessionInfo {
    pub device_id: Option<String>,
    pub identity_keys: HashMap<String, String>,
    pub has_cross_signing: bool,
    pub verified_devices: Vec<String>,
}

impl DeviceKeys {
    pub fn new(user_id: String, device_id: String) -> Self {
        Self {
            user_id,
            device_id,
            algorithms: vec![
                "m.olm.v1.curve25519-aes-sha2".to_string(),
                "m.megolm.v1.aes-sha2".to_string(),
            ],
            keys: HashMap::new(),
            signatures: HashMap::new(),
            unsigned: None,
        }
    }

    pub fn ed25519_key(&self) -> Option<&str> {
        self.keys
            .get(&format!("ed25519:{}", self.device_id))
            .map(|s| s.as_str())
    }

    pub fn curve25519_key(&self) -> Option<&str> {
        self.keys
            .get(&format!("curve25519:{}", self.device_id))
            .map(|s| s.as_str())
    }
}
