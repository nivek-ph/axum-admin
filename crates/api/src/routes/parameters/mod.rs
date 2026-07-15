mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get},
};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handler::get_sys_params_list).post(handler::create_sys_params),
        )
        .route("/by-key", get(handler::get_sys_param))
        .route("/batch", delete(handler::delete_sys_params_by_ids))
        .route(
            "/{id}",
            get(handler::find_sys_params_by_id)
                .put(handler::update_sys_params_by_id)
                .delete(handler::delete_sys_params_by_id),
        )
}
