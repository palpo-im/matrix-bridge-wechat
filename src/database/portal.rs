use super::PortalKey;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::schema::portal;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = portal)]
pub struct Portal {
    pub uid: String,
    pub receiver: String,
    pub mxid: Option<String>,
    pub name: String,
    pub name_set: bool,
    pub topic: String,
    pub topic_set: bool,
    pub avatar: String,
    pub avatar_url: Option<String>,
    pub avatar_set: bool,
    pub encrypted: bool,
    pub last_sync: i64,
    pub first_event_id: Option<String>,
    pub next_batch_id: Option<String>,
}

impl Portal {
    pub fn key(&self) -> PortalKey {
        PortalKey::new(&self.uid, &self.receiver)
    }

    pub fn last_sync_time(&self) -> Option<DateTime<Utc>> {
        if self.last_sync > 0 {
            DateTime::from_timestamp(self.last_sync, 0)
        } else {
            None
        }
    }

    pub fn set_last_sync(&mut self, time: DateTime<Utc>) {
        self.last_sync = time.timestamp();
    }
}

pub struct PortalQuery;

macro_rules! impl_portal_query_for_conn {
    (
        $get_by_key:ident,
        $get_by_mxid:ident,
        $get_all_with_mxid:ident,
        $insert:ident,
        $update:ident,
        $delete:ident,
        $conn_ty:ty
    ) => {
        pub fn $get_by_key(conn: &mut $conn_ty, key: &PortalKey) -> Result<Option<Portal>> {
            let item = portal::table
                .select(Portal::as_select())
                .filter(portal::uid.eq(&key.uid))
                .filter(portal::receiver.eq(&key.receiver))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_by_mxid(conn: &mut $conn_ty, mxid: &str) -> Result<Option<Portal>> {
            let item = portal::table
                .select(Portal::as_select())
                .filter(portal::mxid.eq(mxid))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_all_with_mxid(conn: &mut $conn_ty) -> Result<Vec<Portal>> {
            let items = portal::table
                .select(Portal::as_select())
                .filter(portal::mxid.is_not_null())
                .load(conn)?;
            Ok(items)
        }

        pub fn $insert(conn: &mut $conn_ty, item: &Portal) -> Result<()> {
            diesel::insert_into(portal::table).values(item).execute(conn)?;
            Ok(())
        }

        pub fn $update(conn: &mut $conn_ty, item: &Portal) -> Result<()> {
            diesel::update(
                portal::table
                    .filter(portal::uid.eq(&item.uid))
                    .filter(portal::receiver.eq(&item.receiver)),
            )
            .set((
                portal::mxid.eq(&item.mxid),
                portal::name.eq(&item.name),
                portal::name_set.eq(item.name_set),
                portal::topic.eq(&item.topic),
                portal::topic_set.eq(item.topic_set),
                portal::avatar.eq(&item.avatar),
                portal::avatar_url.eq(&item.avatar_url),
                portal::avatar_set.eq(item.avatar_set),
                portal::encrypted.eq(item.encrypted),
                portal::last_sync.eq(item.last_sync),
                portal::first_event_id.eq(&item.first_event_id),
                portal::next_batch_id.eq(&item.next_batch_id),
            ))
            .execute(conn)?;
            Ok(())
        }

        pub fn $delete(conn: &mut $conn_ty, key: &PortalKey) -> Result<()> {
            diesel::delete(
                portal::table
                    .filter(portal::uid.eq(&key.uid))
                    .filter(portal::receiver.eq(&key.receiver)),
            )
            .execute(conn)?;
            Ok(())
        }
    };
}

impl PortalQuery {
    impl_portal_query_for_conn!(
        get_by_key_sqlite,
        get_by_mxid_sqlite,
        get_all_with_mxid_sqlite,
        insert_sqlite,
        update_sqlite,
        delete_sqlite,
        SqliteConnection
    );

    impl_portal_query_for_conn!(
        get_by_key_postgres,
        get_by_mxid_postgres,
        get_all_with_mxid_postgres,
        insert_postgres,
        update_postgres,
        delete_postgres,
        PgConnection
    );
}
