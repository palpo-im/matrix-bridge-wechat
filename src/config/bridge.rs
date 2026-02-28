use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

pub const NAME_QUALITY_NAME: i8 = 2;
pub const NAME_QUALITY_UIN: i8 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct DoublePuppetConfig {
    #[serde(default)]
    pub server_map: HashMap<String, String>,
    #[serde(default)]
    pub allow_discovery: bool,
    #[serde(default)]
    pub login_shared_secret_map: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EncryptionConfig {
    #[serde(default)]
    pub allow: bool,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub appservice: bool,
    #[serde(default)]
    pub require: bool,
    #[serde(default)]
    pub allow_key_sharing: bool,
    #[serde(default)]
    pub plaintext_mentions: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MessageHandlingTimeout {
    pub error_after: Option<String>,
    pub deadline: Option<String>,
}

impl MessageHandlingTimeout {
    pub fn error_after_duration(&self) -> Option<Duration> {
        self.error_after
            .as_ref()
            .and_then(|s| parse_duration(s).ok())
    }

    pub fn deadline_duration(&self) -> Option<Duration> {
        self.deadline.as_ref().and_then(|s| parse_duration(s).ok())
    }
}

fn parse_duration(s: &str) -> Result<Duration, anyhow::Error> {
    let s = s.trim();

    if s.ends_with('s') {
        let secs: u64 = s.trim_end_matches('s').parse()?;
        Ok(Duration::from_secs(secs))
    } else if s.ends_with('m') {
        let mins: u64 = s.trim_end_matches('m').parse()?;
        Ok(Duration::from_secs(mins * 60))
    } else if s.ends_with('h') {
        let hours: u64 = s.trim_end_matches('h').parse()?;
        Ok(Duration::from_secs(hours * 3600))
    } else {
        let secs: u64 = s.parse()?;
        Ok(Duration::from_secs(secs))
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ManagementRoomTexts {
    #[serde(default = "default_welcome")]
    pub welcome: String,
    #[serde(default = "default_welcome_connected")]
    pub welcome_connected: String,
    #[serde(default = "default_welcome_unconnected")]
    pub welcome_unconnected: String,
    #[serde(default)]
    pub additional_help: String,
}

fn default_welcome() -> String {
    "Hello, I'm a WeChat bridge bot.".to_string()
}

fn default_welcome_connected() -> String {
    "Use `help` for help.".to_string()
}

fn default_welcome_unconnected() -> String {
    "Use `help` for help or `login` to log in.".to_string()
}

impl Default for ManagementRoomTexts {
    fn default() -> Self {
        Self {
            welcome: default_welcome(),
            welcome_connected: default_welcome_connected(),
            welcome_unconnected: default_welcome_unconnected(),
            additional_help: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    User,
    Admin,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BridgeConfig {
    #[serde(default)]
    pub hs_proxy: Option<String>,

    pub username_template: String,
    pub displayname_template: String,
    pub listen_address: String,
    pub listen_secret: String,

    #[serde(default = "default_user_prefix")]
    pub user_prefix: String,

    #[serde(default)]
    pub personal_filtering_spaces: bool,

    #[serde(default)]
    pub message_status_events: bool,
    #[serde(default = "default_message_error_notices")]
    pub message_error_notices: bool,
    #[serde(default = "default_portal_message_buffer")]
    pub portal_message_buffer: usize,

    #[serde(default)]
    pub allow_redaction: bool,

    #[serde(default = "default_user_avatar_sync")]
    pub user_avatar_sync: bool,

    #[serde(default)]
    pub sync_direct_chat_list: bool,
    #[serde(default)]
    pub default_bridge_presence: bool,
    #[serde(default)]
    pub send_presence_on_typing: bool,

    #[serde(default)]
    pub double_puppet_server_map: HashMap<String, String>,
    #[serde(default)]
    pub double_puppet_allow_discovery: bool,
    #[serde(default)]
    pub login_shared_secret_map: HashMap<String, String>,

    #[serde(default = "default_private_chat_portal_meta")]
    pub private_chat_portal_meta: String,
    #[serde(default)]
    pub parallel_member_sync: bool,
    #[serde(default)]
    pub resend_bridge_info: bool,
    #[serde(default)]
    pub mute_bridging: bool,
    #[serde(default)]
    pub allow_user_invite: bool,
    #[serde(default = "default_federate_rooms")]
    pub federate_rooms: bool,

    #[serde(default)]
    pub message_handling_timeout: MessageHandlingTimeout,

    #[serde(default)]
    pub disable_bridge_alerts: bool,

    #[serde(default = "default_command_prefix")]
    pub command_prefix: String,

    #[serde(default)]
    pub management_room_text: ManagementRoomTexts,

    #[serde(default)]
    pub encryption: EncryptionConfig,

    pub permissions: HashMap<String, PermissionLevel>,
}

fn default_message_error_notices() -> bool {
    true
}

fn default_portal_message_buffer() -> usize {
    128
}

fn default_user_avatar_sync() -> bool {
    true
}

fn default_private_chat_portal_meta() -> String {
    "default".to_string()
}

fn default_federate_rooms() -> bool {
    true
}

fn default_command_prefix() -> String {
    "!wechat".to_string()
}

fn default_user_prefix() -> String {
    "wechat_".to_string()
}

impl BridgeConfig {
    pub fn get_permission(&self, mxid: &str) -> PermissionLevel {
        if let Some(level) = self.permissions.get(mxid) {
            return *level;
        }

        if let Some(pos) = mxid.find(':') {
            let domain = &mxid[(pos + 1)..];
            if let Some(level) = self.permissions.get(domain) {
                return *level;
            }
        }

        self.permissions
            .get("*")
            .copied()
            .unwrap_or(PermissionLevel::User)
    }

    pub fn format_displayname(&self, uin: &str, name: &str, remark: &str) -> (String, i8) {
        let displayname = if !name.is_empty() {
            name.to_string()
        } else if !remark.is_empty() {
            remark.to_string()
        } else {
            uin.to_string()
        };

        let result = self
            .displayname_template
            .replace("{{.Name}}", name)
            .replace("{{.Uin}}", uin)
            .replace("{{.Remark}}", remark);

        let quality = if !name.is_empty() {
            NAME_QUALITY_NAME
        } else {
            NAME_QUALITY_UIN
        };

        (result, quality)
    }
}
