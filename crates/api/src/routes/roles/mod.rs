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
            "/{id}/permissions",
            get(handler::get_role_permissions).put(handler::set_role_permissions),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_routes_exclude_department_scope_and_reverse_membership() {
        let _ = routes();
    }
}
