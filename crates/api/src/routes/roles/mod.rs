mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, put},
};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_roles).post(handler::create_role))
        .route(
            "/{id}",
            put(handler::update_role).delete(handler::delete_role),
        )
        .route(
            "/{id}/menus",
            get(handler::get_role_menus).put(handler::set_role_menus),
        )
        .route(
            "/{id}/depts",
            get(handler::get_role_depts).put(handler::set_role_depts),
        )
        .route(
            "/{id}/users",
            get(handler::get_role_users).put(handler::set_role_users),
        )
}
