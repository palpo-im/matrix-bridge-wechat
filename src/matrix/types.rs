use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin_server_ts: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsigned: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacts: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToDeviceEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub events: Vec<RoomEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EphemeralData {
    #[serde(default)]
    pub events: Vec<EphemeralEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToDeviceData {
    #[serde(default)]
    pub events: Vec<ToDeviceEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContent {
    pub msgtype: String,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted_body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub info: Option<serde_json::Value>,
}

impl EventContent {
    pub fn text(body: impl Into<String>) -> Self {
        Self {
            msgtype: "m.text".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: None,
            info: None,
        }
    }

    pub fn text_html(body: impl Into<String>, html: impl Into<String>) -> Self {
        Self {
            msgtype: "m.text".to_string(),
            body: body.into(),
            formatted_body: Some(html.into()),
            format: Some("org.matrix.custom.html".to_string()),
            url: None,
            info: None,
        }
    }

    pub fn notice(body: impl Into<String>) -> Self {
        Self {
            msgtype: "m.notice".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: None,
            info: None,
        }
    }

    pub fn image(body: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            msgtype: "m.image".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: Some(url.into()),
            info: None,
        }
    }

    pub fn file(body: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            msgtype: "m.file".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: Some(url.into()),
            info: None,
        }
    }

    pub fn video(body: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            msgtype: "m.video".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: Some(url.into()),
            info: None,
        }
    }

    pub fn audio(body: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            msgtype: "m.audio".to_string(),
            body: body.into(),
            formatted_body: None,
            format: None,
            url: Some(url.into()),
            info: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomMemberContent {
    pub membership: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub displayname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

impl RoomMemberContent {
    pub fn join() -> Self {
        Self {
            membership: "join".to_string(),
            displayname: None,
            avatar_url: None,
        }
    }

    pub fn join_with(displayname: impl Into<String>, avatar_url: impl Into<String>) -> Self {
        Self {
            membership: "join".to_string(),
            displayname: Some(displayname.into()),
            avatar_url: Some(avatar_url.into()),
        }
    }

    pub fn leave() -> Self {
        Self {
            membership: "leave".to_string(),
            displayname: None,
            avatar_url: None,
        }
    }

    pub fn invite() -> Self {
        Self {
            membership: "invite".to_string(),
            displayname: None,
            avatar_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomCreateContent {
    pub creator: String,
    #[serde(rename = "room_version", default = "default_room_version")]
    pub room_version: String,
}

fn default_room_version() -> String {
    "9".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomNameContent {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomTopicContent {
    pub topic: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomAvatarContent {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerLevelsContent {
    #[serde(default = "default_power_level")]
    pub users_default: i64,
    #[serde(default = "default_power_level")]
    pub events_default: i64,
    #[serde(default = "default_power_level")]
    pub state_default: i64,
    #[serde(default = "default_invite_power")]
    pub invite: i64,
    #[serde(default = "default_kick_power")]
    pub kick: i64,
    #[serde(default = "default_ban_power")]
    pub ban: i64,
    #[serde(default = "default_redact_power")]
    pub redact: i64,
    #[serde(default)]
    pub users: std::collections::HashMap<String, i64>,
    #[serde(default)]
    pub events: std::collections::HashMap<String, i64>,
}

fn default_power_level() -> i64 {
    0
}
fn default_invite_power() -> i64 {
    50
}
fn default_kick_power() -> i64 {
    50
}
fn default_ban_power() -> i64 {
    50
}
fn default_redact_power() -> i64 {
    50
}

impl Default for PowerLevelsContent {
    fn default() -> Self {
        Self {
            users_default: 0,
            events_default: 0,
            state_default: 50,
            invite: 50,
            kick: 50,
            ban: 50,
            redact: 50,
            users: std::collections::HashMap::new(),
            events: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_alias_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(default)]
    pub invite: Vec<String>,
    #[serde(default)]
    pub invite_3pid: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub room_version: Option<String>,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub is_direct: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_state: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub power_level_content_override: Option<PowerLevelsContent>,
}

impl CreateRoomRequest {
    pub fn private(name: impl Into<String>) -> Self {
        Self {
            visibility: Some("private".to_string()),
            room_alias_name: None,
            name: Some(name.into()),
            topic: None,
            invite: Vec::new(),
            invite_3pid: Vec::new(),
            room_version: None,
            preset: Some("private_chat".to_string()),
            is_direct: true,
            initial_state: None,
            power_level_content_override: None,
        }
    }

    pub fn public(name: impl Into<String>) -> Self {
        Self {
            visibility: Some("public".to_string()),
            room_alias_name: None,
            name: Some(name.into()),
            topic: None,
            invite: Vec::new(),
            invite_3pid: Vec::new(),
            room_version: None,
            preset: Some("public_chat".to_string()),
            is_direct: false,
            initial_state: None,
            power_level_content_override: None,
        }
    }

    pub fn with_invite(mut self, user_id: impl Into<String>) -> Self {
        self.invite.push(user_id.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoomResponse {
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinedMembersResponse {
    pub joined: std::collections::HashMap<String, JoinedMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinedMember {
    #[serde(rename = "display_name")]
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub displayname: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub errcode: String,
    pub error: String,
}
