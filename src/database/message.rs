use super::PortalKey;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::schema::message;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageErrorType {
    None,
    DecryptionFailed,
    MediaNotFound,
}

impl Default for MessageErrorType {
    fn default() -> Self {
        Self::None
    }
}

impl std::fmt::Display for MessageErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, ""),
            Self::DecryptionFailed => write!(f, "decryption_failed"),
            Self::MediaNotFound => write!(f, "media_not_found"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Unknown,
    Fake,
    Normal,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown => write!(f, ""),
            Self::Fake => write!(f, "fake"),
            Self::Normal => write!(f, "message"),
        }
    }
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = message)]
pub struct Message {
    pub chat_uid: String,
    pub chat_receiver: String,
    pub msg_id: String,
    pub mxid: String,
    pub sender: String,
    pub timestamp: i64,
    pub sent: bool,
    pub error: Option<String>,
    #[diesel(column_name = msg_type)]
    pub msg_type: String,
}

impl Message {
    pub fn key(&self) -> PortalKey {
        PortalKey::new(&self.chat_uid, &self.chat_receiver)
    }

    pub fn timestamp_time(&self) -> Option<DateTime<Utc>> {
        if self.timestamp > 0 {
            DateTime::from_timestamp(self.timestamp, 0)
        } else {
            None
        }
    }

    pub fn is_fake_mxid(&self) -> bool {
        self.mxid.starts_with("me.lxduo.wechat.fake::")
    }

    pub fn is_fake_msg_id(&self) -> bool {
        self.msg_id.starts_with("FAKE::") || self.msg_id == self.mxid
    }

    pub fn new(
        mxid: String,
        room_mxid: String,
        sender_mxid: String,
        msg_id: String,
        sender_id: String,
        timestamp: i64,
    ) -> Self {
        let key = PortalKey::new(&room_mxid, &sender_id);
        Self {
            chat_uid: room_mxid,
            chat_receiver: sender_id,
            msg_id,
            mxid,
            sender: sender_mxid,
            timestamp,
            sent: true,
            error: None,
            msg_type: String::new(),
        }
    }
}

pub struct MessageQuery;

macro_rules! impl_message_query_for_conn {
    (
        $get_by_id:ident,
        $get_by_mxid:ident,
        $get_by_msg_id:ident,
        $get_last:ident,
        $insert:ident,
        $update_mxid:ident,
        $delete:ident,
        $conn_ty:ty
    ) => {
        pub fn $get_by_id(
            conn: &mut $conn_ty,
            key: &PortalKey,
            msg_id: &str,
        ) -> Result<Option<Message>> {
            let item = message::table
                .select(Message::as_select())
                .filter(message::chat_uid.eq(&key.uid))
                .filter(message::chat_receiver.eq(&key.receiver))
                .filter(message::msg_id.eq(msg_id))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_by_mxid(conn: &mut $conn_ty, mxid: &str) -> Result<Option<Message>> {
            let item = message::table
                .select(Message::as_select())
                .filter(message::mxid.eq(mxid))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_by_msg_id(conn: &mut $conn_ty, msg_id: &str) -> Result<Option<Message>> {
            let item = message::table
                .select(Message::as_select())
                .filter(message::msg_id.eq(msg_id))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_last(conn: &mut $conn_ty, key: &PortalKey) -> Result<Option<Message>> {
            let item = message::table
                .select(Message::as_select())
                .filter(message::chat_uid.eq(&key.uid))
                .filter(message::chat_receiver.eq(&key.receiver))
                .order(message::timestamp.desc())
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $insert(conn: &mut $conn_ty, item: &Message) -> Result<()> {
            diesel::insert_into(message::table)
                .values(item)
                .execute(conn)?;
            Ok(())
        }

        pub fn $update_mxid(
            conn: &mut $conn_ty,
            key: &PortalKey,
            msg_id: &str,
            mxid: &str,
            msg_type: &str,
            error: Option<&str>,
        ) -> Result<()> {
            diesel::update(
                message::table
                    .filter(message::chat_uid.eq(&key.uid))
                    .filter(message::chat_receiver.eq(&key.receiver))
                    .filter(message::msg_id.eq(msg_id)),
            )
            .set((
                message::mxid.eq(mxid),
                message::msg_type.eq(msg_type),
                message::error.eq(error),
            ))
            .execute(conn)?;
            Ok(())
        }

        pub fn $delete(conn: &mut $conn_ty, key: &PortalKey, msg_id: &str) -> Result<()> {
            diesel::delete(
                message::table
                    .filter(message::chat_uid.eq(&key.uid))
                    .filter(message::chat_receiver.eq(&key.receiver))
                    .filter(message::msg_id.eq(msg_id)),
            )
            .execute(conn)?;
            Ok(())
        }
    };
}

impl MessageQuery {
    impl_message_query_for_conn!(
        get_by_id_sqlite,
        get_by_mxid_sqlite,
        get_by_msg_id_sqlite,
        get_last_sqlite,
        insert_sqlite,
        update_mxid_sqlite,
        delete_sqlite,
        SqliteConnection
    );

    impl_message_query_for_conn!(
        get_by_id_postgres,
        get_by_mxid_postgres,
        get_by_msg_id_postgres,
        get_last_postgres,
        insert_postgres,
        update_mxid_postgres,
        delete_postgres,
        PgConnection
    );
}
