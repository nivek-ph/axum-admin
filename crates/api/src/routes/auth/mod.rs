pub(crate) mod captcha;
mod error;
pub(crate) mod login;
pub(crate) mod logout;
pub(crate) mod refresh;
use axum::{Router, routing::post};
pub(crate) use captcha::captcha;
pub(crate) use login::login;
pub(crate) use logout::logout;
pub(crate) use refresh::refresh;

use crate::state::AppState;

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/captcha", post(captcha))
}

pub fn protected_routes() -> Router<AppState> {
    Router::new().route("/auth/logout", post(logout))
}
