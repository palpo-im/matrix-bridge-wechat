use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub const SEP_UID: &str = "\u{0001}";

pub const USER_TYPE: &str = "u";
pub const GROUP_TYPE: &str = "g";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct UID {
    pub uin: String,
    pub uid_type: String,
}

impl UID {
    pub fn new_user(uin: impl Into<String>) -> Self {
        Self {
            uin: uin.into(),
            uid_type: USER_TYPE.to_string(),
        }
    }

    pub fn new_group(uin: impl Into<String>) -> Self {
        Self {
            uin: uin.into(),
            uid_type: GROUP_TYPE.to_string(),
        }
    }

    pub fn is_user(&self) -> bool {
        self.uid_type == USER_TYPE
    }

    pub fn is_group(&self) -> bool {
        self.uid_type == GROUP_TYPE
    }

    pub fn is_empty(&self) -> bool {
        self.uid_type.is_empty()
    }
}

impl fmt::Display for UID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.uin, SEP_UID, self.uid_type)
    }
}

impl Serialize for UID {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for UID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromStr for UID {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(SEP_UID).collect();
        if parts.len() != 2 {
            anyhow::bail!("failed to parse UID: {}", s);
        }
        Ok(Self {
            uin: parts[0].to_string(),
            uid_type: parts[1].to_string(),
        })
    }
}
