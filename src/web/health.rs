use salvo::prelude::*;
use serde_json::json;

use crate::web::web_state;

#[handler]
pub async fn health_check(res: &mut Response) {
    res.render("OK");
}

#[handler]
pub async fn get_status(res: &mut Response) {
    let state = web_state();
    let uptime_seconds = state.started_at.elapsed().as_secs();

    let status = json!({
        "status": "running",
        "version": state.version,
        "uptime_seconds": uptime_seconds,
        "bridge": {
            "name": state.bridge_name,
        }
    });

    res.render(Json(status));
}
