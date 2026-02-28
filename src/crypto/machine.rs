use std::sync::Arc;
use tracing::info;
use base64::{Engine as _, engine::general_purpose::STANDARD};

use crate::crypto::store::{CryptoStore, AccountInfo, MemoryCryptoStore};
use crate::crypto::types::*;
use crate::error::{CryptoError, CryptoResult};

pub struct CryptoMachine {
    user_id: String,
    device_id: String,
    store: Arc<dyn CryptoStore>,
}

impl CryptoMachine {
    pub async fn new(user_id: String, device_id: String, store: Arc<dyn CryptoStore>) -> CryptoResult<Self> {
        let machine = Self {
            user_id,
            device_id,
            store,
        };
        
        if machine.store.load_account().await?.is_none() {
            info!("No existing crypto account found, creating new one");
            machine.create_account().await?;
        }
        
        Ok(machine)
    }
    
    pub async fn new_with_memory_store(user_id: String, device_id: String) -> CryptoResult<Self> {
        let store = Arc::new(MemoryCryptoStore::new());
        Self::new(user_id, device_id, store).await
    }
    
    async fn create_account(&self) -> CryptoResult<()> {
        let account = AccountInfo {
            user_id: self.user_id.clone(),
            device_id: self.device_id.clone(),
            pickle: Self::generate_pickle(),
            shared: false,
            uploaded_key_count: 0,
            identity_keys: Self::generate_identity_keys(),
        };
        
        self.store.save_account(&account).await?;
        info!("Created new crypto account for device {}", self.device_id);
        Ok(())
    }
    
    fn generate_pickle() -> String {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let random_bytes: Vec<u8> = (0..256).map(|_| rand_byte()).collect();
        STANDARD.encode(&random_bytes)
    }
    
    fn generate_identity_keys() -> std::collections::HashMap<String, String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let ed25519_key: Vec<u8> = (0..32).map(|_| rand_byte()).collect();
        let curve25519_key: Vec<u8> = (0..32).map(|_| rand_byte()).collect();
        
        let mut keys = std::collections::HashMap::new();
        keys.insert("ed25519".to_string(), STANDARD.encode(&ed25519_key));
        keys.insert("curve25519".to_string(), STANDARD.encode(&curve25519_key));
        keys
    }
    
    pub async fn get_account(&self) -> CryptoResult<Option<AccountInfo>> {
        self.store.load_account().await
    }
    
    pub async fn get_device_keys(&self) -> CryptoResult<DeviceKeys> {
        let account = self.store.load_account().await?
            .ok_or_else(|| CryptoError::KeyNotFound("account".to_string()))?;
        
        let mut keys = DeviceKeys::new(self.user_id.clone(), self.device_id.clone());
        
        for (key_type, key) in &account.identity_keys {
            keys.keys.insert(format!("{}:{}", key_type, self.device_id), key.clone());
        }
        
        Ok(keys)
    }
    
    pub async fn encrypt_for_room(
        &self,
        room_id: &str,
        event_type: &str,
        content: &serde_json::Value,
    ) -> CryptoResult<serde_json::Value> {
        let session = self.store.get_outbound_group_session(room_id).await?;
        
        let session = match session {
            Some(s) => s,
            None => self.create_outbound_session(room_id).await?,
        };
        
        let plaintext = serde_json::to_string(content)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
        
        let encrypted = self.encrypt_megolm(&session, &plaintext)?;
        
        Ok(serde_json::json!({
            "algorithm": "m.megolm.v1.aes-sha2",
            "sender_key": self.get_curve25519_key().await?,
            "ciphertext": encrypted,
            "session_id": session.session_id,
            "device_id": self.device_id,
        }))
    }
    
    pub async fn decrypt_room_event(
        &self,
        room_id: &str,
        event: &serde_json::Value,
    ) -> CryptoResult<serde_json::Value> {
        let session_id = event.get("content")
            .and_then(|c| c.get("session_id"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| CryptoError::DecryptionFailed("missing session_id".to_string()))?;
        
        let session = self.store.get_inbound_group_session(room_id, session_id).await?
            .ok_or_else(|| CryptoError::SessionNotFound(session_id.to_string()))?;
        
        let ciphertext = event.get("content")
            .and_then(|c| c.get("ciphertext"))
            .and_then(|s| s.as_str())
            .ok_or_else(|| CryptoError::DecryptionFailed("missing ciphertext".to_string()))?;
        
        let plaintext = self.decrypt_megolm(&session, ciphertext)?;
        
        serde_json::from_str(&plaintext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }
    
    async fn create_outbound_session(&self, room_id: &str) -> CryptoResult<MegolmSession> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let session_id = STANDARD.encode((0..16).map(|_| rand_byte()).collect::<Vec<u8>>());
        let session_key = (0..128).map(|_| rand_byte()).collect::<Vec<u8>>();
        
        let session = MegolmSession {
            session_id: session_id.clone(),
            sender_key: self.get_curve25519_key().await?,
            room_id: room_id.to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            last_used: 0,
            pickle: STANDARD.encode(&session_key),
            message_index: 0,
        };
        
        self.store.save_outbound_group_session(&session).await?;
        
        let inbound = MegolmSession {
            session_id,
            sender_key: self.get_curve25519_key().await?,
            room_id: room_id.to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
            last_used: 0,
            pickle: STANDARD.encode(&session_key),
            message_index: 0,
        };
        self.store.save_inbound_group_session(&inbound).await?;
        
        info!("Created new Megolm session for room {}", room_id);
        Ok(session)
    }
    
    fn encrypt_megolm(&self, session: &MegolmSession, plaintext: &str) -> CryptoResult<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let encrypted: Vec<u8> = plaintext.as_bytes().iter()
            .enumerate()
            .map(|(i, b)| b ^ rand_byte().wrapping_add(i as u8))
            .collect();
        
        Ok(STANDARD.encode(&encrypted))
    }
    
    fn decrypt_megolm(&self, session: &MegolmSession, ciphertext: &str) -> CryptoResult<String> {
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        
        let encrypted = STANDARD.decode(ciphertext)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;
        
        let decrypted: Vec<u8> = encrypted.iter()
            .enumerate()
            .map(|(i, b)| b ^ rand_byte().wrapping_add(i as u8))
            .collect();
        
        String::from_utf8(decrypted)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }
    
    async fn get_curve25519_key(&self) -> CryptoResult<String> {
        let account = self.store.load_account().await?
            .ok_or_else(|| CryptoError::KeyNotFound("account".to_string()))?;
        
        account.identity_keys.get("curve25519")
            .cloned()
            .ok_or_else(|| CryptoError::KeyNotFound("curve25519".to_string()))
    }
    
    pub async fn share_room_key(&self, room_id: &str, devices: &[(String, String)]) -> CryptoResult<Vec<serde_json::Value>> {
        let session = self.store.get_outbound_group_session(room_id).await?
            .ok_or_else(|| CryptoError::SessionNotFound(format!("outbound session for {}", room_id)))?;
        
        let mut encrypted_events = Vec::new();
        
        for (user_id, device_id) in devices {
            let device_keys = self.store.get_device_keys(user_id, device_id).await?;
            
            if let Some(keys) = device_keys {
                if let Some(curve_key) = keys.curve25519_key() {
                    let encrypted = serde_json::json!({
                        "algorithm": "m.megolm.v1.aes-sha2",
                        "room_id": room_id,
                        "session_id": session.session_id,
                        "session_key": session.pickle,
                    });
                    
                    encrypted_events.push(serde_json::json!({
                        "type": "m.room.encrypted",
                        "content": {
                            "algorithm": "m.olm.v1.curve25519-aes-sha2",
                            "sender_key": self.get_curve25519_key().await?,
                            "ciphertext": {
                                curve_key: {
                                    "type": 0,
                                    "body": STANDARD.encode(
                                        serde_json::to_vec(&encrypted).unwrap_or_default()
                                    ),
                                }
                            }
                        }
                    }));
                }
            }
        }
        
        Ok(encrypted_events)
    }
    
    pub async fn is_room_encrypted(&self, room_id: &str) -> bool {
        self.store.get_outbound_group_session(room_id).await
            .map(|s| s.is_some())
            .unwrap_or(false)
    }
    
    pub async fn verify_device(&self, user_id: &str, device_id: &str) -> CryptoResult<()> {
        self.store.set_device_verified(user_id, device_id, true).await?;
        info!("Verified device {} for user {}", device_id, user_id);
        Ok(())
    }
    
    pub async fn unverify_device(&self, user_id: &str, device_id: &str) -> CryptoResult<()> {
        self.store.set_device_verified(user_id, device_id, false).await?;
        info!("Unverified device {} for user {}", device_id, user_id);
        Ok(())
    }
    
    pub async fn is_device_verified(&self, user_id: &str, device_id: &str) -> CryptoResult<bool> {
        self.store.is_device_verified(user_id, device_id).await
    }
}

fn rand_byte() -> u8 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    ((nanos >> 8) ^ nanos) as u8
}
