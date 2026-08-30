pub mod audit;
pub mod auth;
pub mod departments;
pub mod dictionaries;
pub mod files;
pub mod health;
pub mod menus;
pub mod parameters;
pub mod protected;
pub mod public;
pub mod roles;
pub mod storages;
pub mod users;

use axum::Router;

pub fn public_routes() -> Router<crate::state::AppState> {
    public::router()
}

pub fn public_root_routes() -> Router<crate::state::AppState> {
    public::root_router()
}

pub fn captcha_routes() -> Router<crate::state::AppState> {
    auth::captcha_routes()
}

pub fn protected_routes() -> Router<crate::state::AppState> {
    protected::router()
}
