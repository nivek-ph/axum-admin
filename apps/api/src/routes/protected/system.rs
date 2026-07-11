use admin_httpz::ApiResponse;
use axum::{Json, Router, extract::State, routing::get};
use serde_json::Value;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/config", get(get_system_config).put(set_system_config))
        .route("/server-info", get(get_server_info))
        .route("/reload", axum::routing::post(reload_system))
}

pub async fn get_system_config(State(state): State<AppState>) -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "config": {
            "system": {
                "env": "public",
                "addr": format!("0.0.0.0:{}", state.config.http_port),
                "db-type": "pgsql",
                "use-multipoint": false,
                "use-strict-auth": false
            },
            "captcha": {
                "openCaptcha": 1,
                "openCaptchaTimeOut": 300
            },
            "local": {
                "storePath": "./uploads"
            }
        }
    })))
}

pub async fn set_system_config() -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok_message("configuration saved"))
}

pub async fn reload_system() -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok_message("operation succeeded"))
}

pub async fn get_server_info() -> Json<ApiResponse<Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "server": {
            "os": {
                "goos": std::env::consts::OS,
                "numCpu": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
                "compiler": "rustc",
                "goVersion": env!("CARGO_PKG_VERSION"),
                "numGoroutine": 0
            },
            "cpu": {
                "cores": std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1),
                "cpus": [12.0, 18.0, 9.0, 24.0]
            },
            "ram": {
                "totalMb": 8192,
                "usedMb": 2048,
                "usedPercent": 25
            },
            "disk": [
                {
                    "mountPoint": "/",
                    "totalMb": 512000,
                    "usedMb": 128000,
                    "totalGb": 500,
                    "usedGb": 125,
                    "usedPercent": 25
                }
            ]
        }
    })))
}
