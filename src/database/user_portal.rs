use super::PortalKey;
use anyhow::Result;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::schema::user_portal;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = user_portal)]
pub struct UserPortal {
    pub user_mxid: String,
    pub portal_uid: String,
    pub portal_receiver: String,
    pub last_read_ts: i64,
    pub in_space: bool,
}

pub struct UserPortalQuery;

macro_rules! impl_user_portal_query_for_conn {
    (
        $get_last_read_ts:ident,
        $set_last_read_ts:ident,
        $is_in_space:ident,
        $mark_in_space:ident,
        $find_portals_not_in_space:ident,
        $conn_ty:ty
    ) => {
        pub fn $get_last_read_ts(
            conn: &mut $conn_ty,
            user_mxid: &str,
            key: &PortalKey,
        ) -> Result<i64> {
            let ts = user_portal::table
                .select(user_portal::last_read_ts)
                .filter(user_portal::user_mxid.eq(user_mxid))
                .filter(user_portal::portal_uid.eq(&key.uid))
                .filter(user_portal::portal_receiver.eq(&key.receiver))
                .first::<i64>(conn)
                .optional()?;
            Ok(ts.unwrap_or(0))
        }

        pub fn $set_last_read_ts(
            conn: &mut $conn_ty,
            user_mxid: &str,
            key: &PortalKey,
            ts: i64,
        ) -> Result<()> {
            diesel::sql_query(
                "INSERT INTO user_portal (user_mxid, portal_uid, portal_receiver, last_read_ts) \
                 VALUES ($1, $2, $3, $4) \
                 ON CONFLICT (user_mxid, portal_uid, portal_receiver) \
                 DO UPDATE SET last_read_ts=excluded.last_read_ts \
                 WHERE user_portal.last_read_ts<excluded.last_read_ts",
            )
            .bind::<diesel::sql_types::Text, _>(user_mxid)
            .bind::<diesel::sql_types::Text, _>(&key.uid)
            .bind::<diesel::sql_types::Text, _>(&key.receiver)
            .bind::<diesel::sql_types::BigInt, _>(ts)
            .execute(conn)?;
            Ok(())
        }

        pub fn $is_in_space(
            conn: &mut $conn_ty,
            user_mxid: &str,
            key: &PortalKey,
        ) -> Result<bool> {
            let in_space = user_portal::table
                .select(user_portal::in_space)
                .filter(user_portal::user_mxid.eq(user_mxid))
                .filter(user_portal::portal_uid.eq(&key.uid))
                .filter(user_portal::portal_receiver.eq(&key.receiver))
                .first::<bool>(conn)
                .optional()?;
            Ok(in_space.unwrap_or(false))
        }

        pub fn $mark_in_space(
            conn: &mut $conn_ty,
            user_mxid: &str,
            key: &PortalKey,
        ) -> Result<()> {
            diesel::sql_query(
                "INSERT INTO user_portal (user_mxid, portal_uid, portal_receiver, in_space) \
                 VALUES ($1, $2, $3, true) \
                 ON CONFLICT (user_mxid, portal_uid, portal_receiver) \
                 DO UPDATE SET in_space=true",
            )
            .bind::<diesel::sql_types::Text, _>(user_mxid)
            .bind::<diesel::sql_types::Text, _>(&key.uid)
            .bind::<diesel::sql_types::Text, _>(&key.receiver)
            .execute(conn)?;
            Ok(())
        }

        pub fn $find_portals_not_in_space(
            conn: &mut $conn_ty,
            receiver: &str,
        ) -> Result<Vec<PortalKey>> {
            use diesel::sql_types::Text;

            let results: Vec<PortalUidRow> = diesel::sql_query(
                "SELECT uid FROM portal \
                 LEFT JOIN user_portal ON portal.uid=user_portal.portal_uid \
                     AND portal.receiver=user_portal.portal_receiver \
                 WHERE mxid<>'' AND receiver=$1 AND (in_space=false OR in_space IS NULL)",
            )
            .bind::<Text, _>(receiver)
            .load(conn)?;

            Ok(results
                .into_iter()
                .map(|row| PortalKey::new(&row.uid, receiver))
                .collect())
        }
    };
}

#[derive(Debug, QueryableByName)]
struct PortalUidRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    uid: String,
}

impl UserPortalQuery {
    impl_user_portal_query_for_conn!(
        get_last_read_ts_sqlite,
        set_last_read_ts_sqlite,
        is_in_space_sqlite,
        mark_in_space_sqlite,
        find_portals_not_in_space_sqlite,
        SqliteConnection
    );

    impl_user_portal_query_for_conn!(
        get_last_read_ts_postgres,
        set_last_read_ts_postgres,
        is_in_space_postgres,
        mark_in_space_postgres,
        find_portals_not_in_space_postgres,
        PgConnection
    );
}
