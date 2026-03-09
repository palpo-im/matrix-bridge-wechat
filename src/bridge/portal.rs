use std::sync::Arc;
use std::collections::HashMap;

use tracing::{info, debug, warn, error};

use crate::database::{Portal as DbPortal, PortalKey, Database};
use crate::matrix::client::MatrixClient;
use crate::matrix::types::{CreateRoomRequest, RoomMemberContent, PowerLevelsContent};
use crate::wechat::ChatType;

pub struct BridgePortal {
    pub key: PortalKey,
    pub inner: DbPortal,
    db: Database,
}

impl BridgePortal {
    pub fn new(key: PortalKey, db: Database) -> Self {
        Self {
            key: key.clone(),
            inner: DbPortal {
                uid: key.uid,
                receiver: key.receiver,
                mxid: None,
                name: String::new(),
                name_set: false,
                topic: String::new(),
                topic_set: false,
                avatar: String::new(),
                avatar_url: None,
                avatar_set: false,
                encrypted: false,
                last_sync: 0,
                first_event_id: None,
                next_batch_id: None,
            },
            db,
        }
    }

    pub fn from_db(portal: DbPortal, db: Database) -> Self {
        let key = portal.key();
        Self {
            key,
            inner: portal,
            db,
        }
    }

    pub fn mxid(&self) -> Option<&str> {
        self.inner.mxid.as_deref()
    }

    pub fn name(&self) -> &str {
        &self.inner.name
    }

    pub fn encrypted(&self) -> bool {
        self.inner.encrypted
    }

    pub fn is_private(&self) -> bool {
        !self.key.uid.starts_with("@@")
    }

    pub fn is_group(&self) -> bool {
        self.key.uid.starts_with("@@")
    }

    pub async fn set_mxid(&mut self, mxid: &str) -> anyhow::Result<()> {
        self.inner.mxid = Some(mxid.to_string());
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn set_name(&mut self, name: &str) -> anyhow::Result<()> {
        self.inner.name = name.to_string();
        self.inner.name_set = true;
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn set_topic(&mut self, topic: &str) -> anyhow::Result<()> {
        self.inner.topic = topic.to_string();
        self.inner.topic_set = true;
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn set_encrypted(&mut self, encrypted: bool) -> anyhow::Result<()> {
        self.inner.encrypted = encrypted;
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn get_matrix_room(
        &mut self,
        client: &MatrixClient,
        user_mxid: &str,
        puppet_mxid: &str,
        name: Option<&str>,
        avatar_url: Option<&str>,
        is_direct: bool,
        encrypted: bool,
    ) -> anyhow::Result<String> {
        if let Some(mxid) = &self.inner.mxid {
            return Ok(mxid.clone());
        }

        self.create_matrix_room(client, user_mxid, puppet_mxid, name, avatar_url, is_direct, encrypted).await
    }

    pub async fn create_matrix_room(
        &mut self,
        client: &MatrixClient,
        user_mxid: &str,
        puppet_mxid: &str,
        name: Option<&str>,
        avatar_url: Option<&str>,
        is_direct: bool,
        encrypted: bool,
    ) -> anyhow::Result<String> {
        let room_name = name.unwrap_or(&self.inner.name);
        
        let mut initial_state = vec![];

        let bridge_info_content = serde_json::json!({
            "bridgebot": client.user_id().unwrap_or(""),
            "creator": client.user_id().unwrap_or(""),
            "protocol": {
                "id": "wechat",
                "displayname": "WeChat",
                "avatar_url": "",
                "external_url": "",
            },
            "network": {
                "id": "wechat",
                "displayname": "WeChat",
                "avatar_url": "",
                "external_url": "",
            },
            "channel": {
                "id": self.key.uid,
                "displayname": room_name,
                "avatar_url": avatar_url.unwrap_or(""),
            },
        });

        let state_key = format!("net.maunium.wechat://wechat/{}", self.key.uid);

        // Send both m.room.bridge and uk.half-shot.bridge (per Go reference)
        initial_state.push(serde_json::json!({
            "type": "m.room.bridge",
            "state_key": state_key,
            "content": bridge_info_content
        }));

        initial_state.push(serde_json::json!({
            "type": "uk.half-shot.bridge",
            "state_key": state_key,
            "content": bridge_info_content
        }));

        if encrypted {
            initial_state.push(serde_json::json!({
                "type": "m.room.encryption",
                "state_key": "",
                "content": {
                    "algorithm": "m.megolm.v1.aes-sha2"
                }
            }));
        }

        let preset = if is_direct {
            "private_chat"
        } else {
            "public_chat"
        };

        let mut power_levels = PowerLevelsContent::default();
        power_levels.users.insert(user_mxid.to_string(), 100);
        power_levels.users.insert(puppet_mxid.to_string(), 100);

        let request = CreateRoomRequest {
            visibility: Some("private".to_string()),
            room_alias_name: None,
            name: Some(room_name.to_string()),
            topic: None,
            invite: vec![user_mxid.to_string(), puppet_mxid.to_string()],
            invite_3pid: vec![],
            room_version: None,
            preset: Some(preset.to_string()),
            is_direct,
            initial_state: Some(initial_state),
            power_level_content_override: Some(power_levels),
        };

        let room_id = client.create_room(&request).await?;
        
        info!("Created Matrix room {} for WeChat chat {}", room_id, self.key.uid);
        
        self.inner.mxid = Some(room_id.clone());
        self.inner.encrypted = encrypted;
        if let Some(name) = name {
            self.inner.name = name.to_string();
        }
        self.db.update_portal(&self.inner).await?;

        Ok(room_id)
    }

    pub async fn update_matrix_room(
        &mut self,
        client: &MatrixClient,
        name: Option<&str>,
        topic: Option<&str>,
        avatar_url: Option<&str>,
    ) -> anyhow::Result<()> {
        let Some(room_id) = &self.inner.mxid else {
            return Ok(());
        };

        if let Some(name) = name {
            if !self.inner.name_set || name != self.inner.name {
                client.set_room_name(room_id, name).await?;
                self.inner.name = name.to_string();
                self.inner.name_set = true;
            }
        }

        if let Some(topic) = topic {
            if !self.inner.topic_set || topic != self.inner.topic {
                client.set_room_topic(room_id, topic).await?;
                self.inner.topic = topic.to_string();
                self.inner.topic_set = true;
            }
        }

        if let Some(url) = avatar_url {
            if !self.inner.avatar_set || Some(url) != self.inner.avatar_url.as_deref() {
                client.set_room_avatar(room_id, url).await?;
                self.inner.avatar_url = Some(url.to_string());
                self.inner.avatar_set = true;
            }
        }

        // Also update bridge info state events
        if let Some(bot_mxid) = client.user_id() {
            self.update_bridge_info(client, bot_mxid).await.ok();
        }

        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    pub async fn sync_participants(
        &mut self,
        client: &MatrixClient,
        puppet_mxids: &[(&str, &str, Option<&str>)],
        user_prefix: &str,
    ) -> anyhow::Result<()> {
        let Some(room_id) = &self.inner.mxid else {
            return Ok(());
        };

        let members = client.get_joined_members(room_id).await?;
        let mut joined_mxids: std::collections::HashSet<String> = members.joined.keys().cloned().collect();

        // Collect expected puppet mxids for later comparison
        let expected_puppet_mxids: std::collections::HashSet<String> = puppet_mxids
            .iter()
            .map(|(_, mxid, _)| mxid.to_string())
            .collect();

        for (uin, puppet_mxid, displayname) in puppet_mxids {
            if joined_mxids.contains(*puppet_mxid) {
                joined_mxids.remove(*puppet_mxid);
                continue;
            }

            let content = if let Some(name) = displayname {
                RoomMemberContent::join_with(*name, "")
            } else {
                RoomMemberContent::join()
            };

            match client.set_membership(room_id, puppet_mxid, &content).await {
                Ok(_) => {
                    debug!("Invited puppet {} to room {}", puppet_mxid, room_id);
                }
                Err(e) => {
                    warn!("Failed to invite puppet {} to room {}: {}", puppet_mxid, room_id, e);
                }
            }
        }

        // Kick puppet members who are no longer in the WeChat group
        let puppet_prefix = format!("@{}", user_prefix);
        for leftover_mxid in &joined_mxids {
            if leftover_mxid.starts_with(&puppet_prefix) && !expected_puppet_mxids.contains(leftover_mxid.as_str()) {
                debug!("Kicking removed puppet {} from room {}", leftover_mxid, room_id);
                let _ = client.kick_user(room_id, leftover_mxid, Some("User left the WeChat group")).await;
            }
        }

        self.inner.last_sync = chrono::Utc::now().timestamp();
        self.db.update_portal(&self.inner).await?;
        Ok(())
    }

    /// Clean up a portal: kick real users, leave puppet users, then clear metadata.
    /// Ported from Go Cleanup.
    pub async fn cleanup(&mut self, client: &MatrixClient, user_prefix: &str) -> anyhow::Result<()> {
        if let Some(room_id) = &self.inner.mxid {
            // Get all joined members and handle them
            match client.get_joined_members(room_id).await {
                Ok(members) => {
                    let puppet_prefix = format!("@{}", user_prefix);
                    for member_mxid in members.joined.keys() {
                        if member_mxid.starts_with(&puppet_prefix) {
                            // Puppet user - leave
                            let _ = client.leave_room(room_id).await;
                        } else {
                            // Real user - kick
                            let _ = client.kick_user(room_id, member_mxid, Some("Portal deleted")).await;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get room members for cleanup: {}", e);
                }
            }
            let _ = client.leave_room(room_id).await;
        }

        self.inner.mxid = None;
        self.inner.name_set = false;
        self.inner.topic_set = false;
        self.inner.avatar_set = false;
        self.db.update_portal(&self.inner).await?;

        Ok(())
    }

    pub async fn delete(&self) -> anyhow::Result<()> {
        self.db.delete_portal(&self.key).await?;
        Ok(())
    }

    /// Update bridge info state events (m.room.bridge and uk.half-shot.bridge).
    /// Ported from Go UpdateBridgeInfo.
    pub async fn update_bridge_info(&self, client: &MatrixClient, bot_mxid: &str) -> anyhow::Result<()> {
        let Some(room_id) = &self.inner.mxid else {
            return Ok(());
        };
        let bridge_info = serde_json::json!({
            "bridgebot": bot_mxid,
            "creator": bot_mxid,
            "protocol": { "id": "wechat", "displayname": "WeChat" },
            "network": { "id": "wechat", "displayname": "WeChat" },
            "channel": {
                "id": self.key.uid,
                "displayname": self.inner.name,
                "avatar_url": self.inner.avatar_url.as_deref().unwrap_or(""),
            },
        });
        let state_key = format!("net.maunium.wechat://wechat/{}", self.key.uid);
        client.send_state(room_id, "m.room.bridge", &state_key, &bridge_info).await?;
        client.send_state(room_id, "uk.half-shot.bridge", &state_key, &bridge_info).await?;
        Ok(())
    }

    /// Add this portal to a user's personal filtering space.
    /// Ported from Go addToPersonalSpace.
    pub async fn add_to_space(
        &self,
        client: &MatrixClient,
        space_room_id: &str,
        db: &Database,
        user_mxid: &str,
    ) -> anyhow::Result<()> {
        let Some(room_id) = &self.inner.mxid else {
            return Ok(());
        };
        // Add as child of the space
        let content = serde_json::json!({ "via": [] });
        client.send_state(space_room_id, "m.space.child", room_id, &content).await?;
        // Mark in database
        db.mark_in_space(user_mxid, &self.key).await?;
        Ok(())
    }

    pub fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            inner: self.inner.clone(),
            db: self.db.clone(),
        }
    }
}

impl Clone for BridgePortal {
    fn clone(&self) -> Self {
        self.clone()
    }
}
