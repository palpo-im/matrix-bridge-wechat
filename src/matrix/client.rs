use std::sync::Arc;

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tracing::debug;

use crate::matrix::types::*;

#[derive(Clone)]
pub struct MatrixClient {
    homeserver: String,
    access_token: String,
    client: Client,
    user_id: Option<String>,
}

impl MatrixClient {
    pub fn new(homeserver: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            homeserver: homeserver.into(),
            access_token: access_token.into(),
            client: Client::new(),
            user_id: None,
        }
    }

    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.homeserver.trim_end_matches('/'), path)
    }

    async fn request<T: DeserializeOwned>(&self, method: reqwest::Method, path: &str, body: Option<&serde_json::Value>) -> Result<T> {
        let url = self.url(path);
        let mut req = self.client
            .request(method.clone(), &url)
            .bearer_auth(&self.access_token);
        
        if let Some(json) = body {
            req = req.json(json);
        }
        
        debug!("Matrix API request: {:?} {}", method, url);
        
        let resp = req.send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        
        debug!("Matrix API response: {} - {}", status, text);
        
        if !status.is_success() {
            if let Ok(error) = serde_json::from_str::<ErrorResponse>(&text) {
                return Err(anyhow!("Matrix error: {} - {}", error.errcode, error.error));
            }
            return Err(anyhow!("Matrix request failed: {} - {}", status, text));
        }
        
        if text.is_empty() || text == "{}" {
            return Ok(serde_json::from_str("{}")?);
        }
        
        serde_json::from_str(&text).map_err(|e| anyhow!("Failed to parse response: {} - {}", e, text))
    }

    pub async fn get_user_id(&self) -> Result<String> {
        let result: serde_json::Value = self.request(reqwest::Method::GET, "/_matrix/client/v3/account/whoami", None).await?;
        result.get("user_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No user_id in response"))
    }

    pub async fn send_message(&self, room_id: &str, event_type: &str, content: &serde_json::Value, txn_id: Option<&str>) -> Result<String> {
        let default_txn = chrono::Utc::now().timestamp_millis().to_string();
        let txn_id = txn_id.unwrap_or(&default_txn);
        let path = format!(
            "/_matrix/client/v3/rooms/{}/send/{}/{}?access_token={}",
            room_id, event_type, txn_id, self.access_token
        );
        let result: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(content)).await?;
        result.get("event_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No event_id in response"))
    }

    pub async fn send_text(&self, room_id: &str, text: impl Into<String>) -> Result<String> {
        let content = EventContent::text(text.into());
        let content = serde_json::to_value(&content)?;
        self.send_message(room_id, "m.room.message", &content, None).await
    }

    pub async fn send_text_html(&self, room_id: &str, plain: impl Into<String>, html: impl Into<String>) -> Result<String> {
        let content = EventContent::text_html(plain, html);
        let content = serde_json::to_value(&content)?;
        self.send_message(room_id, "m.room.message", &content, None).await
    }

    pub async fn send_notice(&self, room_id: &str, text: impl Into<String>) -> Result<String> {
        let content = EventContent::notice(text);
        let content = serde_json::to_value(&content)?;
        self.send_message(room_id, "m.room.message", &content, None).await
    }

    pub async fn send_emote(&self, room_id: &str, text: impl Into<String>) -> Result<String> {
        let content = serde_json::json!({
            "msgtype": "m.emote",
            "body": text.into()
        });
        self.send_message(room_id, "m.room.message", &content, None).await
    }

    pub async fn redact(&self, room_id: &str, event_id: &str, reason: Option<&str>) -> Result<String> {
        let txn_id = chrono::Utc::now().timestamp_millis().to_string();
        let path = format!(
            "/_matrix/client/v3/rooms/{}/redact/{}/{}?access_token={}",
            room_id, event_id, txn_id, self.access_token
        );
        let body = if let Some(r) = reason {
            Some(serde_json::json!({ "reason": r }))
        } else {
            None
        };
        let result: serde_json::Value = self.request(reqwest::Method::PUT, &path, body.as_ref()).await?;
        result.get("event_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No event_id in response"))
    }

    pub async fn send_state(&self, room_id: &str, event_type: &str, state_key: &str, content: &serde_json::Value) -> Result<String> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/state/{}/{}?access_token={}",
            room_id, event_type, state_key, self.access_token
        );
        let result: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(content)).await?;
        result.get("event_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No event_id in response"))
    }

    pub async fn set_room_name(&self, room_id: &str, name: &str) -> Result<String> {
        let content = RoomNameContent { name: name.to_string() };
        self.send_state(room_id, "m.room.name", "", &serde_json::to_value(&content)?).await
    }

    pub async fn set_room_topic(&self, room_id: &str, topic: &str) -> Result<String> {
        let content = RoomTopicContent { topic: topic.to_string() };
        self.send_state(room_id, "m.room.topic", "", &serde_json::to_value(&content)?).await
    }

    pub async fn set_room_avatar(&self, room_id: &str, url: &str) -> Result<String> {
        let content = RoomAvatarContent { url: url.to_string() };
        self.send_state(room_id, "m.room.avatar", "", &serde_json::to_value(&content)?).await
    }

    pub async fn join_room(&self, room_id: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/join/{}?access_token={}", room_id, self.access_token);
        let _: serde_json::Value = self.request(reqwest::Method::POST, &path, Some(&serde_json::json!({}))).await?;
        Ok(())
    }

    pub async fn leave_room(&self, room_id: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/rooms/{}/leave?access_token={}", room_id, self.access_token);
        let _: serde_json::Value = self.request(reqwest::Method::POST, &path, Some(&serde_json::json!({}))).await?;
        Ok(())
    }

    pub async fn invite_user(&self, room_id: &str, user_id: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/rooms/{}/invite?access_token={}", room_id, self.access_token);
        let body = serde_json::json!({ "user_id": user_id });
        let _: serde_json::Value = self.request(reqwest::Method::POST, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn kick_user(&self, room_id: &str, user_id: &str, reason: Option<&str>) -> Result<()> {
        let path = format!("/_matrix/client/v3/rooms/{}/kick?access_token={}", room_id, self.access_token);
        let mut body = serde_json::json!({ "user_id": user_id });
        if let Some(r) = reason {
            body["reason"] = r.into();
        }
        let _: serde_json::Value = self.request(reqwest::Method::POST, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn set_membership(&self, room_id: &str, user_id: &str, content: &RoomMemberContent) -> Result<String> {
        self.send_state(room_id, "m.room.member", user_id, &serde_json::to_value(content)?).await
    }

    pub async fn create_room(&self, request: &CreateRoomRequest) -> Result<String> {
        let path = format!("/_matrix/client/v3/createRoom?access_token={}", self.access_token);
        let result: CreateRoomResponse = self.request(reqwest::Method::POST, &path, Some(&serde_json::to_value(request)?)).await?;
        Ok(result.room_id)
    }

    pub async fn get_joined_members(&self, room_id: &str) -> Result<JoinedMembersResponse> {
        let path = format!("/_matrix/client/v3/rooms/{}/joined_members?access_token={}", room_id, self.access_token);
        self.request(reqwest::Method::GET, &path, None).await
    }

    pub async fn get_room_state(&self, room_id: &str, event_type: &str, state_key: &str) -> Result<serde_json::Value> {
        let path = format!(
            "/_matrix/client/v3/rooms/{}/state/{}/{}?access_token={}",
            room_id, event_type, state_key, self.access_token
        );
        self.request(reqwest::Method::GET, &path, None).await
    }

    pub async fn set_displayname(&self, user_id: &str, displayname: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/profile/{}/displayname?access_token={}", user_id, self.access_token);
        let body = serde_json::json!({ "displayname": displayname });
        let _: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn set_avatar_url(&self, user_id: &str, avatar_url: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/profile/{}/avatar_url?access_token={}", user_id, self.access_token);
        let body = serde_json::json!({ "avatar_url": avatar_url });
        let _: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn get_profile(&self, user_id: &str) -> Result<ProfileResponse> {
        let path = format!("/_matrix/client/v3/profile/{}?access_token={}", user_id, self.access_token);
        self.request(reqwest::Method::GET, &path, None).await
    }

    pub async fn upload_media(&self, data: &[u8], content_type: &str, filename: &str) -> Result<String> {
        let path = format!(
            "/_matrix/media/v3/upload?access_token={}&filename={}",
            self.access_token, urlencoding::encode(filename)
        );
        let url = self.url(&path);
        
        let resp = self.client
            .post(&url)
            .header("Content-Type", content_type)
            .body(data.to_vec())
            .send()
            .await?;
        
        let status = resp.status();
        let text = resp.text().await?;
        
        if !status.is_success() {
            return Err(anyhow!("Media upload failed: {} - {}", status, text));
        }
        
        let result: serde_json::Value = serde_json::from_str(&text)?;
        result.get("content_uri")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No content_uri in response"))
    }

    pub async fn download_media(&self, mxc_url: &str) -> Result<Vec<u8>> {
        let mxc_url = mxc_url.strip_prefix("mxc://").unwrap_or(mxc_url);
        let parts: Vec<&str> = mxc_url.split('/').collect();
        if parts.len() < 2 {
            return Err(anyhow!("Invalid mxc URL: {}", mxc_url));
        }
        let server = parts[0];
        let media_id = parts[1..].join("/");
        
        let path = format!(
            "/_matrix/media/v3/download/{}/{}?access_token={}",
            server, media_id, self.access_token
        );
        let url = self.url(&path);
        
        let resp = self.client
            .get(&url)
            .send()
            .await?;
        
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            return Err(anyhow!("Media download failed: {} - {}", status, text));
        }
        
        let data = resp.bytes().await?;
        Ok(data.to_vec())
    }

    pub async fn get_room_alias(&self, alias: &str) -> Result<String> {
        let path = format!("/_matrix/client/v3/directory/room/{}?access_token={}", urlencoding::encode(alias), self.access_token);
        let result: serde_json::Value = self.request(reqwest::Method::GET, &path, None).await?;
        result.get("room_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow!("No room_id in response"))
    }

    pub async fn set_room_alias(&self, room_id: &str, alias: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/directory/room/{}?access_token={}", urlencoding::encode(alias), self.access_token);
        let body = serde_json::json!({ "room_id": room_id });
        let _: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn sync(&self, since: Option<&str>, timeout_ms: u64) -> Result<serde_json::Value> {
        let mut path = format!("/_matrix/client/v3/sync?access_token={}", self.access_token);
        if let Some(s) = since {
            path.push_str(&format!("&since={}", s));
        }
        path.push_str(&format!("&timeout={}", timeout_ms));
        self.request(reqwest::Method::GET, &path, None).await
    }

    pub async fn get_event(&self, room_id: &str, event_id: &str) -> Result<RoomEvent> {
        let path = format!("/_matrix/client/v3/rooms/{}/event/{}?access_token={}", room_id, event_id, self.access_token);
        self.request(reqwest::Method::GET, &path, None).await
    }

    pub async fn set_presence(&self, presence: &str, status_msg: Option<&str>) -> Result<()> {
        let path = format!("/_matrix/client/v3/presence/{}/status?access_token={}", 
            self.user_id.as_deref().unwrap_or(""), self.access_token);
        let mut body = serde_json::json!({ "presence": presence });
        if let Some(msg) = status_msg {
            body["status_msg"] = msg.into();
        }
        let _: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn set_typing(&self, room_id: &str, typing: bool, timeout: Option<u32>) -> Result<()> {
        let user_id = self.user_id.as_deref().unwrap_or("");
        let path = format!("/_matrix/client/v3/rooms/{}/typing/{}?access_token={}", 
            room_id, user_id, self.access_token);
        let timeout_val = timeout.unwrap_or(if typing { 30000 } else { 0 });
        let body = serde_json::json!({
            "typing": typing,
            "timeout": timeout_val
        });
        let _: serde_json::Value = self.request(reqwest::Method::PUT, &path, Some(&body)).await?;
        Ok(())
    }

    pub async fn send_read_receipt(&self, room_id: &str, event_id: &str) -> Result<()> {
        let path = format!("/_matrix/client/v3/rooms/{}/receipt/m.read/{}?access_token={}", 
            room_id, event_id, self.access_token);
        let _: serde_json::Value = self.request(reqwest::Method::POST, &path, Some(&serde_json::json!({}))).await?;
        Ok(())
    }
}

pub struct MatrixClientBuilder {
    homeserver: String,
    access_token: String,
    user_id: Option<String>,
}

impl MatrixClientBuilder {
    pub fn new(homeserver: impl Into<String>, access_token: impl Into<String>) -> Self {
        Self {
            homeserver: homeserver.into(),
            access_token: access_token.into(),
            user_id: None,
        }
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn build(self) -> MatrixClient {
        let mut client = MatrixClient::new(self.homeserver, self.access_token);
        if let Some(user_id) = self.user_id {
            client = client.with_user_id(user_id);
        }
        client
    }
}

pub fn ensure_bot_client(homeserver: &str, token: &str, bot_mxid: &str) -> Arc<MatrixClient> {
    Arc::new(MatrixClient::new(homeserver.to_string(), token.to_string()).with_user_id(bot_mxid))
}
