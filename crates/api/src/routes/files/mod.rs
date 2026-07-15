mod dto;
mod handler;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_file_list_by_query))
        .route("/import-url", post(handler::import_url))
        .route(
            "/upload",
            post(handler::upload_file).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .route("/{id}", delete(handler::delete_file_by_id))
        .route("/{id}/name", patch(handler::edit_file_name_by_id))
}
