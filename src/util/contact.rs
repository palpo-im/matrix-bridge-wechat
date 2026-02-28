use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContactInfo {
    pub uin: String,
    pub name: String,
    pub remark: String,
}

impl ContactInfo {
    pub fn new(uin: impl Into<String>, name: impl Into<String>, remark: impl Into<String>) -> Self {
        Self {
            uin: uin.into(),
            name: name.into(),
            remark: remark.into(),
        }
    }
}
