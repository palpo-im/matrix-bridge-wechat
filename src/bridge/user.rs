use std::sync::Arc;

use tracing::{info, warn, debug};

use crate::bridge::WechatBridge;
use crate::database::{User as DbUser, Database, PortalKey};
use crate::wechat::WechatClient;
use crate::matrix::MatrixClient;
use crate::config::Config;

pub struct BridgeUser {
    pub mxid: String,
    pub inner: DbUser,
    pub client: Option<WechatClient>,
    db: Database,
    config: Option<Arc<Config>>,
}

impl BridgeUser {
    pub fn new(mxid: String, db: Database) -> Self {
        Self {
            mxid: mxid.clone(),
            inner: DbUser::new(mxid),
            client: None,
            db,
            config: None,
        }
    }

    pub fn with_config(mut self, config: Arc<Config>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn from_db(user: DbUser, db: Database) -> Self {
        let mxid = user.mxid.clone();
        Self {
            mxid,
            inner: user,
            client: None,
            db,
            config: None,
        }
    }

    pub fn from_db_with_config(user: DbUser, db: Database, config: Arc<Config>) -> Self {
        let mxid = user.mxid.clone();
        Self {
            mxid,
            inner: user,
            client: None,
            db,
            config: Some(config),
        }
    }

    pub fn uid(&self) -> Option<crate::util::UID> {
        self.inner.uid()
    }

    pub fn uin(&self) -> Option<&str> {
        self.inner.uin.as_deref()
    }

    pub fn management_room(&self) -> Option<&str> {
        self.inner.management_room.as_deref()
    }

    pub fn space_room(&self) -> Option<&str> {
        self.inner.space_room.as_deref()
    }

    pub fn is_logged_in(&self) -> bool {
        self.inner.uin.is_some() && self.client.is_some()
    }

    pub fn get_client(&self) -> Option<&WechatClient> {
        self.client.as_ref()
    }

    pub async fn set_uin(&mut self, uin: &str) -> anyhow::Result<()> {
        self.inner.uin = Some(uin.to_string());
        self.db.update_user(&self.inner).await?;
        info!("User {} logged in with uin {}", self.mxid, uin);
        Ok(())
    }

    pub async fn set_management_room(&mut self, room_id: &str) -> anyhow::Result<()> {
        self.inner.management_room = Some(room_id.to_string());
        self.db.update_user(&self.inner).await?;
        info!("Set management room for {} to {}", self.mxid, room_id);
        Ok(())
    }

    pub async fn set_space_room(&mut self, room_id: &str) -> anyhow::Result<()> {
        self.inner.space_room = Some(room_id.to_string());
        self.db.update_user(&self.inner).await?;
        info!("Set space room for {} to {}", self.mxid, room_id);
        Ok(())
    }

    pub fn set_client(&mut self, client: WechatClient) {
        self.client = Some(client);
    }

    pub async fn login(&mut self, wechat_service: Arc<crate::wechat::WechatService>) -> anyhow::Result<()> {
        let client = WechatClient::new(self.mxid.clone(), wechat_service);
        
        client.connect().await?;
        
        let is_logged = client.is_logged_in().await?;
        if is_logged {
            let user_info = client.get_self().await?;
            self.inner.uin = Some(user_info.id.clone());
            self.client = Some(client);
            self.db.update_user(&self.inner).await?;
            info!("User {} logged in as {}", self.mxid, user_info.id);
        } else {
            info!("User {} connected but not logged in, waiting for QR scan", self.mxid);
            self.client = Some(client);
        }
        
        Ok(())
    }

    pub async fn logout(&mut self) -> anyhow::Result<()> {
        if let Some(client) = &self.client {
            let _ = client.disconnect().await;
        }
        self.client = None;
        self.inner.uin = None;
        self.db.update_user(&self.inner).await?;
        info!("User {} logged out", self.mxid);
        Ok(())
    }

    pub async fn get_or_create_management_room(
        &mut self,
        matrix_client: &MatrixClient,
        bot_mxid: &str,
    ) -> anyhow::Result<String> {
        if let Some(room_id) = &self.inner.management_room {
            return Ok(room_id.clone());
        }

        let request = crate::matrix::types::CreateRoomRequest::private("WeChat Bridge")
            .with_invite(self.mxid.clone());

        let room_id = matrix_client.create_room(&request).await?;
        
        matrix_client.set_room_name(&room_id, "WeChat Bridge").await?;
        
        let welcome_msg = if self.is_logged_in() {
            "Welcome to the WeChat bridge! Use `!wechat help` for available commands."
        } else {
            "Welcome to the WeChat bridge! Use `!wechat login` to connect your WeChat account."
        };
        
        let _ = matrix_client.send_notice(&room_id, welcome_msg).await;
        
        self.inner.management_room = Some(room_id.clone());
        self.db.update_user(&self.inner).await?;
        
        info!("Created management room {} for user {}", room_id, self.mxid);
        Ok(room_id)
    }

    pub async fn send_management_notice(
        &self,
        matrix_client: &MatrixClient,
        message: &str,
    ) -> anyhow::Result<()> {
        let Some(room_id) = &self.inner.management_room else {
            warn!("User {} has no management room", self.mxid);
            return Ok(());
        };

        matrix_client.send_notice(room_id, message).await?;
        Ok(())
    }

    pub async fn sync_direct_chats(
        &mut self,
        _matrix_client: &MatrixClient,
    ) -> anyhow::Result<()> {
        let Some(client) = &self.client else {
            return Ok(());
        };

        let friends = client.get_friend_list().await?;
        debug!("User {} has {} friends", self.mxid, friends.len());

        Ok(())
    }

    /// Resync contacts: iterate friends and sync each puppet's name/avatar.
    /// Ported from Go ResyncContacts.
    pub async fn sync_contacts(
        &self,
        bridge: &WechatBridge,
    ) -> anyhow::Result<()> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("not logged in"))?;
        let friends = client.get_friend_list().await?;
        info!("Syncing {} contacts for {}", friends.len(), self.mxid);
        let matrix_client = bridge.get_matrix_client();
        for friend in &friends {
            match bridge.get_puppet_by_uin(&friend.id).await {
                Ok(puppet) => {
                    let mut puppet = Arc::try_unwrap(puppet).unwrap_or_else(|p| (*p).clone());
                    if let Err(e) = puppet.sync(
                        &matrix_client,
                        Some(&friend.name),
                        friend.avatar.as_deref(),
                        false,
                    ).await {
                        warn!("Failed to sync puppet {}: {}", friend.id, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to get puppet for {}: {}", friend.id, e);
                }
            }
        }
        Ok(())
    }

    /// Resync groups: create or update portals for each WeChat group.
    /// Ported from Go ResyncGroups.
    pub async fn sync_groups(
        &self,
        bridge: &WechatBridge,
        create_portals: bool,
    ) -> anyhow::Result<()> {
        let client = self.client.as_ref().ok_or_else(|| anyhow::anyhow!("not logged in"))?;
        let groups = client.get_group_list().await?;
        info!("Syncing {} groups for {}", groups.len(), self.mxid);
        let matrix_client = bridge.get_matrix_client();
        for group in &groups {
            let key = PortalKey::new(group.id.clone(), group.id.clone());
            match bridge.get_portal_by_key(&key).await {
                Ok(portal) => {
                    let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
                    if portal.mxid().is_some() {
                        if let Err(e) = portal.update_matrix_room(
                            &matrix_client,
                            Some(&group.name),
                            group.notice.as_deref(),
                            group.avatar.as_deref(),
                        ).await {
                            warn!("Failed to update portal {}: {}", group.id, e);
                        }
                    } else if create_portals {
                        let bot_mxid = bridge.config.appservice.bot.mxid(&bridge.config.homeserver.domain);
                        let puppet_mxid = bridge.puppet_mxid(&group.id);
                        if let Err(e) = portal.create_matrix_room(
                            &matrix_client,
                            &bot_mxid,
                            &puppet_mxid,
                            Some(&group.name),
                            group.avatar.as_deref(),
                            false,
                            bridge.config.bridge.encryption.default,
                        ).await {
                            warn!("Failed to create portal for {}: {}", group.id, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to get portal for {}: {}", group.id, e);
                }
            }
        }
        Ok(())
    }

    /// Get or create the user's personal filtering space room.
    /// Ported from Go GetSpaceRoom.
    pub async fn get_or_create_space_room(
        &mut self,
        matrix_client: &MatrixClient,
    ) -> anyhow::Result<String> {
        if let Some(room_id) = &self.inner.space_room {
            return Ok(room_id.clone());
        }
        // Create a space room
        let initial_state = vec![
            serde_json::json!({
                "type": "m.room.type",
                "state_key": "",
                "content": { "type": "m.space" }
            }),
        ];

        let request = crate::matrix::types::CreateRoomRequest {
            visibility: Some("private".to_string()),
            name: Some("WeChat".to_string()),
            topic: Some("Your WeChat bridged chats".to_string()),
            preset: Some("private_chat".to_string()),
            is_direct: false,
            invite: vec![self.mxid.clone()],
            initial_state: Some(initial_state),
            ..Default::default()
        };
        let room_id = matrix_client.create_room(&request).await?;
        self.inner.space_room = Some(room_id.clone());
        self.db.update_user(&self.inner).await?;
        info!("Created space room {} for user {}", room_id, self.mxid);
        Ok(room_id)
    }

    /// Update the m.direct account data to track DM rooms.
    /// Ported from Go UpdateDirectChats.
    pub async fn update_direct_chats(
        &self,
        matrix_client: &MatrixClient,
        puppet_mxid: &str,
        room_id: &str,
    ) -> anyhow::Result<()> {
        // Would need to get current m.direct, add/update the entry, and PUT it back
        // For now, a simplified version that just logs
        debug!(
            "Would update m.direct for {} with puppet {} room {}",
            self.mxid, puppet_mxid, room_id
        );
        Ok(())
    }

    /// Start a private message flow: ensure portal + room exist, return room_id.
    /// Ported from Go StartPM.
    pub async fn start_pm(
        &self,
        bridge: &WechatBridge,
        uid: &str,
    ) -> anyhow::Result<String> {
        let key = PortalKey::new(uid.to_string(), self.uin().unwrap_or("").to_string());
        let portal = bridge.get_portal_by_key(&key).await?;
        let matrix_client = bridge.get_matrix_client();
        let puppet_mxid = bridge.puppet_mxid(uid);
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        let room_id = portal.get_matrix_room(
            &matrix_client,
            &self.mxid,
            &puppet_mxid,
            None,
            None,
            true,
            false,
        ).await?;
        Ok(room_id)
    }

    pub fn clone(&self) -> Self {
        Self {
            mxid: self.mxid.clone(),
            inner: self.inner.clone(),
            client: self.client.clone(),
            db: self.db.clone(),
            config: self.config.clone(),
        }
    }
}
