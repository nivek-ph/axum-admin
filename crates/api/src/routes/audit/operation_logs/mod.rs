mod dto;
mod handler;

use axum::{Router, routing::get};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_operation_log_list))
        .route("/{id}", get(handler::find_operation_log_by_id))
}
