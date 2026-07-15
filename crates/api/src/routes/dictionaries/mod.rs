mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, post},
};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handler::get_sys_dictionary_list).post(handler::create_sys_dictionary),
        )
        .route("/import", post(handler::import_sys_dictionary))
        .route(
            "/by-type/{dictionary_type}/tree",
            get(handler::get_dictionary_tree_by_type),
        )
        .route(
            "/{id}",
            get(handler::find_sys_dictionary_by_id)
                .put(handler::update_sys_dictionary_by_id)
                .delete(handler::delete_sys_dictionary_by_id),
        )
        .route("/{id}/export", get(handler::export_sys_dictionary_by_id))
        .route(
            "/{id}/tree",
            get(handler::get_dictionary_tree).post(handler::create_dictionary_tree_node),
        )
        .route(
            "/{id}/tree/{node_id}",
            get(handler::find_dictionary_tree_node)
                .put(handler::update_dictionary_tree_node)
                .delete(handler::delete_dictionary_tree_node),
        )
        .route(
            "/{id}/tree/{node_id}/children",
            get(handler::get_dictionary_tree_node_children),
        )
        .route(
            "/{id}/tree/{node_id}/path",
            get(handler::get_dictionary_tree_node_path),
        )
}
