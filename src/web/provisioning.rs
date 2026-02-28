use salvo::prelude::*;
use serde_json::json;

use crate::bridge::WechatBridge;
use crate::database::PortalKey;

fn render_error(res: &mut Response, status: StatusCode, message: &str) {
    res.status_code(status);
    res.render(Json(json!({ "error": message })));
}

#[handler]
pub async fn list_rooms(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b.clone(),
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let limit = req.query::<i64>("limit").unwrap_or(100).clamp(1, 1000);
    let _offset = req.query::<i64>("offset").unwrap_or(0).max(0);

    match bridge.db.get_all_portals_with_mxid().await {
        Ok(portals) => {
            let rooms: Vec<serde_json::Value> = portals
                .into_iter()
                .take(limit as usize)
                .map(|p| {
                    json!({
                        "uid": p.uid,
                        "receiver": p.receiver,
                        "mxid": p.mxid,
                        "name": p.name,
                        "encrypted": p.encrypted,
                    })
                })
                .collect();
            
            res.render(Json(json!({
                "rooms": rooms,
                "count": rooms.len(),
            })));
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
pub async fn create_bridge(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b.clone(),
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let matrix_room_id = match req.query::<String>("matrix_room_id") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(
                res,
                StatusCode::BAD_REQUEST,
                "missing matrix_room_id query parameter",
            );
            return;
        }
    };
    
    let wechat_chat_id = match req.query::<String>("wechat_chat_id") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(
                res,
                StatusCode::BAD_REQUEST,
                "missing wechat_chat_id query parameter",
            );
            return;
        }
    };

    let receiver = req.query::<String>("receiver").unwrap_or_default();
    
    let key = PortalKey::new(wechat_chat_id.clone(), receiver.clone());
    
    match bridge.db.get_portal_by_key(&key).await {
        Ok(Some(_)) => {
            render_error(res, StatusCode::BAD_REQUEST, "bridge already exists");
        }
        Ok(None) => {
            let portal = crate::database::Portal {
                uid: wechat_chat_id,
                receiver,
                mxid: Some(matrix_room_id.clone()),
                name: String::new(),
                name_set: false,
                topic: String::new(),
                topic_set: false,
                avatar: String::new(),
                avatar_url: None,
                avatar_set: false,
                encrypted: false,
                last_sync: 0,
                first_event_id: None,
                next_batch_id: None,
            };
            
            match bridge.db.insert_portal(&portal).await {
                Ok(()) => {
                    res.status_code(StatusCode::CREATED);
                    res.render(Json(json!({
                        "ok": true,
                        "message": "bridge created successfully",
                        "matrix_room_id": matrix_room_id,
                    })));
                }
                Err(err) => {
                    render_error(res, StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", err));
                }
            }
        }
        Err(err) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", err));
        }
    }
}

#[handler]
pub async fn delete_bridge(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b.clone(),
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let uid = match req.query::<String>("uid") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(res, StatusCode::BAD_REQUEST, "missing uid query parameter");
            return;
        }
    };
    
    let receiver = match req.query::<String>("receiver") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(res, StatusCode::BAD_REQUEST, "missing receiver query parameter");
            return;
        }
    };

    let key = PortalKey::new(uid, receiver);
    
    match bridge.db.get_portal_by_key(&key).await {
        Ok(Some(portal)) => {
            if let Some(mxid) = &portal.mxid {
                let client = bridge.get_matrix_client();
                if let Err(e) = client.leave_room(mxid).await {
                    tracing::warn!("Failed to leave room {}: {}", mxid, e);
                }
            }
            
            match bridge.db.delete_portal(&key).await {
                Ok(()) => {
                    res.render(Json(json!({ "ok": true, "message": "bridge deleted" })));
                }
                Err(err) => {
                    render_error(res, StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", err));
                }
            }
        }
        Ok(None) => {
            render_error(res, StatusCode::NOT_FOUND, "bridge not found");
        }
        Err(err) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", err));
        }
    }
}

#[handler]
pub async fn get_bridge_info(req: &mut Request, res: &mut Response, depot: &mut Depot) {
    let bridge = match depot.get::<std::sync::Arc<WechatBridge>>("bridge") {
        Ok(b) => b.clone(),
        Err(_) => {
            render_error(res, StatusCode::INTERNAL_SERVER_ERROR, "bridge not available");
            return;
        }
    };

    let uid = match req.query::<String>("uid") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(res, StatusCode::BAD_REQUEST, "missing uid query parameter");
            return;
        }
    };
    
    let receiver = match req.query::<String>("receiver") {
        Some(v) if !v.is_empty() => v,
        _ => {
            render_error(res, StatusCode::BAD_REQUEST, "missing receiver query parameter");
            return;
        }
    };

    let key = PortalKey::new(uid, receiver);
    
    match bridge.db.get_portal_by_key(&key).await {
        Ok(Some(portal)) => {
            res.render(Json(json!({ "portal": portal })));
        }
        Ok(None) => {
            render_error(res, StatusCode::NOT_FOUND, "bridge not found");
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
