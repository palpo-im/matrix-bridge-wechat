pub mod health;
pub mod provisioning;
pub mod thirdparty;

use std::sync::Arc;
use std::time::Instant;

use salvo::prelude::*;
use tracing::info;

use crate::bridge::WechatBridge;
use crate::matrix::AppService;

#[derive(Clone)]
pub struct WebState {
    pub started_at: Instant,
    pub bridge_name: String,
    pub version: String,
}

impl WebState {
    pub fn new() -> Self {
        Self {
            started_at: Instant::now(),
            bridge_name: "matrix-bridge-wechat".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

lazy_static::lazy_static! {
    static ref WEB_STATE: Arc<WebState> = Arc::new(WebState::new());
}

pub fn web_state() -> Arc<WebState> {
    WEB_STATE.clone()
}

pub fn create_router() -> Router {
    Router::new()
        .push(Router::with_path("/health").get(health::health_check))
        .push(Router::with_path("/status").get(health::get_status))
}

pub fn create_appservice_router(bridge: Arc<WechatBridge>) -> Router {
    let bridge_for_appservice = bridge.clone();
    let bridge_for_hoop = bridge.clone();
    
    let appservice = Arc::new(AppService::new(
        &bridge.config.appservice.as_token,
        &bridge.config.appservice.hs_token,
        &bridge.config.appservice.bot.mxid(&bridge.config.homeserver.domain),
        &bridge.config.homeserver.address,
        Arc::new((*bridge_for_appservice).clone()),
    ));
    
    Router::new()
        .hoop(BridgeHoop { bridge: bridge_for_hoop })
        .push(Router::with_path("/_matrix/app/v1/transactions/{txn_id}")
            .put(AppserviceTransactionHandler { appservice: appservice.clone() }))
        .push(Router::with_path("/_matrix/app/v1/users/{user_id}")
            .get(AppserviceUserHandler { appservice: appservice.clone() }))
        .push(Router::with_path("/_matrix/app/v1/rooms/{room_alias}")
            .get(AppserviceRoomHandler { appservice }))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/protocol")
            .get(thirdparty::get_protocol))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/protocol/wechat")
            .get(thirdparty::get_network))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/network")
            .get(thirdparty::get_networks))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/network/wechat")
            .get(thirdparty::get_networks))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/location")
            .get(thirdparty::get_locations))
        .push(Router::with_path("/_matrix/app/v1/thirdparty/user")
            .get(thirdparty::get_users))
        .push(Router::with_path("/_matrix/app/v1/bridges")
            .get(provisioning::list_rooms)
            .post(provisioning::create_bridge))
        .push(Router::with_path("/_matrix/app/v1/bridge")
            .get(provisioning::get_bridge_info)
            .delete(provisioning::delete_bridge))
        .push(Router::with_path("/health").get(health::health_check))
        .push(Router::with_path("/status").get(health::get_status))
}

struct BridgeHoop {
    bridge: Arc<WechatBridge>,
}

#[async_trait::async_trait]
impl Handler for BridgeHoop {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, ctrl: &mut FlowCtrl) {
        depot.insert("bridge", self.bridge.clone());
        ctrl.call_next(req, depot, res).await;
    }
}

struct AppserviceTransactionHandler {
    appservice: Arc<AppService>,
}

#[async_trait::async_trait]
impl Handler for AppserviceTransactionHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let txn_id = depot.get::<String>("txn_id").map(|s| s.as_str()).unwrap_or("");
        
        let body: Result<crate::matrix::types::Transaction, _> = req.parse_json().await;
        let transaction = match body {
            Ok(t) => t,
            Err(e) => {
                info!("Failed to parse transaction: {}", e);
                res.render(Json(serde_json::json!({})));
                return;
            }
        };

        info!("Received transaction {} with {} events", txn_id, transaction.events.len());

        if let Err(e) = self.appservice.bridge.handle_transaction(txn_id, transaction.events).await {
            info!("Error handling transaction: {}", e);
        }

        res.render(Json(serde_json::json!({})));
    }
}

impl AppserviceTransactionHandler {
    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.appservice.hs_token
            }
            _ => false,
        }
    }
}

struct AppserviceUserHandler {
    appservice: Arc<AppService>,
}

#[async_trait::async_trait]
impl Handler for AppserviceUserHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let user_id = depot.get::<String>("user_id").map(|s| s.as_str()).unwrap_or("");
        
        if self.appservice.bridge.is_user_in_namespace(user_id) {
            info!("User {} is in namespace", user_id);
            res.render(Json(serde_json::json!({})));
        } else {
            res.render(StatusError::not_found());
        }
    }
}

impl AppserviceUserHandler {
    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.appservice.hs_token
            }
            _ => false,
        }
    }
}

struct AppserviceRoomHandler {
    appservice: Arc<AppService>,
}

#[async_trait::async_trait]
impl Handler for AppserviceRoomHandler {
    async fn handle(&self, req: &mut Request, depot: &mut Depot, res: &mut Response, _ctrl: &mut FlowCtrl) {
        let auth = req.header::<String>("Authorization");
        if !self.verify_auth(&auth) {
            res.render(StatusError::unauthorized());
            return;
        }

        let room_alias = depot.get::<String>("room_alias").map(|s| s.as_str()).unwrap_or("");
        info!("Room alias query: {}", room_alias);
        
        res.render(StatusError::not_found());
    }
}

impl AppserviceRoomHandler {
    fn verify_auth(&self, auth: &Option<String>) -> bool {
        match auth {
            Some(header) if header.starts_with("Bearer ") => {
                &header[7..] == self.appservice.hs_token
            }
            _ => false,
        }
    }
}
