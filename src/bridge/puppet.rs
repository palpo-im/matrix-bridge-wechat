use std::sync::Arc;

use tracing::{info, debug, warn};

use crate::database::{Puppet as DbPuppet, Database};
use crate::matrix::client::MatrixClient;
use crate::util::UID;
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
        let mxid = self.mxid(
            client.user_id()
                .and_then(|id| id.split(':').nth(1))
                .unwrap_or("localhost"),
            "",
        );

        if let Some(name) = name {
            let quality = crate::config::NAME_QUALITY_NAME as i16;
            if force || !self.inner.name_set || self.inner.name_quality < quality {
                if let Err(e) = client.set_displayname(&mxid, name).await {
                    warn!("Failed to set displayname for {}: {}", mxid, e);
                } else {
                    self.inner.displayname = Some(name.to_string());
                    self.inner.name_quality = quality;
                    self.inner.name_set = true;
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
                }
            }
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
