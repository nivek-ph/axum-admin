pub mod attachment_categories;
pub mod auth;
pub mod captcha;
pub mod departments;
pub mod dictionaries;
pub mod dictionary_details;
pub mod files;
pub mod health;
pub mod init;
pub mod login_logs;
pub mod menus;
pub mod operation_logs;
pub mod parameters;
pub mod protected;
pub mod public;
pub mod roles;
pub mod session;
pub mod system;
pub mod users;

use axum::Router;

pub fn public_routes() -> Router<crate::state::AppState> {
    public::router()
}

pub fn protected_routes() -> Router<crate::state::AppState> {
    protected::router()
}
