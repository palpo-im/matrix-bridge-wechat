use crate::util::UID;
use anyhow::Result;
use chrono::{DateTime, Utc};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::schema::puppet;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = puppet)]
pub struct Puppet {
    pub uin: String,
    pub avatar: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_set: bool,
    pub displayname: Option<String>,
    pub name_quality: i16,
    pub name_set: bool,
    pub last_sync: i64,
    pub custom_mxid: Option<String>,
    pub access_token: Option<String>,
    pub next_batch: Option<String>,
    pub enable_presence: bool,
}

impl Puppet {
    pub fn new(uin: impl Into<String>) -> Self {
        Self {
            uin: uin.into(),
            avatar: None,
            avatar_url: None,
            avatar_set: false,
            displayname: None,
            name_quality: 0,
            name_set: false,
            last_sync: 0,
            custom_mxid: None,
            access_token: None,
            next_batch: None,
            enable_presence: true,
        }
    }

    pub fn uid(&self) -> UID {
        UID::new_user(&self.uin)
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

pub struct PuppetQuery;

macro_rules! impl_puppet_query_for_conn {
    (
        $get_by_uin:ident,
        $get_by_custom_mxid:ident,
        $get_all_with_custom_mxid:ident,
        $insert:ident,
        $update:ident,
        $conn_ty:ty
    ) => {
        pub fn $get_by_uin(conn: &mut $conn_ty, uin: &str) -> Result<Option<Puppet>> {
            let item = puppet::table
                .select(Puppet::as_select())
                .filter(puppet::uin.eq(uin))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_by_custom_mxid(conn: &mut $conn_ty, mxid: &str) -> Result<Option<Puppet>> {
            let item = puppet::table
                .select(Puppet::as_select())
                .filter(puppet::custom_mxid.eq(mxid))
                .first(conn)
                .optional()?;
            Ok(item)
        }

        pub fn $get_all_with_custom_mxid(conn: &mut $conn_ty) -> Result<Vec<Puppet>> {
            let items = puppet::table
                .select(Puppet::as_select())
                .filter(puppet::custom_mxid.is_not_null().and(puppet::custom_mxid.ne("")))
                .load(conn)?;
            Ok(items)
        }

        pub fn $insert(conn: &mut $conn_ty, item: &Puppet) -> Result<()> {
            diesel::insert_into(puppet::table).values(item).execute(conn)?;
            Ok(())
        }

        pub fn $update(conn: &mut $conn_ty, item: &Puppet) -> Result<()> {
            diesel::update(puppet::table.filter(puppet::uin.eq(&item.uin)))
                .set((
                    puppet::displayname.eq(&item.displayname),
                    puppet::name_quality.eq(item.name_quality),
                    puppet::name_set.eq(item.name_set),
                    puppet::avatar.eq(&item.avatar),
                    puppet::avatar_url.eq(&item.avatar_url),
                    puppet::avatar_set.eq(item.avatar_set),
                    puppet::last_sync.eq(item.last_sync),
                    puppet::custom_mxid.eq(&item.custom_mxid),
                    puppet::access_token.eq(&item.access_token),
                    puppet::next_batch.eq(&item.next_batch),
                    puppet::enable_presence.eq(item.enable_presence),
                ))
                .execute(conn)?;
            Ok(())
        }
    };
}

impl PuppetQuery {
    impl_puppet_query_for_conn!(
        get_by_uin_sqlite,
        get_by_custom_mxid_sqlite,
        get_all_with_custom_mxid_sqlite,
        insert_sqlite,
        update_sqlite,
        SqliteConnection
    );

    impl_puppet_query_for_conn!(
        get_by_uin_postgres,
        get_by_custom_mxid_postgres,
        get_all_with_custom_mxid_postgres,
        insert_postgres,
        update_postgres,
        PgConnection
    );
}
