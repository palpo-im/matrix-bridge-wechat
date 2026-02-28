use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::{debug, info, warn, error};

use crate::matrix::types::RoomEvent;
use crate::bridge::WechatBridge;

pub struct MatrixEventHandler {
    bridge: Arc<WechatBridge>,
    event_age_limit: Duration,
}

impl MatrixEventHandler {
    pub fn new(bridge: Arc<WechatBridge>) -> Self {
        Self {
            bridge,
            event_age_limit: Duration::from_secs(300),
        }
    }

    pub fn with_event_age_limit(mut self, limit: Duration) -> Self {
        self.event_age_limit = limit;
        self
    }

    pub async fn handle_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        if self.is_event_too_old(event) {
            debug!("Dropping old event: {:?}", event.event_id);
            return Ok(());
        }

        if self.is_own_event(event) {
            debug!("Dropping own event: {:?}", event.event_id);
            return Ok(());
        }

        match event.event_type.as_str() {
            "m.room.message" | "m.room.sticker" => {
                self.handle_message_event(event).await?;
            }
            "m.room.redaction" => {
                self.handle_redaction_event(event).await?;
            }
            "m.room.reaction" => {
                self.handle_reaction_event(event).await?;
            }
            "m.room.member" => {
                self.handle_member_event(event).await?;
            }
            "m.room.encryption" => {
                self.handle_encryption_event(event).await?;
            }
            "m.room.name" => {
                self.handle_room_name_event(event).await?;
            }
            "m.room.topic" => {
                self.handle_room_topic_event(event).await?;
            }
            "m.room.avatar" => {
                self.handle_room_avatar_event(event).await?;
            }
            "m.room.power_levels" => {
                self.handle_power_levels_event(event).await?;
            }
            "m.typing" => {
                self.handle_typing_event(event).await?;
            }
            "m.presence" => {
                self.handle_presence_event(event).await?;
            }
            "m.receipt" => {
                self.handle_receipt_event(event).await?;
            }
            _ => {
                debug!("Unhandled event type: {}", event.event_type);
            }
        }

        Ok(())
    }

    fn is_event_too_old(&self, event: &RoomEvent) -> bool {
        let Some(ts) = event.origin_server_ts else {
            return false;
        };
        
        let event_time = UNIX_EPOCH + Duration::from_millis(ts as u64);
        let now = SystemTime::now();
        
        match now.duration_since(event_time) {
            Ok(age) => age > self.event_age_limit,
            Err(_) => false,
        }
    }

    fn is_own_event(&self, event: &RoomEvent) -> bool {
        let bot_mxid = self.bridge.config.appservice.bot.mxid(&self.bridge.config.homeserver.domain);
        if let Some(sender) = &event.sender {
            sender == &bot_mxid || self.is_puppet_mxid(sender)
        } else {
            false
        }
    }

    fn is_puppet_mxid(&self, mxid: &str) -> bool {
        let prefix = format!("@{}", self.bridge.config.bridge.user_prefix);
        mxid.starts_with(&prefix)
    }

    async fn handle_message_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        let Some(sender) = &event.sender else {
            return Ok(());
        };

        debug!("Handling message event in room {} from {}", room_id, sender);

        let content = event.content.as_ref();
        let msgtype = content
            .and_then(|c| c.get("msgtype"))
            .and_then(|v| v.as_str())
            .unwrap_or("m.text");
        let body = content
            .and_then(|c| c.get("body"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if body.is_empty() && msgtype != "m.sticker" {
            debug!("Empty message body, skipping");
            return Ok(());
        }

        let command_prefix = self.bridge.command_processor().command_prefix();
        if body.starts_with(command_prefix) {
            self.handle_command(event, body).await?;
            return Ok(());
        }

        let Some(portal) = self.get_portal_by_mxid(room_id).await? else {
            debug!("No portal found for room {}", room_id);
            return Ok(());
        };

        let Some(user) = self.get_user_by_mxid(sender).await? else {
            debug!("No user found for mxid {}", sender);
            return Ok(());
        };

        match msgtype {
            "m.text" | "m.notice" | "m.emote" => {
                self.handle_text_message(&user, &portal, event, body, msgtype).await?;
            }
            "m.image" => {
                self.handle_image_message(&user, &portal, event).await?;
            }
            "m.video" => {
                self.handle_video_message(&user, &portal, event).await?;
            }
            "m.audio" => {
                self.handle_audio_message(&user, &portal, event).await?;
            }
            "m.file" => {
                self.handle_file_message(&user, &portal, event).await?;
            }
            "m.sticker" => {
                self.handle_sticker_message(&user, &portal, event).await?;
            }
            _ => {
                warn!("Unsupported msgtype: {}", msgtype);
            }
        }

        Ok(())
    }

    async fn handle_redaction_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };

        debug!("Handling redaction event in room {}", room_id);

        let redacts = event.content.as_ref()
            .and_then(|c| c.get("redacts"))
            .and_then(|v| v.as_str())
            .or_else(|| event.redacts.as_deref());

        let Some(redacted_event_id) = redacts else {
            debug!("No redacts field in redaction event");
            return Ok(());
        };

        let Some(portal) = self.get_portal_by_mxid(room_id).await? else {
            return Ok(());
        };

        let key = portal.key.clone();
        let msg = self.bridge.db.get_message_by_mxid(redacted_event_id).await?;
        
        if let Some(msg) = msg {
            let client = self.bridge.get_client(&portal.key.receiver);
            if let Err(e) = client.revoke_message(&key.uid, &msg.msg_id).await {
                warn!("Failed to revoke message on WeChat: {}", e);
            } else {
                info!("Revoked message {} on WeChat", msg.msg_id);
            }
        }

        Ok(())
    }

    async fn handle_reaction_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        debug!("Handling reaction event: {:?}", event.event_id);
        Ok(())
    }

    async fn handle_member_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        let Some(sender) = &event.sender else {
            return Ok(());
        };

        let membership = event.content.as_ref()
            .and_then(|c| c.get("membership"))
            .and_then(|v| v.as_str())
            .unwrap_or("leave");

        debug!("Member event in {}: {} -> {}", room_id, sender, membership);

        if self.is_puppet_mxid(sender) {
            return Ok(());
        }

        match membership {
            "invite" => {
                self.handle_invite(event).await?;
            }
            "join" => {
                self.handle_join(event).await?;
            }
            "leave" => {
                self.handle_leave(event).await?;
            }
            "ban" => {
                self.handle_ban(event).await?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn handle_invite(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        let state_key = event.state_key.as_deref();
        
        let bot_mxid = self.bridge.config.appservice.bot.mxid(&self.bridge.config.homeserver.domain);
        if state_key == Some(bot_mxid.as_str()) {
            info!("Bot invited to room {}, auto-joining", room_id);
            let client = self.bridge.get_matrix_client();
            if let Err(e) = client.join_room(room_id).await {
                warn!("Failed to join room {}: {}", room_id, e);
            }
        }

        Ok(())
    }

    async fn handle_join(&self, _event: &RoomEvent) -> anyhow::Result<()> {
        Ok(())
    }

    async fn handle_leave(&self, _event: &RoomEvent) -> anyhow::Result<()> {
        Ok(())
    }

    async fn handle_ban(&self, _event: &RoomEvent) -> anyhow::Result<()> {
        Ok(())
    }

    async fn handle_encryption_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };

        info!("Encryption enabled in room {}", room_id);

        let Some(portal) = self.get_portal_by_mxid(room_id).await? else {
            return Ok(());
        };

        let mut portal = portal.as_ref().clone();
        portal.set_encrypted(true).await?;

        Ok(())
    }

    async fn handle_room_name_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        debug!("Room name changed: {:?}", event);
        Ok(())
    }

    async fn handle_room_topic_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        debug!("Room topic changed: {:?}", event);
        Ok(())
    }

    async fn handle_room_avatar_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        debug!("Room avatar changed: {:?}", event);
        Ok(())
    }

    async fn handle_power_levels_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        debug!("Power levels changed: {:?}", event);
        Ok(())
    }

    async fn handle_typing_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        
        let typing_users = event.content.as_ref()
            .and_then(|c| c.get("user_ids"))
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect::<Vec<_>>());
        
        if let Some(users) = typing_users {
            debug!("Typing event in room {}: {} users typing", room_id, users.len());
        }
        
        Ok(())
    }

    async fn handle_presence_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(sender) = &event.sender else {
            return Ok(());
        };
        
        let presence = event.content.as_ref()
            .and_then(|c| c.get("presence"))
            .and_then(|v| v.as_str())
            .unwrap_or("offline");
        
        debug!("Presence event from {}: {}", sender, presence);
        
        Ok(())
    }

    async fn handle_receipt_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        
        debug!("Receipt event in room {}", room_id);
        
        Ok(())
    }

    async fn handle_command(&self, event: &RoomEvent, body: &str) -> anyhow::Result<()> {
        let Some(room_id) = &event.room_id else {
            return Ok(());
        };
        let Some(sender) = &event.sender else {
            return Ok(());
        };

        let result = self.bridge.command_processor().parse_command(body);
        if let Some((cmd, args)) = result {
            let outcome = self.bridge.command_processor().process(&cmd, &args);
            
            let client = self.bridge.get_matrix_client();
            let reply = match outcome {
                crate::bridge::command::CommandResult::Success(msg) => msg,
                crate::bridge::command::CommandResult::Error(msg) => msg,
                crate::bridge::command::CommandResult::NeedsLogin => {
                    "Please login first using `login` command.".to_string()
                }
                crate::bridge::command::CommandResult::Login => {
                    let user = self.get_or_create_user_by_mxid(sender).await?;
                    let mut user = user.as_ref().clone();
                    
                    if user.is_logged_in() {
                        "You are already logged in.".to_string()
                    } else {
                        user.set_client(self.bridge.get_client(sender));
                        match user.login(self.bridge.wechat_service.clone()).await {
                            Ok(_) => {
                                if let Some(room) = user.management_room() {
                                    let _ = user.get_or_create_management_room(&client, &self.bridge.config.appservice.bot.mxid(&self.bridge.config.homeserver.domain)).await;
                                }
                                "Login successful!".to_string()
                            }
                            Err(e) => {
                                format!("Login failed: {}", e)
                            }
                        }
                    }
                }
                crate::bridge::command::CommandResult::Logout => {
                    let user = self.get_user_by_mxid(sender).await?;
                    if let Some(user) = user {
                        let mut user = user.as_ref().clone();
                        user.logout().await?;
                        "Logged out successfully.".to_string()
                    } else {
                        "You are not logged in.".to_string()
                    }
                }
                crate::bridge::command::CommandResult::ListContacts => {
                    let user = self.get_user_by_mxid(sender).await?;
                    if let Some(user) = user {
                        if let Some(wechat_client) = user.get_client() {
                            match wechat_client.get_friend_list().await {
                                Ok(friends) => {
                                    let mut lines = vec![format!("You have {} contacts:", friends.len())];
                                    for friend in friends.iter().take(20) {
                                        let name = friend.name.as_str();
                                        let remark = friend.remark.as_deref().unwrap_or("");
                                        if remark.is_empty() {
                                            lines.push(format!("- {}", name));
                                        } else {
                                            lines.push(format!("- {} ({})", name, remark));
                                        }
                                    }
                                    if friends.len() > 20 {
                                        lines.push(format!("... and {} more", friends.len() - 20));
                                    }
                                    lines.join("\n")
                                }
                                Err(e) => format!("Failed to get contacts: {}", e)
                            }
                        } else {
                            "Please login first.".to_string()
                        }
                    } else {
                        "Please login first.".to_string()
                    }
                }
                crate::bridge::command::CommandResult::ListGroups => {
                    let user = self.get_user_by_mxid(sender).await?;
                    if let Some(user) = user {
                        if let Some(wechat_client) = user.get_client() {
                            match wechat_client.get_group_list().await {
                                Ok(groups) => {
                                    let mut lines = vec![format!("You have {} groups:", groups.len())];
                                    for group in groups.iter().take(20) {
                                        lines.push(format!("- {} ({})", group.name, group.id));
                                    }
                                    if groups.len() > 20 {
                                        lines.push(format!("... and {} more", groups.len() - 20));
                                    }
                                    lines.join("\n")
                                }
                                Err(e) => format!("Failed to get groups: {}", e)
                            }
                        } else {
                            "Please login first.".to_string()
                        }
                    } else {
                        "Please login first.".to_string()
                    }
                }
                crate::bridge::command::CommandResult::SyncContacts => {
                    "Syncing contacts...".to_string()
                }
                crate::bridge::command::CommandResult::SyncGroups => {
                    "Syncing groups...".to_string()
                }
                crate::bridge::command::CommandResult::SyncSpace => {
                    "Syncing space...".to_string()
                }
                crate::bridge::command::CommandResult::DeletePortal => {
                    let user = self.get_user_by_mxid(sender).await?;
                    if let Some(_user) = user {
                        if let Some(portal) = self.bridge.get_portal_by_mxid(room_id).await? {
                            let mut portal = Arc::try_unwrap(portal).unwrap_or_else(|p| (*p).clone());
                            portal.cleanup(&client).await?;
                            "Portal deleted.".to_string()
                        } else {
                            "This is not a portal room.".to_string()
                        }
                    } else {
                        "User not found.".to_string()
                    }
                }
                crate::bridge::command::CommandResult::DeleteAllPortals => {
                    let portals = self.bridge.db.get_all_portals_with_mxid().await?;
                    let count = portals.len();
                    for portal in portals {
                        let p = crate::bridge::portal::BridgePortal::from_db(portal, self.bridge.db.clone());
                        let mut p = p;
                        if let Err(e) = p.cleanup(&client).await {
                            warn!("Failed to cleanup portal: {}", e);
                        }
                    }
                    format!("Deleted {} portals.", count)
                }
                crate::bridge::command::CommandResult::DoublePuppet(token) => {
                    match token {
                        Some(access_token) => {
                            let user = self.get_user_by_mxid(sender).await?;
                            if let Some(user) = user {
                                if let Some(uin) = user.uin() {
                                    let puppet = self.bridge.get_puppet_by_uin(uin).await?;
                                    let mut puppet = Arc::try_unwrap(puppet).unwrap_or_else(|p| (*p).clone());
                                    puppet.set_custom_mxid(sender, &access_token).await?;
                                    format!("Double puppeting enabled for {}", sender)
                                } else {
                                    "Please login to WeChat first.".to_string()
                                }
                            } else {
                                "User not found.".to_string()
                            }
                        }
                        None => {
                            "Usage: double-puppet <access_token>".to_string()
                        }
                    }
                }
            };

            client.send_notice(room_id, &reply).await?;
        }

        Ok(())
    }

    async fn handle_text_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
        body: &str,
        msgtype: &str,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let text = if msgtype == "m.emote" {
            format!("/me {}", body)
        } else {
            body.to_string()
        };

        let reply_to = self.get_reply_target(event).await?;

        if let Err(e) = client.send_text_message(&portal.key.uid, &text, reply_to.as_deref()).await {
            warn!("Failed to send text message to WeChat: {}", e);
        }

        Ok(())
    }

    async fn handle_image_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let content = event.content.as_ref();
        let url = content
            .and_then(|c| c.get("url"))
            .and_then(|v| v.as_str());
        
        let Some(url) = url else {
            warn!("Image message without URL");
            return Ok(());
        };

        debug!("Downloading image from {}", url);
        
        let matrix_client = self.bridge.get_matrix_client();
        let image_data = match matrix_client.download_media(url).await {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to download image: {}", e);
                return Ok(());
            }
        };

        let reply_to = self.get_reply_target(event).await?;
        
        match client.send_image_message(&portal.key.uid, &image_data, reply_to.as_deref()).await {
            Ok(msg_id) => {
                info!("Sent image message to WeChat: {}", msg_id);
                if let Some(event_id) = &event.event_id {
                    if let Some(room_id) = &event.room_id {
                        let msg = crate::database::Message {
                            chat_uid: portal.key.uid.clone(),
                            chat_receiver: portal.key.receiver.clone(),
                            msg_id,
                            mxid: event_id.clone(),
                            sender: event.sender.clone().unwrap_or_default(),
                            timestamp: event.origin_server_ts.unwrap_or(0),
                            sent: true,
                            error: None,
                            msg_type: "m.image".to_string(),
                        };
                        self.bridge.db.insert_message(&msg).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send image message to WeChat: {}", e);
            }
        }

        Ok(())
    }

    async fn handle_video_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let content = event.content.as_ref();
        let url = content
            .and_then(|c| c.get("url"))
            .and_then(|v| v.as_str());
        
        let Some(url) = url else {
            warn!("Video message without URL");
            return Ok(());
        };

        debug!("Downloading video from {}", url);
        
        let matrix_client = self.bridge.get_matrix_client();
        let video_data = match matrix_client.download_media(url).await {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to download video: {}", e);
                return Ok(());
            }
        };

        let reply_to = self.get_reply_target(event).await?;
        
        match client.send_video_message(&portal.key.uid, &video_data, reply_to.as_deref()).await {
            Ok(msg_id) => {
                info!("Sent video message to WeChat: {}", msg_id);
                if let Some(event_id) = &event.event_id {
                    if let Some(room_id) = &event.room_id {
                        let msg = crate::database::Message {
                            chat_uid: portal.key.uid.clone(),
                            chat_receiver: portal.key.receiver.clone(),
                            msg_id,
                            mxid: event_id.clone(),
                            sender: event.sender.clone().unwrap_or_default(),
                            timestamp: event.origin_server_ts.unwrap_or(0),
                            sent: true,
                            error: None,
                            msg_type: "m.video".to_string(),
                        };
                        self.bridge.db.insert_message(&msg).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send video message to WeChat: {}", e);
            }
        }

        Ok(())
    }

    async fn handle_audio_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let content = event.content.as_ref();
        let url = content
            .and_then(|c| c.get("url"))
            .and_then(|v| v.as_str());
        
        let Some(url) = url else {
            warn!("Audio message without URL");
            return Ok(());
        };

        debug!("Downloading audio from {}", url);
        
        let matrix_client = self.bridge.get_matrix_client();
        let audio_data = match matrix_client.download_media(url).await {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to download audio: {}", e);
                return Ok(());
            }
        };

        let reply_to = self.get_reply_target(event).await?;
        
        let body = content
            .and_then(|c| c.get("body"))
            .and_then(|v| v.as_str())
            .unwrap_or("audio");
        
        match client.send_file_message(&portal.key.uid, &audio_data, body, reply_to.as_deref()).await {
            Ok(msg_id) => {
                info!("Sent audio message to WeChat: {}", msg_id);
                if let Some(event_id) = &event.event_id {
                    if let Some(room_id) = &event.room_id {
                        let msg = crate::database::Message {
                            chat_uid: portal.key.uid.clone(),
                            chat_receiver: portal.key.receiver.clone(),
                            msg_id,
                            mxid: event_id.clone(),
                            sender: event.sender.clone().unwrap_or_default(),
                            timestamp: event.origin_server_ts.unwrap_or(0),
                            sent: true,
                            error: None,
                            msg_type: "m.audio".to_string(),
                        };
                        self.bridge.db.insert_message(&msg).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send audio message to WeChat: {}", e);
            }
        }

        Ok(())
    }

    async fn handle_file_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let content = event.content.as_ref();
        let url = content
            .and_then(|c| c.get("url"))
            .and_then(|v| v.as_str());
        let filename = content
            .and_then(|c| c.get("body"))
            .and_then(|v| v.as_str())
            .unwrap_or("file");
        
        let Some(url) = url else {
            warn!("File message without URL");
            return Ok(());
        };

        debug!("Downloading file from {}", url);
        
        let matrix_client = self.bridge.get_matrix_client();
        let file_data = match matrix_client.download_media(url).await {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to download file: {}", e);
                return Ok(());
            }
        };

        let reply_to = self.get_reply_target(event).await?;
        
        match client.send_file_message(&portal.key.uid, &file_data, filename, reply_to.as_deref()).await {
            Ok(msg_id) => {
                info!("Sent file message to WeChat: {}", msg_id);
                if let Some(event_id) = &event.event_id {
                    if let Some(room_id) = &event.room_id {
                        let msg = crate::database::Message {
                            chat_uid: portal.key.uid.clone(),
                            chat_receiver: portal.key.receiver.clone(),
                            msg_id,
                            mxid: event_id.clone(),
                            sender: event.sender.clone().unwrap_or_default(),
                            timestamp: event.origin_server_ts.unwrap_or(0),
                            sent: true,
                            error: None,
                            msg_type: "m.file".to_string(),
                        };
                        self.bridge.db.insert_message(&msg).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send file message to WeChat: {}", e);
            }
        }

        Ok(())
    }

    async fn handle_sticker_message(
        &self,
        user: &crate::bridge::user::BridgeUser,
        portal: &crate::bridge::portal::BridgePortal,
        event: &RoomEvent,
    ) -> anyhow::Result<()> {
        let Some(client) = user.get_client() else {
            warn!("User has no WeChat client");
            return Ok(());
        };

        let content = event.content.as_ref();
        let url = content
            .and_then(|c| c.get("url"))
            .and_then(|v| v.as_str());
        
        let Some(url) = url else {
            warn!("Sticker message without URL");
            return Ok(());
        };

        debug!("Downloading sticker from {}", url);
        
        let matrix_client = self.bridge.get_matrix_client();
        let sticker_data = match matrix_client.download_media(url).await {
            Ok(data) => data,
            Err(e) => {
                warn!("Failed to download sticker: {}", e);
                return Ok(());
            }
        };
        
        match client.send_emoji_message(&portal.key.uid, &sticker_data).await {
            Ok(msg_id) => {
                info!("Sent sticker message to WeChat: {}", msg_id);
                if let Some(event_id) = &event.event_id {
                    if let Some(room_id) = &event.room_id {
                        let msg = crate::database::Message {
                            chat_uid: portal.key.uid.clone(),
                            chat_receiver: portal.key.receiver.clone(),
                            msg_id,
                            mxid: event_id.clone(),
                            sender: event.sender.clone().unwrap_or_default(),
                            timestamp: event.origin_server_ts.unwrap_or(0),
                            sent: true,
                            error: None,
                            msg_type: "m.sticker".to_string(),
                        };
                        self.bridge.db.insert_message(&msg).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to send sticker message to WeChat: {}", e);
            }
        }

        Ok(())
    }

    async fn get_reply_target(&self, event: &RoomEvent) -> anyhow::Result<Option<String>> {
        let relates_to = event.content.as_ref()
            .and_then(|c| c.get("m.relates_to"));
        
        if let Some(relates_to) = relates_to {
            let in_reply_to = relates_to.get("m.in_reply_to")
                .and_then(|r| r.get("event_id"))
                .and_then(|e| e.as_str());
            
            if let Some(event_id) = in_reply_to {
                let Some(room_id) = &event.room_id else {
                    return Ok(None);
                };
                
                if let Some(msg) = self.bridge.db.get_message_by_mxid(event_id).await? {
                    return Ok(Some(msg.msg_id));
                }
            }
        }

        Ok(None)
    }

    async fn get_portal_by_mxid(&self, mxid: &str) -> anyhow::Result<Option<Arc<crate::bridge::portal::BridgePortal>>> {
        let portal = self.bridge.db.get_portal_by_mxid(mxid).await?;
        if let Some(p) = portal {
            Ok(Some(Arc::new(crate::bridge::portal::BridgePortal::from_db(p, self.bridge.db.clone()))))
        } else {
            Ok(None)
        }
    }

    async fn get_user_by_mxid(&self, mxid: &str) -> anyhow::Result<Option<Arc<crate::bridge::user::BridgeUser>>> {
        let user = self.bridge.get_user_by_mxid(mxid).await?;
        Ok(Some(user))
    }

    async fn get_or_create_user_by_mxid(&self, mxid: &str) -> anyhow::Result<Arc<crate::bridge::user::BridgeUser>> {
        let user = self.bridge.get_user_by_mxid(mxid).await?;
        Ok(user)
    }
}

pub struct MatrixEventProcessor {
    handler: Arc<dyn MatrixEventHandlerTrait + Send + Sync>,
    event_age_limit: Duration,
}

#[async_trait::async_trait]
pub trait MatrixEventHandlerTrait {
    async fn handle_event(&self, event: &RoomEvent) -> anyhow::Result<()>;
}

impl MatrixEventProcessor {
    pub fn new(handler: Arc<dyn MatrixEventHandlerTrait + Send + Sync>) -> Self {
        Self {
            handler,
            event_age_limit: Duration::from_secs(300),
        }
    }

    pub fn with_age_limit(mut self, limit_ms: u64) -> Self {
        self.event_age_limit = Duration::from_millis(limit_ms);
        self
    }

    pub async fn process_event(&self, event: RoomEvent) -> anyhow::Result<()> {
        if self.is_event_too_old(&event) {
            debug!("Dropping old event: {:?}", event.event_id);
            return Ok(());
        }

        self.handler.handle_event(&event).await
    }

    fn is_event_too_old(&self, event: &RoomEvent) -> bool {
        let Some(ts) = event.origin_server_ts else {
            return false;
        };
        
        let event_time = UNIX_EPOCH + Duration::from_millis(ts as u64);
        let now = SystemTime::now();
        
        match now.duration_since(event_time) {
            Ok(age) => age > self.event_age_limit,
            Err(_) => false,
        }
    }
}

#[async_trait::async_trait]
impl MatrixEventHandlerTrait for MatrixEventHandler {
    async fn handle_event(&self, event: &RoomEvent) -> anyhow::Result<()> {
        self.handle_event(event).await
    }
}
