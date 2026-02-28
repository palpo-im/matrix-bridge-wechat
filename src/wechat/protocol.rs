use super::types::{Chat, GroupInfo, ReplyInfo, User, UserInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Request,
    Response,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request => write!(f, "request"),
            Self::Response => write!(f, "response"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestType {
    Event,
    Connect,
    Disconnect,
    LoginQr,
    IsLogin,
    GetSelf,
    GetUserInfo,
    GetGroupInfo,
    GetGroupMembers,
    GetGroupMemberNickname,
    GetFriendList,
    GetGroupList,
    SendText,
    SendImage,
    SendVideo,
    SendAudio,
    SendFile,
    SendEmoji,
    RevokeMsg,
    DownloadImage,
    DownloadVideo,
    DownloadAudio,
    DownloadFile,
    SetNickname,
    SetAvatar,
    GetQRCode,
    AcceptFriend,
    CreateGroup,
    SetGroupName,
    InviteGroupMember,
    RemoveGroupMember,
    QuitGroup,
    RefreshContacts,
    SyncMessages,
}

impl std::fmt::Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event => write!(f, "event"),
            Self::Connect => write!(f, "connect"),
            Self::Disconnect => write!(f, "disconnect"),
            Self::LoginQr => write!(f, "login_qr"),
            Self::IsLogin => write!(f, "is_login"),
            Self::GetSelf => write!(f, "get_self"),
            Self::GetUserInfo => write!(f, "get_user_info"),
            Self::GetGroupInfo => write!(f, "get_group_info"),
            Self::GetGroupMembers => write!(f, "get_group_members"),
            Self::GetGroupMemberNickname => write!(f, "get_group_member_nickname"),
            Self::GetFriendList => write!(f, "get_friend_list"),
            Self::GetGroupList => write!(f, "get_group_list"),
            Self::SendText => write!(f, "send_text"),
            Self::SendImage => write!(f, "send_image"),
            Self::SendVideo => write!(f, "send_video"),
            Self::SendAudio => write!(f, "send_audio"),
            Self::SendFile => write!(f, "send_file"),
            Self::SendEmoji => write!(f, "send_emoji"),
            Self::RevokeMsg => write!(f, "revoke_msg"),
            Self::DownloadImage => write!(f, "download_image"),
            Self::DownloadVideo => write!(f, "download_video"),
            Self::DownloadAudio => write!(f, "download_audio"),
            Self::DownloadFile => write!(f, "download_file"),
            Self::SetNickname => write!(f, "set_nickname"),
            Self::SetAvatar => write!(f, "set_avatar"),
            Self::GetQRCode => write!(f, "get_qrcode"),
            Self::AcceptFriend => write!(f, "accept_friend"),
            Self::CreateGroup => write!(f, "create_group"),
            Self::SetGroupName => write!(f, "set_group_name"),
            Self::InviteGroupMember => write!(f, "invite_group_member"),
            Self::RemoveGroupMember => write!(f, "remove_group_member"),
            Self::QuitGroup => write!(f, "quit_group"),
            Self::RefreshContacts => write!(f, "refresh_contacts"),
            Self::SyncMessages => write!(f, "sync_messages"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Event,
    Connect,
    Disconnect,
    LoginQr,
    IsLogin,
    GetSelf,
    GetUserInfo,
    GetGroupInfo,
    GetGroupMembers,
    GetGroupMemberNickname,
    GetFriendList,
    GetGroupList,
    SendText,
    SendImage,
    SendVideo,
    SendAudio,
    SendFile,
    SendEmoji,
    RevokeMsg,
    DownloadImage,
    DownloadVideo,
    DownloadAudio,
    DownloadFile,
    SetNickname,
    SetAvatar,
    GetQRCode,
    AcceptFriend,
    CreateGroup,
    SetGroupName,
    InviteGroupMember,
    RemoveGroupMember,
    QuitGroup,
    RefreshContacts,
    SyncMessages,
}

impl std::fmt::Display for ResponseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Event => write!(f, "event"),
            Self::Connect => write!(f, "connect"),
            Self::Disconnect => write!(f, "disconnect"),
            Self::LoginQr => write!(f, "login_qr"),
            Self::IsLogin => write!(f, "is_login"),
            Self::GetSelf => write!(f, "get_self"),
            Self::GetUserInfo => write!(f, "get_user_info"),
            Self::GetGroupInfo => write!(f, "get_group_info"),
            Self::GetGroupMembers => write!(f, "get_group_members"),
            Self::GetGroupMemberNickname => write!(f, "get_group_member_nickname"),
            Self::GetFriendList => write!(f, "get_friend_list"),
            Self::GetGroupList => write!(f, "get_group_list"),
            Self::SendText => write!(f, "send_text"),
            Self::SendImage => write!(f, "send_image"),
            Self::SendVideo => write!(f, "send_video"),
            Self::SendAudio => write!(f, "send_audio"),
            Self::SendFile => write!(f, "send_file"),
            Self::SendEmoji => write!(f, "send_emoji"),
            Self::RevokeMsg => write!(f, "revoke_msg"),
            Self::DownloadImage => write!(f, "download_image"),
            Self::DownloadVideo => write!(f, "download_video"),
            Self::DownloadAudio => write!(f, "download_audio"),
            Self::DownloadFile => write!(f, "download_file"),
            Self::SetNickname => write!(f, "set_nickname"),
            Self::SetAvatar => write!(f, "set_avatar"),
            Self::GetQRCode => write!(f, "get_qrcode"),
            Self::AcceptFriend => write!(f, "accept_friend"),
            Self::CreateGroup => write!(f, "create_group"),
            Self::SetGroupName => write!(f, "set_group_name"),
            Self::InviteGroupMember => write!(f, "invite_group_member"),
            Self::RemoveGroupMember => write!(f, "remove_group_member"),
            Self::QuitGroup => write!(f, "quit_group"),
            Self::RefreshContacts => write!(f, "refresh_contacts"),
            Self::SyncMessages => write!(f, "sync_messages"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatType {
    Private,
    Group,
}

impl std::fmt::Display for ChatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Private => write!(f, "private"),
            Self::Group => write!(f, "group"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Text,
    Photo,
    Sticker,
    Audio,
    Video,
    File,
    Location,
    Notice,
    App,
    Revoke,
    Voip,
    System,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Photo => write!(f, "photo"),
            Self::Sticker => write!(f, "sticker"),
            Self::Audio => write!(f, "audio"),
            Self::Video => write!(f, "video"),
            Self::File => write!(f, "file"),
            Self::Location => write!(f, "location"),
            Self::Notice => write!(f, "notice"),
            Self::App => write!(f, "app"),
            Self::Revoke => write!(f, "revoke"),
            Self::Voip => write!(f, "voip"),
            Self::System => write!(f, "system"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    #[serde(skip)]
    pub http_status: u16,
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ErrorResponse {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub mxid: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Message {
    pub fn request(id: i64, mxid: &str, request: &Request) -> Self {
        Self {
            id,
            mxid: mxid.to_string(),
            msg_type: MessageType::Request,
            data: serde_json::to_value(request).ok(),
        }
    }

    pub fn as_request(&self) -> Option<Request> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_response(&self) -> Option<Response> {
        serde_json::from_value(self.data.clone()?).ok()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    #[serde(rename = "type")]
    pub request_type: RequestType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    #[serde(rename = "type")]
    pub response_type: ResponseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Response {
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.data.as_ref()?.as_bool()
    }

    pub fn as_user_info(&self) -> Option<UserInfo> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_group_info(&self) -> Option<GroupInfo> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_user_list(&self) -> Option<Vec<UserInfo>> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_group_list(&self) -> Option<Vec<GroupInfo>> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_string_list(&self) -> Option<Vec<String>> {
        serde_json::from_value(self.data.clone()?).ok()
    }

    pub fn as_string(&self) -> Option<String> {
        self.data.as_ref()?.as_str().map(|s| s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    pub timestamp: i64,
    pub from: User,
    pub chat: Chat,
    #[serde(rename = "type")]
    pub event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mentions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<ReplyInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
