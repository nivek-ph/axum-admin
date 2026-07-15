mod dto;
mod handler;

use axum::{Router, routing::get};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_audit_events))
        .route("/{id}", get(find_audit_event))
}
