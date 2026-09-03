mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub(crate) use handler::*;

use crate::{middleware::permission::PermissionRouteExt, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handler::get_dept_tree).permission("system:dept:list"),
        )
        .route(
            "/",
            post(handler::create_dept).permission("system:dept:create"),
        )
        .route(
            "/{id}",
            get(handler::find_dept_by_id).permission("system:dept:get"),
        )
        .route(
            "/{id}",
            put(handler::update_dept_by_id).permission("system:dept:update"),
        )
        .route(
            "/{id}",
            delete(handler::delete_dept_by_id).permission("system:dept:delete"),
        )
}
