mod dto;
mod handler;

use axum::{Router, routing::get};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_dept_tree).post(handler::create_dept))
        .route(
            "/{id}",
            get(handler::find_dept_by_id)
                .put(handler::update_dept_by_id)
                .delete(handler::delete_dept_by_id),
        )
}
