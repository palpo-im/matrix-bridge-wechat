use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use salvo::prelude::*;
use salvo::conn::TcpListener;
use tracing::{info, debug, warn, error};

use crate::matrix::types::*;
use super::MatrixClient;

pub struct AppService {
    pub as_token: String,
    pub hs_token: String,
    pub bot_mxid: String,
    pub bot_client: Arc<MatrixClient>,
    pub homeserver: String,
    pub bridge: Arc<dyn AppServiceBridge>,
}

pub trait AppServiceBridge: Send + Sync {
    fn handle_transaction(&self, txn_id: &str, events: Vec<RoomEvent>) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + '_>>;
    fn is_user_in_namespace(&self, mxid: &str) -> bool;
}

impl AppService {
    pub fn new(
        as_token: &str,
        hs_token: &str,
        bot_mxid: &str,
        homeserver: &str,
        bridge: Arc<dyn AppServiceBridge>,
    ) -> Self {
        let bot_client = Arc::new(MatrixClient::new(homeserver.to_string(), as_token.to_string()).with_user_id(bot_mxid));
        
        Self {
            as_token: as_token.to_string(),
            hs_token: hs_token.to_string(),
            bot_mxid: bot_mxid.to_string(),
            bot_client,
            homeserver: homeserver.to_string(),
            bridge,
        }
    }

    pub async fn start(self: Arc<Self>, addr: impl Into<String> + 'static) -> anyhow::Result<()> {
        let addr = addr.into();
        info!("Starting AppService on {}", addr);
        
        let router = Router::new()
            .push(Router::with_path("/_matrix/app/v1/transactions/<txn_id>")
                .put(TransactionHandler { as_: self.clone() }))
            .push(Router::with_path("/_matrix/app/v1/users/<user_id>")
                .get(UserHandler { as_: self.clone() }))
            .push(Router::with_path("/_matrix/app/v1/rooms/<room_alias>")
                .get(RoomHandler { as_: self }));
        
        let addr_for_listener = addr.to_string();
        let listener = TcpListener::new(addr_for_listener).bind().await;
        Server::new(listener).serve(router).await;
        
        Ok(())
    }
}

struct TransactionHandler {
    as_: Arc<AppService>,
}

#[handler]
impl TransactionHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let txn_id = depot.get::<String>("txn_id").map(|s| s.as_str()).unwrap_or("");
        
        let body: Result<Transaction, _> = req.parse_json().await;
        let transaction = match body {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to parse transaction: {}", e);
                res.render(Json(serde_json::json!({})));
                return;
            }
        };

        debug!("Received transaction {} with {} events", txn_id, transaction.events.len());

        if let Err(e) = self.as_.bridge.handle_transaction(txn_id, transaction.events).await {
            error!("Error handling transaction: {}", e);
        }

        res.render(Json(serde_json::json!({})));
    }

    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.as_.hs_token
            }
            _ => false,
        }
    }
}

struct UserHandler {
    as_: Arc<AppService>,
}

#[handler]
impl UserHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let user_id = depot.get::<String>("user_id").map(|s| s.as_str()).unwrap_or("");
        
        if self.as_.bridge.is_user_in_namespace(user_id) {
            debug!("User {} is in namespace", user_id);
            res.render(Json(serde_json::json!({})));
        } else {
            res.render(StatusError::not_found());
        }
    }

    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.as_.hs_token
            }
            _ => false,
        }
    }
}

struct RoomHandler {
    as_: Arc<AppService>,
}

#[handler]
impl RoomHandler {
    async fn handle(&self, req: &mut Request, res: &mut Response, depot: &mut Depot) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let room_alias = depot.get::<String>("room_alias").map(|s| s.as_str()).unwrap_or("");
        debug!("Room alias query: {}", room_alias);
        
        res.render(StatusError::not_found());
    }

    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.as_.hs_token
            }
            _ => false,
        }
    }
}

pub fn format_mxid(localpart: &str, domain: &str) -> String {
    format!("@{}:{}", localpart, domain)
}

pub fn parse_mxid(mxid: &str) -> Option<(String, String)> {
    if !mxid.starts_with('@') {
        return None;
    }
    let rest = &mxid[1..];
    let parts: Vec<&str> = rest.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    Some((parts[0].to_string(), parts[1].to_string()))
}

pub fn is_mxid(mxid: &str) -> bool {
    parse_mxid(mxid).is_some()
}

pub fn localpart(mxid: &str) -> Option<String> {
    parse_mxid(mxid).map(|(local, _)| local)
}

pub fn server_name(mxid: &str) -> Option<String> {
    parse_mxid(mxid).map(|(_, server)| server)
}

pub fn make_wechat_user_mxid(prefix: &str, uin: &str, domain: &str) -> String {
    format!("@{}_{}:{}", prefix, uin, domain)
}

pub fn make_wechat_room_mxid(prefix: &str, chat_id: &str, domain: &str) -> String {
    format!("@{}_{}:{}", prefix, chat_id, domain)
}
