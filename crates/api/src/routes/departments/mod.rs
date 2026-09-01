mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub(crate) use handler::*;

use crate::{middleware::permission::permission, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            permission("system:dept:list", get(handler::get_dept_tree)),
        )
        .route(
            "/",
            permission("system:dept:create", post(handler::create_dept)),
        )
        .route(
            "/{id}",
            permission("system:dept:get", get(handler::find_dept_by_id)),
        )
        .route(
            "/{id}",
            permission("system:dept:update", put(handler::update_dept_by_id)),
        )
        .route(
            "/{id}",
            permission("system:dept:delete", delete(handler::delete_dept_by_id)),
        )
}
