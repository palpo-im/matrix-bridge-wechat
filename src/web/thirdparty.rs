use std::collections::HashMap;

use salvo::prelude::*;
use serde::Serialize;
use serde_json::json;

use crate::bridge::WechatBridge;

fn render_error(res: &mut Response, status: StatusCode, message: &str) {
    res.status_code(status);
    res.render(Json(json!({ "error": message })));
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyProtocol {
    pub user_fields: Vec<String>,
    pub location_fields: Vec<String>,
    pub field_types: HashMap<String, ThirdPartyFieldType>,
    pub instances: Vec<ThirdPartyInstance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyFieldType {
    #[serde(rename = "type")]
    pub field_type: String,
    pub placeholder: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyInstance {
    pub network_id: String,
    pub bot_user_id: String,
    pub desc: String,
    pub icon: Option<String>,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyLocation {
    pub alias: String,
    pub protocol: String,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyUser {
    pub userid: String,
    pub protocol: String,
    pub fields: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThirdPartyNetwork {
    pub name: String,
    pub protocol: String,
    pub fields: HashMap<String, String>,
}

fn protocol_payload(bot_user_id: &str, domain: &str) -> ThirdPartyProtocol {
    let mut field_types = HashMap::new();
    field_types.insert(
        "chat_id".to_string(),
        ThirdPartyFieldType {
            field_type: "text".to_string(),
            placeholder: "WeChat chat id".to_string(),
        },
    );
    field_types.insert(
        "user_id".to_string(),
        ThirdPartyFieldType {
            field_type: "text".to_string(),
            placeholder: "WeChat user id".to_string(),
        },
    );

    ThirdPartyProtocol {
        user_fields: vec!["user_id".to_string()],
        location_fields: vec!["chat_id".to_string()],
        field_types,
        instances: vec![ThirdPartyInstance {
            network_id: "wechat".to_string(),
            bot_user_id: bot_user_id.to_string(),
            desc: "WeChat".to_string(),
            icon: Some("mxc://maunium.net/wechat".to_string()),
            fields: HashMap::from([
                ("domain".to_string(), domain.to_string()),
            ]),
        }],
    }
}

#[handler]
pub async fn get_protocol(depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let bot_user_id = bridge.config.appservice.bot.mxid(&bridge.config.homeserver.domain);
    res.render(Json(protocol_payload(&bot_user_id, &bridge.config.homeserver.domain)));
}

#[handler]
pub async fn get_network(depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let bot_user_id = bridge.config.appservice.bot.mxid(&bridge.config.homeserver.domain);
    res.render(Json(protocol_payload(&bot_user_id, &bridge.config.homeserver.domain)));
}

#[handler]
pub async fn get_networks(depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    match bridge.db.get_all_portals_with_mxid().await {
        Ok(portals) => {
            let mut networks: Vec<ThirdPartyNetwork> = Vec::new();
            
            let mut seen = std::collections::HashSet::new();
            for portal in portals {
                let is_group = portal.uid.starts_with("@@");
                let network_name = if is_group { "groups" } else { "users" };
                
                if seen.insert(network_name.to_string()) {
                    networks.push(ThirdPartyNetwork {
                        name: network_name.to_string(),
                        protocol: "wechat".to_string(),
                        fields: HashMap::from([
                            ("type".to_string(), if is_group { "group" } else { "user" }.to_string()),
                        ]),
                    });
                }
            }
            
            if networks.is_empty() {
                networks.push(ThirdPartyNetwork {
                    name: "wechat".to_string(),
                    protocol: "wechat".to_string(),
                    fields: HashMap::new(),
                });
            }
            
            res.render(Json(networks));
        }
        Err(err) => {
            render_error(
                res,
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("database error: {}", err),
            );
        }
    }
}

#[handler]
pub async fn get_locations(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let chat_filter = req.query::<String>("chat_id");

    match bridge.db.get_all_portals_with_mxid().await {
        Ok(portals) => {
            let locations: Vec<ThirdPartyLocation> = portals
                .into_iter()
                .filter(|portal| {
                    chat_filter
                        .as_ref()
                        .map(|chat| portal.uid.contains(chat))
                        .unwrap_or(true)
                })
                .filter_map(|portal| {
                    portal.mxid.map(|mxid| ThirdPartyLocation {
                        alias: mxid,
                        protocol: "wechat".to_string(),
                        fields: HashMap::from([
                            ("chat_id".to_string(), portal.uid.clone()),
                            ("receiver".to_string(), portal.receiver.clone()),
                        ]),
                    })
                })
                .collect();
            
            res.render(Json(locations));
        }
        Err(err) => {
            render_error(
                res,
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("database error: {}", err),
            );
        }
    }
}

#[handler]
pub async fn get_user(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let user_filter = req.query::<String>("user_id").or_else(|| req.query::<String>("userid"));
    let domain = &bridge.config.homeserver.domain;
    let user_prefix = &bridge.config.bridge.user_prefix;

    match bridge.db.get_all_puppets_with_custom_mxid().await {
        Ok(puppets) => {
            let users: Vec<ThirdPartyUser> = puppets
                .into_iter()
                .filter(|puppet| {
                    user_filter
                        .as_ref()
                        .map(|filter| puppet.uin.contains(filter))
                        .unwrap_or(true)
                })
                .map(|puppet| {
                    let mxid = puppet.custom_mxid.unwrap_or_else(|| {
                        format!("@{}{}:{}", user_prefix, puppet.uin, domain)
                    });
                    ThirdPartyUser {
                        userid: mxid,
                        protocol: "wechat".to_string(),
                        fields: HashMap::from([
                            ("uin".to_string(), puppet.uin.clone()),
                            ("displayname".to_string(), puppet.displayname.unwrap_or_default()),
                        ]),
                    }
                })
                .collect();
            
            res.render(Json(users));
        }
        Err(err) => {
            render_error(
                res,
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("database error: {}", err),
            );
        }
    }
}

#[handler]
pub async fn get_users(req: &mut Request, depot: &mut Depot, res: &mut Response) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b,
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let user_filter = req.query::<String>("user_id").or_else(|| req.query::<String>("userid"));
    let domain = &bridge.config.homeserver.domain;
    let user_prefix = &bridge.config.bridge.user_prefix;

    match bridge.db.get_all_puppets_with_custom_mxid().await {
        Ok(puppets) => {
            let users: Vec<ThirdPartyUser> = puppets
                .into_iter()
                .filter(|puppet| {
                    user_filter
                        .as_ref()
                        .map(|filter| puppet.uin.contains(filter))
                        .unwrap_or(true)
                })
                .map(|puppet| {
                    let mxid = puppet.custom_mxid.unwrap_or_else(|| {
                        format!("@{}{}:{}", user_prefix, puppet.uin, domain)
                    });
                    ThirdPartyUser {
                        userid: mxid,
                        protocol: "wechat".to_string(),
                        fields: HashMap::from([
                            ("uin".to_string(), puppet.uin.clone()),
                            ("displayname".to_string(), puppet.displayname.unwrap_or_default()),
                        ]),
                    }
                })
                .collect();
            
            res.render(Json(users));
        }
        Err(err) => {
            render_error(
                res,
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("database error: {}", err),
            );
        }
    }
}
