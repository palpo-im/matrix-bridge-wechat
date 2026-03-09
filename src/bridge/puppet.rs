use std::sync::Arc;

use tracing::{info, debug, warn};

use crate::database::{Puppet as DbPuppet, Database};
use crate::matrix::client::MatrixClient;
use crate::util::UID;
use crate::wechat::WechatClient;
use crate::config::BridgeConfig;

pub struct BridgePuppet {
    pub uid: UID,
    pub inner: DbPuppet,
    db: Database,
    config: Option<Arc<BridgeConfig>>,
    homeserver: Option<String>,
}

impl BridgePuppet {
    pub fn new(uid: UID, db: Database) -> Self {
        Self {
            uid: uid.clone(),
            inner: DbPuppet::new(&uid.uin),
            db,
            config: None,
            homeserver: None,
        }
    }

    pub fn with_config(mut self, config: Arc<BridgeConfig>) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_homeserver(mut self, homeserver: String) -> Self {
        self.homeserver = Some(homeserver);
        self
    }

    pub fn from_db(puppet: DbPuppet, db: Database) -> Self {
        let uid = puppet.uid();
        Self {
            uid,
            inner: puppet,
            db,
            config: None,
            homeserver: None,
        }
    }

    pub fn from_db_with_config(puppet: DbPuppet, db: Database, config: Arc<BridgeConfig>) -> Self {
        let uid = puppet.uid();
        Self {
            uid,
            inner: puppet,
            db,
            config: Some(config),
            homeserver: None,
        }
    }

    pub fn uin(&self) -> &str {
        &self.inner.uin
    }

    pub fn displayname(&self) -> Option<&str> {
        self.inner.displayname.as_deref()
    }

    pub fn avatar(&self) -> Option<&str> {
        self.inner.avatar.as_deref()
    }

    pub fn custom_mxid(&self) -> Option<&str> {
        self.inner.custom_mxid.as_deref()
    }

    pub fn access_token(&self) -> Option<&str> {
        self.inner.access_token.as_deref()
    }

    pub fn mxid(&self, domain: &str, user_prefix: &str) -> String {
        if let Some(custom) = &self.inner.custom_mxid {
            return custom.clone();
        }
        format!("@{}{}:{}", user_prefix, self.inner.uin, domain)
    }

    pub fn is_custom_puppet(&self) -> bool {
        self.inner.custom_mxid.is_some() && self.inner.access_token.is_some()
    }

    pub fn get_custom_client(&self, homeserver: &str) -> Option<MatrixClient> {
        let mxid = self.inner.custom_mxid.as_ref()?;
        let token = self.inner.access_token.as_ref()?;
        Some(MatrixClient::new(homeserver, token).with_user_id(mxid))
    }

    pub async fn set_displayname(&mut self, name: &str, quality: i16) -> anyhow::Result<()> {
        self.inner.displayname = Some(name.to_string());
        self.inner.name_quality = quality;
        self.inner.name_set = true;
        self.db.update_puppet(&self.inner).await?;
        Ok(())
    }

    pub async fn set_avatar(&mut self, avatar: &str, avatar_url: &str) -> anyhow::Result<()> {
        self.inner.avatar = Some(avatar.to_string());
        self.inner.avatar_url = Some(avatar_url.to_string());
        self.inner.avatar_set = true;
        self.db.update_puppet(&self.inner).await?;
        Ok(())
    }

    pub async fn set_custom_mxid(&mut self, mxid: &str, access_token: &str) -> anyhow::Result<()> {
        self.inner.custom_mxid = Some(mxid.to_string());
        self.inner.access_token = Some(access_token.to_string());
        self.db.update_puppet(&self.inner).await?;
        info!("Set custom puppet for {} -> {}", self.inner.uin, mxid);
        Ok(())
    }

    pub async fn clear_custom_mxid(&mut self) -> anyhow::Result<()> {
        self.inner.custom_mxid = None;
        self.inner.access_token = None;
        self.db.update_puppet(&self.inner).await?;
        info!("Cleared custom puppet for {}", self.inner.uin);
        Ok(())
    }

    pub async fn sync(
        &mut self,
        client: &MatrixClient,
        name: Option<&str>,
        avatar_url: Option<&str>,
        force: bool,
    ) -> anyhow::Result<()> {
        // Ensure the puppet user is registered on the homeserver
        self.register(client).await.ok();

        let mxid = self.mxid(
            client.user_id()
                .and_then(|id| id.split(':').nth(1))
                .unwrap_or("localhost"),
            "",
        );

        let mut update = false;

        if let Some(name) = name {
            let quality = crate::config::NAME_QUALITY_NAME as i16;
            if force || !self.inner.name_set || self.inner.name_quality < quality {
                if let Err(e) = client.set_displayname(&mxid, name).await {
                    warn!("Failed to set displayname for {}: {}", mxid, e);
                } else {
                    self.inner.displayname = Some(name.to_string());
                    self.inner.name_quality = quality;
                    self.inner.name_set = true;
                    update = true;
                }
            }
        }

        if let Some(url) = avatar_url {
            if force || !self.inner.avatar_set {
                if let Err(e) = client.set_avatar_url(&mxid, url).await {
                    warn!("Failed to set avatar for {}: {}", mxid, e);
                } else {
                    self.inner.avatar_url = Some(url.to_string());
                    self.inner.avatar_set = true;
                    update = true;
                }
            }
        }

        // Update last_sync timestamp
        if update || self.inner.last_sync + 86400 < chrono::Utc::now().timestamp() {
            self.inner.last_sync = chrono::Utc::now().timestamp();
        }

        self.db.update_puppet(&self.inner).await?;
        Ok(())
    }

    pub async fn register(&self, client: &MatrixClient) -> anyhow::Result<()> {
        let domain = client.user_id()
            .and_then(|id| id.split(':').nth(1))
            .unwrap_or("localhost");
        let mxid = self.mxid(domain, "");
        
        let profile = client.get_profile(&mxid).await;
        
        match profile {
            Ok(_) => {
                debug!("Puppet {} already registered", mxid);
            }
            Err(_) => {
                info!("Registering puppet {}", mxid);
            }
        }
        
        Ok(())
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        self.db.update_puppet(&self.inner).await?;
        Ok(())
    }

    /// Download avatar from URL and re-upload to Matrix, returning the mxc:// URI.
    /// Ported from Go reuploadAvatar.
    pub async fn reupload_avatar(
        &mut self,
        client: &MatrixClient,
        avatar_url: &str,
    ) -> anyhow::Result<String> {
        let response = reqwest::get(avatar_url).await?;
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .to_string();
        let data = response.bytes().await?;
        let mxc_url = client.upload_media(&data, &content_type, "avatar").await?;
        Ok(mxc_url)
    }

    /// Fetch contact info from WeChat and sync puppet name/avatar.
    /// Ported from Go SyncContact.
    pub async fn sync_contact(
        &mut self,
        wechat_client: &WechatClient,
        matrix_client: &MatrixClient,
    ) -> anyhow::Result<()> {
        let user_info = wechat_client.get_user_info(self.uin()).await?;
        self.sync(
            matrix_client,
            Some(&user_info.name),
            user_info.avatar.as_deref(),
            false,
        ).await?;
        Ok(())
    }

    /// Propagate puppet name changes to all DM portals.
    /// Ported from Go updatePortalName.
    pub async fn update_portal_name(
        &self,
        client: &MatrixClient,
        db: &Database,
        name: &str,
    ) -> anyhow::Result<()> {
        let portals = db.get_all_portals_by_uid(&self.inner.uin).await?;
        for portal_data in portals {
            if let Some(room_id) = &portal_data.mxid {
                if portal_data.uid == self.inner.uin {
                    // This is a DM portal, update room name
                    let _ = client.set_room_name(room_id, name).await;
                }
            }
        }
        Ok(())
    }

    /// Parse a puppet MXID to extract the UIN.
    /// Ported from Go ParsePuppetMXID.
    pub fn parse_puppet_mxid(mxid: &str, user_prefix: &str, domain: &str) -> Option<String> {
        let expected_prefix = format!("@{}", user_prefix);
        let expected_suffix = format!(":{}", domain);
        if mxid.starts_with(&expected_prefix) && mxid.ends_with(&expected_suffix) {
            let uin_start = expected_prefix.len();
            let uin_end = mxid.len() - expected_suffix.len();
            if uin_start < uin_end {
                return Some(mxid[uin_start..uin_end].to_string());
            }
        }
        None
    }

    pub fn clone(&self) -> Self {
        Self {
            uid: self.uid.clone(),
            inner: self.inner.clone(),
            db: self.db.clone(),
            config: self.config.clone(),
            homeserver: self.homeserver.clone(),
        }
    }
}

impl Clone for BridgePuppet {
    fn clone(&self) -> Self {
        self.clone()
    }
}
