use anyhow::Result;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::schema::users;

#[derive(Debug, Clone, Queryable, Selectable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct User {
    pub mxid: String,
    pub uin: Option<String>,
    pub management_room: Option<String>,
    pub space_room: Option<String>,
}

impl User {
    pub fn new(mxid: impl Into<String>) -> Self {
        Self {
            mxid: mxid.into(),
            uin: None,
            management_room: None,
            space_room: None,
        }
    }

    pub fn uid(&self) -> Option<crate::util::UID> {
        self.uin.as_ref().map(|uin| crate::util::UID::new_user(uin))
    }
}

pub struct UserQuery;

macro_rules! impl_user_query_for_conn {
    ($get_by_mxid:ident, $get_by_uin:ident, $get_all_logged_in:ident, $insert:ident, $update:ident, $conn_ty:ty) => {
        pub fn $get_by_mxid(conn: &mut $conn_ty, mxid: &str) -> Result<Option<User>> {
            let user = users::table
                .select(User::as_select())
                .filter(users::mxid.eq(mxid))
                .first(conn)
                .optional()?;
            Ok(user)
        }

        pub fn $get_by_uin(conn: &mut $conn_ty, uin: &str) -> Result<Option<User>> {
            let user = users::table
                .select(User::as_select())
                .filter(users::uin.eq(uin))
                .first(conn)
                .optional()?;
            Ok(user)
        }

        pub fn $get_all_logged_in(conn: &mut $conn_ty) -> Result<Vec<User>> {
            let items = users::table
                .select(User::as_select())
                .filter(users::uin.is_not_null().and(users::uin.ne("")))
                .load(conn)?;
            Ok(items)
        }

        pub fn $insert(conn: &mut $conn_ty, user: &User) -> Result<()> {
            diesel::insert_into(users::table).values(user).execute(conn)?;
            Ok(())
        }

        pub fn $update(conn: &mut $conn_ty, user: &User) -> Result<()> {
            diesel::update(users::table.filter(users::mxid.eq(&user.mxid)))
                .set((
                    users::uin.eq(&user.uin),
                    users::management_room.eq(&user.management_room),
                    users::space_room.eq(&user.space_room),
                ))
                .execute(conn)?;
            Ok(())
        }
    };
}

impl UserQuery {
    impl_user_query_for_conn!(
        get_by_mxid_sqlite,
        get_by_uin_sqlite,
        get_all_logged_in_sqlite,
        insert_sqlite,
        update_sqlite,
        SqliteConnection
    );

    impl_user_query_for_conn!(
        get_by_mxid_postgres,
        get_by_uin_postgres,
        get_all_logged_in_postgres,
        insert_postgres,
        update_postgres,
        PgConnection
    );
}
