mod schema;
mod user;
mod portal;
mod puppet;
mod message;

pub use user::*;
pub use portal::*;
pub use puppet::*;
pub use message::*;

use anyhow::Context;
use anyhow::Result;
use diesel::connection::SimpleConnection;
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::sqlite::SqliteConnection;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Database {
    inner: DatabaseInner,
}

#[derive(Debug, Clone)]
enum DatabaseInner {
    Sqlite(Pool<ConnectionManager<SqliteConnection>>),
    Postgres(Pool<ConnectionManager<PgConnection>>),
}

impl Database {
    pub async fn connect(db_type: &str, uri: &str, max_open: u32, max_idle: u32) -> Result<Self> {
        let max_open = max_open.max(1);
        let max_idle = max_idle.min(max_open);
        let db_type = db_type.trim().to_ascii_lowercase();

        match db_type.as_str() {
            "sqlite" | "sqlite3" => {
                info!("Connecting to SQLite database with Diesel");
                let database_url = normalize_sqlite_uri(uri);
                let manager = ConnectionManager::<SqliteConnection>::new(database_url);
                let pool = Pool::builder()
                    .max_size(max_open)
                    .min_idle(Some(max_idle))
                    .build(manager)
                    .context("failed to create sqlite connection pool")?;
                Ok(Self {
                    inner: DatabaseInner::Sqlite(pool),
                })
            }
            "postgres" | "postgresql" | "pgsql" => {
                info!("Connecting to PostgreSQL database with Diesel");
                let manager = ConnectionManager::<PgConnection>::new(uri.to_owned());
                let pool = Pool::builder()
                    .max_size(max_open)
                    .min_idle(Some(max_idle))
                    .build(manager)
                    .context("failed to create postgres connection pool")?;
                Ok(Self {
                    inner: DatabaseInner::Postgres(pool),
                })
            }
            _ => anyhow::bail!(
                "Unsupported database type: {db_type}. Supported types: sqlite/sqlite3/postgres/postgresql/pgsql"
            ),
        }
    }

    pub fn is_sqlite(&self) -> bool {
        matches!(self.inner, DatabaseInner::Sqlite(_))
    }

    pub async fn run_migrations(&self) -> Result<()> {
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                let migration_sql = include_str!("../../migrations/001_initial.sql")
                    .lines()
                    .filter(|line| !line.starts_with("-- only: postgres"))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.with_sqlite_conn(move |conn| {
                    conn.batch_execute(&migration_sql)?;
                    Ok(())
                })
                .await?;
            }
            DatabaseInner::Postgres(_) => {
                let migration_sql = include_str!("../../migrations/001_initial.sql");
                self.with_postgres_conn(move |conn| {
                    conn.batch_execute(migration_sql)?;
                    Ok(())
                })
                .await?;
            }
        }

        info!("Database migrations completed");
        Ok(())
    }

    pub async fn get_user_by_mxid(&self, mxid: &str) -> Result<Option<User>> {
        let mxid = mxid.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| UserQuery::get_by_mxid_sqlite(conn, &mxid))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| UserQuery::get_by_mxid_postgres(conn, &mxid))
                    .await
            }
        }
    }

    pub async fn get_user_by_uin(&self, uin: &str) -> Result<Option<User>> {
        let uin = uin.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| UserQuery::get_by_uin_sqlite(conn, &uin))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| UserQuery::get_by_uin_postgres(conn, &uin))
                    .await
            }
        }
    }

    pub async fn get_all_logged_in_users(&self) -> Result<Vec<User>> {
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(UserQuery::get_all_logged_in_sqlite).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(UserQuery::get_all_logged_in_postgres).await,
        }
    }

    pub async fn insert_user(&self, user: &User) -> Result<()> {
        let user = user.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| UserQuery::insert_sqlite(conn, &user)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| UserQuery::insert_postgres(conn, &user)).await,
        }
    }

    pub async fn update_user(&self, user: &User) -> Result<()> {
        let user = user.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| UserQuery::update_sqlite(conn, &user)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| UserQuery::update_postgres(conn, &user)).await,
        }
    }

    pub async fn get_portal_by_key(&self, key: &PortalKey) -> Result<Option<Portal>> {
        let key = key.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| PortalQuery::get_by_key_sqlite(conn, &key))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| PortalQuery::get_by_key_postgres(conn, &key))
                    .await
            }
        }
    }

    pub async fn get_portal_by_mxid(&self, mxid: &str) -> Result<Option<Portal>> {
        let mxid = mxid.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| PortalQuery::get_by_mxid_sqlite(conn, &mxid))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| PortalQuery::get_by_mxid_postgres(conn, &mxid))
                    .await
            }
        }
    }

    pub async fn get_all_portals_with_mxid(&self) -> Result<Vec<Portal>> {
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(PortalQuery::get_all_with_mxid_sqlite).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(PortalQuery::get_all_with_mxid_postgres).await,
        }
    }

    pub async fn insert_portal(&self, portal: &Portal) -> Result<()> {
        let portal = portal.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| PortalQuery::insert_sqlite(conn, &portal)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| PortalQuery::insert_postgres(conn, &portal)).await,
        }
    }

    pub async fn update_portal(&self, portal: &Portal) -> Result<()> {
        let portal = portal.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| PortalQuery::update_sqlite(conn, &portal)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| PortalQuery::update_postgres(conn, &portal)).await,
        }
    }

    pub async fn delete_portal(&self, key: &PortalKey) -> Result<()> {
        let key = key.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| PortalQuery::delete_sqlite(conn, &key)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| PortalQuery::delete_postgres(conn, &key)).await,
        }
    }

    pub async fn get_puppet_by_uin(&self, uin: &str) -> Result<Option<Puppet>> {
        let uin = uin.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| PuppetQuery::get_by_uin_sqlite(conn, &uin))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| PuppetQuery::get_by_uin_postgres(conn, &uin))
                    .await
            }
        }
    }

    pub async fn get_puppet_by_custom_mxid(&self, mxid: &str) -> Result<Option<Puppet>> {
        let mxid = mxid.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| PuppetQuery::get_by_custom_mxid_sqlite(conn, &mxid))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| PuppetQuery::get_by_custom_mxid_postgres(conn, &mxid))
                    .await
            }
        }
    }

    pub async fn get_all_puppets_with_custom_mxid(&self) -> Result<Vec<Puppet>> {
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(PuppetQuery::get_all_with_custom_mxid_sqlite).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(PuppetQuery::get_all_with_custom_mxid_postgres).await,
        }
    }

    pub async fn insert_puppet(&self, puppet: &Puppet) -> Result<()> {
        let puppet = puppet.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| PuppetQuery::insert_sqlite(conn, &puppet)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| PuppetQuery::insert_postgres(conn, &puppet)).await,
        }
    }

    pub async fn update_puppet(&self, puppet: &Puppet) -> Result<()> {
        let puppet = puppet.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| PuppetQuery::update_sqlite(conn, &puppet)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| PuppetQuery::update_postgres(conn, &puppet)).await,
        }
    }

    pub async fn get_message_by_id(&self, key: &PortalKey, msg_id: &str) -> Result<Option<Message>> {
        let key = key.clone();
        let msg_id = msg_id.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| MessageQuery::get_by_id_sqlite(conn, &key, &msg_id))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| MessageQuery::get_by_id_postgres(conn, &key, &msg_id))
                    .await
            }
        }
    }

    pub async fn get_message_by_mxid(&self, mxid: &str) -> Result<Option<Message>> {
        let mxid = mxid.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| MessageQuery::get_by_mxid_sqlite(conn, &mxid))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| MessageQuery::get_by_mxid_postgres(conn, &mxid))
                    .await
            }
        }
    }

    pub async fn get_message_by_wechat_id(&self, msg_id: &str) -> Result<Option<Message>> {
        let msg_id = msg_id.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| MessageQuery::get_by_msg_id_sqlite(conn, &msg_id))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| MessageQuery::get_by_msg_id_postgres(conn, &msg_id))
                    .await
            }
        }
    }

    pub async fn get_last_message(&self, key: &PortalKey) -> Result<Option<Message>> {
        let key = key.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| MessageQuery::get_last_sqlite(conn, &key))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| MessageQuery::get_last_postgres(conn, &key))
                    .await
            }
        }
    }

    pub async fn insert_message(&self, msg: &Message) -> Result<()> {
        let msg = msg.clone();
        match &self.inner {
            DatabaseInner::Sqlite(_) => self.with_sqlite_conn(move |conn| MessageQuery::insert_sqlite(conn, &msg)).await,
            DatabaseInner::Postgres(_) => self.with_postgres_conn(move |conn| MessageQuery::insert_postgres(conn, &msg)).await,
        }
    }

    pub async fn update_message_mxid(
        &self,
        key: &PortalKey,
        msg_id: &str,
        mxid: &str,
        msg_type: &str,
        error: Option<&str>,
    ) -> Result<()> {
        let key = key.clone();
        let msg_id = msg_id.to_owned();
        let mxid = mxid.to_owned();
        let msg_type = msg_type.to_owned();
        let error = error.map(str::to_owned);
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| {
                    MessageQuery::update_mxid_sqlite(conn, &key, &msg_id, &mxid, &msg_type, error.as_deref())
                })
                .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| {
                    MessageQuery::update_mxid_postgres(conn, &key, &msg_id, &mxid, &msg_type, error.as_deref())
                })
                .await
            }
        }
    }

    pub async fn delete_message(&self, key: &PortalKey, msg_id: &str) -> Result<()> {
        let key = key.clone();
        let msg_id = msg_id.to_owned();
        match &self.inner {
            DatabaseInner::Sqlite(_) => {
                self.with_sqlite_conn(move |conn| MessageQuery::delete_sqlite(conn, &key, &msg_id))
                    .await
            }
            DatabaseInner::Postgres(_) => {
                self.with_postgres_conn(move |conn| MessageQuery::delete_postgres(conn, &key, &msg_id))
                    .await
            }
        }
    }

    async fn with_sqlite_conn<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut SqliteConnection) -> Result<T> + Send + 'static,
    {
        let pool = match &self.inner {
            DatabaseInner::Sqlite(pool) => pool.clone(),
            DatabaseInner::Postgres(_) => anyhow::bail!("internal error: expected sqlite database"),
        };
        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .context("failed to get sqlite connection from pool")?;
            conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            f(&mut conn)
        })
        .await
        .context("diesel task join error")?
    }

    async fn with_postgres_conn<T, F>(&self, f: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut PgConnection) -> Result<T> + Send + 'static,
    {
        let pool = match &self.inner {
            DatabaseInner::Sqlite(_) => anyhow::bail!("internal error: expected postgres database"),
            DatabaseInner::Postgres(pool) => pool.clone(),
        };
        tokio::task::spawn_blocking(move || {
            let mut conn = pool
                .get()
                .context("failed to get postgres connection from pool")?;
            f(&mut conn)
        })
        .await
        .context("diesel task join error")?
    }
}

fn normalize_sqlite_uri(uri: &str) -> String {
    uri.strip_prefix("sqlite://")
        .or_else(|| uri.strip_prefix("sqlite:"))
        .unwrap_or(uri)
        .to_owned()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct PortalKey {
    pub uid: String,
    pub receiver: String,
}

impl PortalKey {
    pub fn new(uid: impl Into<String>, receiver: impl Into<String>) -> Self {
        Self {
            uid: uid.into(),
            receiver: receiver.into(),
        }
    }
}

impl std::fmt::Display for PortalKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.uid, self.receiver)
    }
}
