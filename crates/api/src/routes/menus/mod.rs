mod dto;
mod handler;

use axum::{Router, routing::get};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/current", get(get_menu))
        .route("/tree", get(get_base_menu_tree))
}
