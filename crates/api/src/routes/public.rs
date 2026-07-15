use axum::{
    Router,
    routing::{get, post},
};

use super::{auth, health};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .route("/health", get(health::health))
        .route("/auth/login", post(auth::login))
        .route("/auth/captcha", post(auth::captcha))
}
