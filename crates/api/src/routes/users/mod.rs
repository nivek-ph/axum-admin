pub(crate) mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, post, put},
};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_user_info).put(set_self_info))
        .route("/me/password", put(change_password))
        .route("/me/settings", put(set_self_setting))
        .route("/", get(get_user_list_by_query).post(admin_register))
        .route("/{id}", put(set_user_info_by_id).delete(delete_user_by_id))
        .route("/{id}/password/reset", post(reset_password_by_id))
        .route("/{id}/roles", put(set_user_roles_by_id))
}
