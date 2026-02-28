use std::collections::HashMap;
use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;

use tokio::sync::RwLock;
use tracing::{info, error, warn, debug};

use crate::config::Config;
use crate::database::{Database, PortalKey, User as DbUser, Portal as DbPortal, Puppet as DbPuppet, Message as DbMessage};
use crate::wechat::{WechatService, WechatClient, Event, EventType};
use crate::matrix::types::RoomEvent;
use crate::matrix::AppServiceBridge;
use super::user::BridgeUser;
use super::portal::BridgePortal;
use super::puppet::BridgePuppet;
use super::command::CommandProcessor;

pub struct WechatBridge {
    pub config: Config,
    pub db: Database,
    pub wechat_service: Arc<WechatService>,
    command_processor: CommandProcessor,
    
    users_by_mxid: RwLock<HashMap<String, Arc<BridgeUser>>>,
    users_by_uin: RwLock<HashMap<String, Arc<BridgeUser>>>,
    portals_by_key: RwLock<HashMap<PortalKey, Arc<BridgePortal>>>,
    portals_by_mxid: RwLock<HashMap<String, Arc<BridgePortal>>>,
    puppets_by_uin: RwLock<HashMap<String, Arc<BridgePuppet>>>,
    puppets_by_mxid: RwLock<HashMap<String, Arc<BridgePuppet>>>,
}

impl WechatBridge {
    pub async fn new(config: Config) -> anyhow::Result<Self> {
        let db_type = &config.appservice.database.r#type;
        let db_uri = &config.appservice.database.uri;
        let max_open = config.appservice.database.max_open_conns;
        let max_idle = config.appservice.database.max_idle_conns;
        
        let db = Database::connect(db_type, db_uri, max_open, max_idle).await?;
        db.run_migrations().await?;
        
        let wechat_service = Arc::new(WechatService::new(
            config.bridge.listen_address.clone(),
            config.bridge.listen_secret.clone(),
        ));
        
        let command_processor = CommandProcessor::new(config.bridge.command_prefix.clone());
        
        Ok(Self {
            config,
            db,
            wechat_service,
            command_processor,
            users_by_mxid: RwLock::new(HashMap::new()),
            users_by_uin: RwLock::new(HashMap::new()),
            portals_by_key: RwLock::new(HashMap::new()),
            portals_by_mxid: RwLock::new(HashMap::new()),
            puppets_by_uin: RwLock::new(HashMap::new()),
            puppets_by_mxid: RwLock::new(HashMap::new()),
        })
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        info!("Starting WeChat bridge");
        
        let service = self.wechat_service.clone();
        tokio::spawn(async move {
            if let Err(e) = service.start().await {
                error!("WeChat service error: {}", e);
            }
        });
        
        self.start_users().await;
        
        let bridge = Arc::new(self.clone());
        let mut event_rx = self.wechat_service.subscribe_events();
        tokio::spawn(async move {
            while let Ok(event) = event_rx.recv().await {
                if let Err(e) = bridge.handle_wechat_event(event).await {
                    error!("Error handling WeChat event: {}", e);
                }
            }
        });
        
        info!("WeChat bridge started");
        Ok(())
    }

    async fn start_users(&self) {
        info!("Starting logged in users");
        match self.db.get_all_logged_in_users().await {
            Ok(users) => {
                for user in users {
                    info!("Found logged in user: {}", user.mxid);
                }
            }
            Err(e) => {
                error!("Failed to get logged in users: {}", e);
            }
        }
    }

    pub async fn stop(&self) {
        info!("Stopping WeChat bridge");
    }

    pub async fn get_user_by_mxid(&self, mxid: &str) -> anyhow::Result<Arc<BridgeUser>> {
        {
            let users = self.users_by_mxid.read().await;
            if let Some(user) = users.get(mxid) {
                return Ok(user.clone());
            }
        }
        
        let db_user = self.db.get_user_by_mxid(mxid).await?;
        let user = if let Some(db_user) = db_user {
            BridgeUser::from_db(db_user, self.db.clone())
        } else {
            let new_user = DbUser::new(mxid);
            self.db.insert_user(&new_user).await?;
            BridgeUser::from_db(new_user, self.db.clone())
        };
        
        let user = Arc::new(user);
        {
            let mut users = self.users_by_mxid.write().await;
            users.insert(mxid.to_string(), user.clone());
        }
        
        Ok(user)
    }

    pub async fn get_portal_by_key(&self, key: &PortalKey) -> anyhow::Result<Arc<BridgePortal>> {
        {
            let portals = self.portals_by_key.read().await;
            if let Some(portal) = portals.get(key) {
                return Ok(portal.clone());
            }
        }
        
        let db_portal = self.db.get_portal_by_key(key).await?;
        let portal = if let Some(db_portal) = db_portal {
            BridgePortal::from_db(db_portal, self.db.clone())
        } else {
            let new_portal = DbPortal {
                uid: key.uid.clone(),
                receiver: key.receiver.clone(),
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
            };
            self.db.insert_portal(&new_portal).await?;
            BridgePortal::from_db(new_portal, self.db.clone())
        };
        
        let portal = Arc::new(portal);
        {
            let mut portals = self.portals_by_key.write().await;
            portals.insert(key.clone(), portal.clone());
        }
        
        Ok(portal)
    }

    pub async fn get_portal_by_mxid(&self, mxid: &str) -> anyhow::Result<Option<Arc<BridgePortal>>> {
        {
            let portals = self.portals_by_mxid.read().await;
            if let Some(portal) = portals.get(mxid) {
                return Ok(Some(portal.clone()));
            }
        }
        
        if let Some(db_portal) = self.db.get_portal_by_mxid(mxid).await? {
            let portal = Arc::new(BridgePortal::from_db(db_portal, self.db.clone()));
            {
                let mut portals = self.portals_by_mxid.write().await;
                portals.insert(mxid.to_string(), portal.clone());
            }
            Ok(Some(portal))
        } else {
            Ok(None)
        }
    }

    pub async fn get_puppet_by_uin(&self, uin: &str) -> anyhow::Result<Arc<BridgePuppet>> {
        {
            let puppets = self.puppets_by_uin.read().await;
            if let Some(puppet) = puppets.get(uin) {
                return Ok(puppet.clone());
            }
        }
        
        let db_puppet = self.db.get_puppet_by_uin(uin).await?;
        let puppet = if let Some(db_puppet) = db_puppet {
            BridgePuppet::from_db(db_puppet, self.db.clone())
        } else {
            let new_puppet = DbPuppet::new(uin);
            self.db.insert_puppet(&new_puppet).await?;
            BridgePuppet::from_db(new_puppet, self.db.clone())
        };
        
        let puppet = Arc::new(puppet);
        {
            let mut puppets = self.puppets_by_uin.write().await;
            puppets.insert(uin.to_string(), puppet.clone());
        }
        
        Ok(puppet)
    }

    pub fn get_client(&self, mxid: &str) -> WechatClient {
        WechatClient::new(mxid.to_string(), self.wechat_service.clone())
    }

    pub fn get_matrix_client(&self) -> crate::matrix::client::MatrixClient {
        crate::matrix::client::MatrixClient::new(
            &self.config.homeserver.address,
            &self.config.appservice.as_token,
        ).with_user_id(&self.config.appservice.bot.mxid(&self.config.homeserver.domain))
    }

    pub fn format_username(&self, username: &str) -> String {
        self.config.format_username(username)
    }

    pub fn puppet_mxid(&self, uin: &str) -> String {
        let prefix = &self.config.bridge.user_prefix;
        format!("@{}{}:{}", prefix, uin, self.config.homeserver.domain)
    }

    pub async fn handle_wechat_event(&self, event: Event) -> anyhow::Result<()> {
        debug!("Handling WeChat event: {:?} from {}", event.event_type, event.from.id);
        
        let receiver = event.from.id.clone();
        
        match event.event_type {
            EventType::Text => {
                self.handle_text_event(event).await?;
            }
            EventType::Photo => {
                self.handle_photo_event(event).await?;
            }
            EventType::Video => {
                self.handle_video_event(event).await?;
            }
            EventType::Audio => {
                self.handle_audio_event(event).await?;
            }
            EventType::File => {
                self.handle_file_event(event).await?;
            }
            EventType::Sticker => {
                self.handle_sticker_event(event).await?;
            }
            EventType::Location => {
                self.handle_location_event(event).await?;
            }
            EventType::App => {
                self.handle_app_event(event).await?;
            }
            EventType::Revoke => {
                self.handle_revoke_event(event).await?;
            }
            EventType::Notice | EventType::Voip | EventType::System => {
                debug!("Unhandled event type: {:?}", event.event_type);
            }
        }
        
        Ok(())
    }

    async fn handle_text_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        let puppet = self.get_puppet_by_uin(sender_id).await?;
        
        let Some(content) = &event.content else {
            return Ok(());
        };

        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            Some(content),
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let formatted = crate::formatter::wechat_to_matrix(content);
        
        let event_id = if let Some(reply) = &event.reply {
            if let Some(msg) = self.db.get_message_by_wechat_id(&reply.id).await? {
                let reply_content = serde_json::json!({
                    "m.relates_to": {
                        "m.in_reply_to": {
                            "event_id": msg.mxid
                        }
                    }
                });
                client.send_text_html(&room_id, content, &formatted).await?
            } else {
                client.send_text_html(&room_id, content, &formatted).await?
            }
        } else {
            client.send_text_html(&room_id, content, &formatted).await?
        };

        let msg = DbMessage {
            chat_uid: chat_id.clone(),
            chat_receiver: sender_id.to_string(),
            msg_id: event.id.clone(),
            mxid: event_id.clone(),
            sender: puppet_mxid,
            timestamp: event.timestamp,
            sent: true,
            error: None,
            msg_type: String::new(),
        };
        self.db.insert_message(&msg).await?;
        
        debug!("Bridged text message {} -> {}", event.id, event_id);
        Ok(())
    }

    async fn handle_photo_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        let puppet = self.get_puppet_by_uin(sender_id).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            warn!("Photo event without data");
            return Ok(());
        };
        
        let xml = data.get("xml")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let wechat_client = self.get_client("");
        match wechat_client.download_image(xml).await {
            Ok(image_data) => {
                let content_type = "image/jpeg";
                let filename = format!("image_{}.jpg", event.timestamp);
                
                match client.upload_media(&image_data, content_type, &filename).await {
                    Ok(mxc_url) => {
                        let content = serde_json::json!({
                            "msgtype": "m.image",
                            "body": filename,
                            "url": mxc_url,
                            "info": {
                                "mimetype": content_type,
                                "size": image_data.len() as u64,
                            }
                        });
                        
                        let event_id = client.send_message(&room_id, "m.room.message", &content, None).await?;
                        
                        let msg = DbMessage {
                            chat_uid: chat_id.clone(),
                            chat_receiver: sender_id.to_string(),
                            msg_id: event.id.clone(),
                            mxid: event_id.clone(),
                            sender: puppet_mxid.clone(),
                            timestamp: event.timestamp,
                            sent: true,
                            error: None,
                            msg_type: String::new(),
                        };
                        self.db.insert_message(&msg).await?;
                        
                        debug!("Bridged photo message {} -> {}", event.id, event_id);
                    }
                    Err(e) => {
                        warn!("Failed to upload image: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download image: {}", e);
            }
        }
        
        Ok(())
    }

    async fn handle_video_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        let puppet = self.get_puppet_by_uin(sender_id).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            warn!("Video event without data");
            return Ok(());
        };
        
        let xml = data.get("xml")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let wechat_client = self.get_client("");
        match wechat_client.download_video(xml).await {
            Ok(video_data) => {
                let content_type = "video/mp4";
                let filename = format!("video_{}.mp4", event.timestamp);
                
                match client.upload_media(&video_data, content_type, &filename).await {
                    Ok(mxc_url) => {
                        let content = serde_json::json!({
                            "msgtype": "m.video",
                            "body": filename,
                            "url": mxc_url,
                            "info": {
                                "mimetype": content_type,
                                "size": video_data.len() as u64,
                            }
                        });
                        
                        let event_id = client.send_message(&room_id, "m.room.message", &content, None).await?;
                        
                        let msg = DbMessage {
                            chat_uid: chat_id.clone(),
                            chat_receiver: sender_id.to_string(),
                            msg_id: event.id.clone(),
                            mxid: event_id.clone(),
                            sender: puppet_mxid.clone(),
                            timestamp: event.timestamp,
                            sent: true,
                            error: None,
                            msg_type: String::new(),
                        };
                        self.db.insert_message(&msg).await?;
                        
                        debug!("Bridged video message {} -> {}", event.id, event_id);
                    }
                    Err(e) => {
                        warn!("Failed to upload video: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download video: {}", e);
            }
        }
        
        Ok(())
    }

    async fn handle_audio_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            warn!("Audio event without data");
            return Ok(());
        };
        
        let xml = data.get("xml")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let wechat_client = self.get_client("");
        match wechat_client.download_audio(xml).await {
            Ok(audio_data) => {
                let content_type = "audio/ogg";
                let filename = format!("audio_{}.ogg", event.timestamp);
                
                match client.upload_media(&audio_data, content_type, &filename).await {
                    Ok(mxc_url) => {
                        let content = serde_json::json!({
                            "msgtype": "m.audio",
                            "body": filename,
                            "url": mxc_url,
                            "info": {
                                "mimetype": content_type,
                                "size": audio_data.len() as u64,
                            }
                        });
                        
                        let event_id = client.send_message(&room_id, "m.room.message", &content, None).await?;
                        
                        let msg = DbMessage {
                            chat_uid: chat_id.clone(),
                            chat_receiver: sender_id.to_string(),
                            msg_id: event.id.clone(),
                            mxid: event_id.clone(),
                            sender: puppet_mxid.clone(),
                            timestamp: event.timestamp,
                            sent: true,
                            error: None,
                            msg_type: String::new(),
                        };
                        self.db.insert_message(&msg).await?;
                        
                        debug!("Bridged audio message {} -> {}", event.id, event_id);
                    }
                    Err(e) => {
                        warn!("Failed to upload audio: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download audio: {}", e);
            }
        }
        
        Ok(())
    }

    async fn handle_file_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            warn!("File event without data");
            return Ok(());
        };
        
        let xml = data.get("xml")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let filename = data.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&event.id);

        let wechat_client = self.get_client("");
        match wechat_client.download_file(xml).await {
            Ok(file_data) => {
                let content_type = "application/octet-stream";
                
                match client.upload_media(&file_data, content_type, filename).await {
                    Ok(mxc_url) => {
                        let content = serde_json::json!({
                            "msgtype": "m.file",
                            "body": filename,
                            "url": mxc_url,
                            "info": {
                                "mimetype": content_type,
                                "size": file_data.len() as u64,
                            }
                        });
                        
                        let event_id = client.send_message(&room_id, "m.room.message", &content, None).await?;
                        
                        let msg = DbMessage {
                            chat_uid: chat_id.clone(),
                            chat_receiver: sender_id.to_string(),
                            msg_id: event.id.clone(),
                            mxid: event_id.clone(),
                            sender: puppet_mxid.clone(),
                            timestamp: event.timestamp,
                            sent: true,
                            error: None,
                            msg_type: String::new(),
                        };
                        self.db.insert_message(&msg).await?;
                        
                        debug!("Bridged file message {} -> {}", event.id, event_id);
                    }
                    Err(e) => {
                        warn!("Failed to upload file: {}", e);
                    }
                }
            }
            Err(e) => {
                warn!("Failed to download file: {}", e);
            }
        }
        
        Ok(())
    }

    async fn handle_sticker_event(&self, event: Event) -> anyhow::Result<()> {
        debug!("Sticker event received: {}", event.id);
        Ok(())
    }

    async fn handle_location_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            warn!("Location event without data");
            return Ok(());
        };

        let lat = data.get("latitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lon = data.get("longitude").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let name = data.get("name").and_then(|v| v.as_str()).unwrap_or("Location");
        let address = data.get("address").and_then(|v| v.as_str()).unwrap_or("");

        let body = if !address.is_empty() {
            format!("{}: {}", name, address)
        } else {
            name.to_string()
        };

        let geo_uri = format!("geo:{},{}", lat, lon);
        
        let content = serde_json::json!({
            "msgtype": "m.location",
            "body": body,
            "geo_uri": geo_uri,
            "info": {
                "name": name,
            }
        });
        
        let event_id = client.send_message(&room_id, "m.room.message", &content, None).await?;
        
        let msg = DbMessage {
            chat_uid: chat_id.clone(),
            chat_receiver: sender_id.to_string(),
            msg_id: event.id.clone(),
            mxid: event_id.clone(),
            sender: puppet_mxid,
            timestamp: event.timestamp,
            sent: true,
            error: None,
            msg_type: String::new(),
        };
        self.db.insert_message(&msg).await?;
        
        debug!("Bridged location message {} -> {}", event.id, event_id);
        Ok(())
    }

    async fn handle_app_event(&self, event: Event) -> anyhow::Result<()> {
        let chat_id = &event.chat.id;
        let sender_id = &event.from.id;
        
        let key = PortalKey::new(chat_id.clone(), sender_id.clone());
        let portal = self.get_portal_by_key(&key).await?;
        
        let client = self.get_matrix_client();
        let puppet_mxid = self.puppet_mxid(sender_id);
        
        let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
        
        let room_id = portal.get_matrix_room(
            &client,
            &self.config.appservice.bot.mxid(&self.config.homeserver.domain),
            &puppet_mxid,
            None,
            None,
            event.chat.chat_type == crate::wechat::ChatType::Private,
            self.config.bridge.encryption.default,
        ).await?;

        {
            let mut portals = self.portals_by_mxid.write().await;
            portals.insert(room_id.clone(), Arc::new(portal.clone()));
        }

        let Some(data) = &event.data else {
            return Ok(());
        };

        let title = data.get("title").and_then(|v| v.as_str()).unwrap_or("Link");
        let desc = data.get("desc").and_then(|v| v.as_str()).unwrap_or("");
        let url = data.get("url").and_then(|v| v.as_str()).unwrap_or("");

        let body = format!("{}\n\n{}", title, url);
        let html = format!(
            "<strong>{}</strong><br/><br/><a href=\"{}\">{}</a>",
            title, url, url
        );
        
        let event_id = client.send_text_html(&room_id, &body, &html).await?;
        
        let msg = DbMessage {
            chat_uid: chat_id.clone(),
            chat_receiver: sender_id.to_string(),
            msg_id: event.id.clone(),
            mxid: event_id.clone(),
            sender: puppet_mxid,
            timestamp: event.timestamp,
            sent: true,
            error: None,
            msg_type: String::new(),
        };
        self.db.insert_message(&msg).await?;
        
        debug!("Bridged app message {} -> {}", event.id, event_id);
        Ok(())
    }

    async fn handle_revoke_event(&self, event: Event) -> anyhow::Result<()> {
        let Some(data) = &event.data else {
            return Ok(());
        };
        
        let msg_id = data.get("msg_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&event.id);

        if let Some(msg) = self.db.get_message_by_wechat_id(msg_id).await? {
            let client = self.get_matrix_client();
            match client.redact(&msg.chat_uid, &msg.mxid, Some("Message revoked")).await {
                Ok(redact_event_id) => {
                    info!("Revoked message {} -> {}", msg_id, redact_event_id);
                }
                Err(e) => {
                    warn!("Failed to redact message: {}", e);
                }
            }
        }
        
        Ok(())
    }

    pub fn command_processor(&self) -> &CommandProcessor {
        &self.command_processor
    }
}

impl Clone for WechatBridge {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            db: self.db.clone(),
            wechat_service: self.wechat_service.clone(),
            command_processor: self.command_processor.clone(),
            users_by_mxid: RwLock::new(HashMap::new()),
            users_by_uin: RwLock::new(HashMap::new()),
            portals_by_key: RwLock::new(HashMap::new()),
            portals_by_mxid: RwLock::new(HashMap::new()),
            puppets_by_uin: RwLock::new(HashMap::new()),
            puppets_by_mxid: RwLock::new(HashMap::new()),
        }
    }
}

impl AppServiceBridge for WechatBridge {
    fn handle_transaction(&self, _txn_id: &str, events: Vec<RoomEvent>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + '_>> {
        Box::pin(async move {
            let handler = crate::matrix::event_handler::MatrixEventHandler::new(Arc::new(self.clone()));
            for event in events {
                if let Err(e) = handler.handle_event(&event).await {
                    warn!("Error handling event: {}", e);
                }
            }
            Ok(())
        })
    }

    fn is_user_in_namespace(&self, mxid: &str) -> bool {
        let prefix = format!("@{}", self.config.bridge.user_prefix);
        mxid.starts_with(&prefix)
    }
}
