use std::sync::Arc;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::{WechatService, Request, RequestType, UserInfo, GroupInfo};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
}

#[derive(Clone)]
pub struct WechatClient {
    mxid: String,
    service: Arc<WechatService>,
}

impl WechatClient {
    pub fn new(mxid: String, service: Arc<WechatService>) -> Self {
        Self { mxid, service }
    }

    pub fn mxid(&self) -> &str {
        &self.mxid
    }

    pub async fn connect(&self) -> Result<()> {
        self.service.request(&self.mxid, &Request {
            request_type: RequestType::Connect,
            data: None,
        }).await?;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.service.request(&self.mxid, &Request {
            request_type: RequestType::Disconnect,
            data: None,
        }).await?;
        Ok(())
    }

    pub async fn is_logged_in(&self) -> Result<bool> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::IsLogin,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(response.data.as_ref().and_then(|d| d.as_bool()).unwrap_or(false))
    }

    pub async fn get_self(&self) -> Result<UserInfo> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetSelf,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_user_info(&self, wxid: &str) -> Result<UserInfo> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetUserInfo,
            data: Some(serde_json::json!([wxid])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_friend_list(&self) -> Result<Vec<UserInfo>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetFriendList,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_group_list(&self) -> Result<Vec<GroupInfo>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetGroupList,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_group_info(&self, group_id: &str) -> Result<GroupInfo> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetGroupInfo,
            data: Some(serde_json::json!([group_id])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMember>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetGroupMembers,
            data: Some(serde_json::json!([group_id])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn get_group_member_nickname(&self, group_id: &str, member_id: &str) -> Result<String> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetGroupMemberNickname,
            data: Some(serde_json::json!([group_id, member_id])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            return serde_json::from_value(data.clone()).map_err(|e| anyhow!("invalid response: {}", e));
        }
        
        Err(anyhow!("invalid response"))
    }

    pub async fn send_text_message(&self, chat_id: &str, text: &str, reply_to: Option<&str>) -> Result<String> {
        let data = if let Some(reply) = reply_to {
            serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "reply_to": reply,
            })
        } else {
            serde_json::json!({
                "chat_id": chat_id,
                "text": text,
            })
        };
        
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SendText,
            data: Some(data),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(msg_id) = data.get("msg_id").and_then(|v| v.as_str()) {
                return Ok(msg_id.to_string());
            }
        }
        
        Err(anyhow!("no msg_id in response"))
    }

    pub async fn send_image_message(&self, chat_id: &str, image_data: &[u8], reply_to: Option<&str>) -> Result<String> {
        let image_base64 = base64_encode(image_data);
        let data = if let Some(reply) = reply_to {
            serde_json::json!({
                "chat_id": chat_id,
                "image": image_base64,
                "reply_to": reply,
            })
        } else {
            serde_json::json!({
                "chat_id": chat_id,
                "image": image_base64,
            })
        };
        
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SendImage,
            data: Some(data),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(msg_id) = data.get("msg_id").and_then(|v| v.as_str()) {
                return Ok(msg_id.to_string());
            }
        }
        
        Err(anyhow!("no msg_id in response"))
    }

    pub async fn send_video_message(&self, chat_id: &str, video_data: &[u8], reply_to: Option<&str>) -> Result<String> {
        let video_base64 = base64_encode(video_data);
        let data = if let Some(reply) = reply_to {
            serde_json::json!({
                "chat_id": chat_id,
                "video": video_base64,
                "reply_to": reply,
            })
        } else {
            serde_json::json!({
                "chat_id": chat_id,
                "video": video_base64,
            })
        };
        
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SendVideo,
            data: Some(data),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(msg_id) = data.get("msg_id").and_then(|v| v.as_str()) {
                return Ok(msg_id.to_string());
            }
        }
        
        Err(anyhow!("no msg_id in response"))
    }

    pub async fn send_file_message(&self, chat_id: &str, file_data: &[u8], filename: &str, reply_to: Option<&str>) -> Result<String> {
        let file_base64 = base64_encode(file_data);
        let data = if let Some(reply) = reply_to {
            serde_json::json!({
                "chat_id": chat_id,
                "file": file_base64,
                "filename": filename,
                "reply_to": reply,
            })
        } else {
            serde_json::json!({
                "chat_id": chat_id,
                "file": file_base64,
                "filename": filename,
            })
        };
        
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SendFile,
            data: Some(data),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(msg_id) = data.get("msg_id").and_then(|v| v.as_str()) {
                return Ok(msg_id.to_string());
            }
        }
        
        Err(anyhow!("no msg_id in response"))
    }

    pub async fn send_emoji_message(&self, chat_id: &str, emoji_data: &[u8]) -> Result<String> {
        let emoji_base64 = base64_encode(emoji_data);
        let data = serde_json::json!({
            "chat_id": chat_id,
            "emoji": emoji_base64,
        });
        
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SendEmoji,
            data: Some(data),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(msg_id) = data.get("msg_id").and_then(|v| v.as_str()) {
                return Ok(msg_id.to_string());
            }
        }
        
        Err(anyhow!("no msg_id in response"))
    }

    pub async fn revoke_message(&self, chat_id: &str, msg_id: &str) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::RevokeMsg,
            data: Some(serde_json::json!([chat_id, msg_id])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn download_image(&self, xml: &str) -> Result<Vec<u8>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::DownloadImage,
            data: Some(serde_json::json!([xml])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(image_base64) = data.get("image").and_then(|v| v.as_str()) {
                return base64_decode(image_base64);
            }
        }
        
        Err(anyhow!("no image in response"))
    }

    pub async fn download_video(&self, xml: &str) -> Result<Vec<u8>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::DownloadVideo,
            data: Some(serde_json::json!([xml])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(video_base64) = data.get("video").and_then(|v| v.as_str()) {
                return base64_decode(video_base64);
            }
        }
        
        Err(anyhow!("no video in response"))
    }

    pub async fn download_audio(&self, xml: &str) -> Result<Vec<u8>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::DownloadAudio,
            data: Some(serde_json::json!([xml])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(audio_base64) = data.get("audio").and_then(|v| v.as_str()) {
                return base64_decode(audio_base64);
            }
        }
        
        Err(anyhow!("no audio in response"))
    }

    pub async fn download_file(&self, xml: &str) -> Result<Vec<u8>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::DownloadFile,
            data: Some(serde_json::json!([xml])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(file_base64) = data.get("file").and_then(|v| v.as_str()) {
                return base64_decode(file_base64);
            }
        }
        
        Err(anyhow!("no file in response"))
    }

    pub async fn set_nickname(&self, nickname: &str) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SetNickname,
            data: Some(serde_json::json!([nickname])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn set_avatar(&self, avatar_data: &[u8]) -> Result<()> {
        let avatar_base64 = base64_encode(avatar_data);
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SetAvatar,
            data: Some(serde_json::json!([avatar_base64])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn get_qrcode(&self) -> Result<Vec<u8>> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::GetQRCode,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(qrcode_base64) = data.get("qrcode").and_then(|v| v.as_str()) {
                return base64_decode(qrcode_base64);
            }
        }
        
        Err(anyhow!("no qrcode in response"))
    }

    pub async fn accept_friend(&self, v3: &str) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::AcceptFriend,
            data: Some(serde_json::json!([v3])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn create_group(&self, user_ids: &[&str], name: &str) -> Result<String> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::CreateGroup,
            data: Some(serde_json::json!([user_ids, name])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        if let Some(data) = &response.data {
            if let Some(group_id) = data.get("group_id").and_then(|v| v.as_str()) {
                return Ok(group_id.to_string());
            }
        }
        
        Err(anyhow!("no group_id in response"))
    }

    pub async fn set_group_name(&self, group_id: &str, name: &str) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SetGroupName,
            data: Some(serde_json::json!([group_id, name])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn invite_group_member(&self, group_id: &str, user_ids: &[&str]) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::InviteGroupMember,
            data: Some(serde_json::json!([group_id, user_ids])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn remove_group_member(&self, group_id: &str, user_ids: &[&str]) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::RemoveGroupMember,
            data: Some(serde_json::json!([group_id, user_ids])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn quit_group(&self, group_id: &str) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::QuitGroup,
            data: Some(serde_json::json!([group_id])),
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn refresh_contacts(&self) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::RefreshContacts,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }

    pub async fn sync_messages(&self) -> Result<()> {
        let response = self.service.request(&self.mxid, &Request {
            request_type: RequestType::SyncMessages,
            data: None,
        }).await?;
        
        if let Some(error) = response.error {
            return Err(anyhow!("{}", error));
        }
        
        Ok(())
    }
}

fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.encode(data)
}

fn base64_decode(s: &str) -> Result<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    STANDARD.decode(s).map_err(|e| anyhow!("base64 decode error: {}", e))
}
