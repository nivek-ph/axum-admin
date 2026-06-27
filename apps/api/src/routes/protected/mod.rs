pub mod api;
pub mod attachment_category;
pub mod dept;
pub mod dictionary;
pub mod dictionary_detail;
pub mod file;
pub mod logs;
pub mod menu;
pub mod params;
pub mod permission;
pub mod role;
pub mod session;
pub mod system;
pub mod user;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post, put},
};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .nest("/roles", role::routes())
        .nest("/permissions", permission::routes())
        .route(
            "/routes",
            get(api::get_api_list_by_query).post(api::create_api),
        )
        .route("/routes/all", get(api::get_all_apis))
        .route("/routes/groups", get(api::get_api_groups))
        .route("/routes/casbin/refresh", post(api::fresh_casbin))
        .route("/routes/authority", get(api::get_authority_apis))
        .route("/routes/role-matrix", get(api::get_api_role_matrix))
        .route(
            "/routes/roles",
            get(api::get_api_roles).put(api::set_api_roles),
        )
        .route(
            "/routes/{id}",
            get(api::get_api_by_path_id)
                .put(api::update_api_by_id)
                .delete(api::delete_api_by_path_id),
        )
        .route("/routes/batch", delete(api::delete_apis_by_ids))
        .route(
            "/attachment-categories",
            get(attachment_category::get_category_list).post(attachment_category::add_category),
        )
        .route(
            "/attachment-categories/{id}",
            delete(attachment_category::delete_category_by_id),
        )
        .route(
            "/params",
            get(params::get_sys_params_list).post(params::create_sys_params),
        )
        .route("/params/by-key", get(params::get_sys_param))
        .route("/params/batch", delete(params::delete_sys_params_by_ids))
        .route(
            "/params/{id}",
            get(params::find_sys_params_by_id)
                .put(params::update_sys_params_by_id)
                .delete(params::delete_sys_params_by_id),
        )
        .route("/depts", get(dept::get_dept_tree).post(dept::create_dept))
        .route(
            "/depts/{id}",
            get(dept::find_dept_by_id)
                .put(dept::update_dept_by_id)
                .delete(dept::delete_dept_by_id),
        )
        .route(
            "/dictionaries",
            get(dictionary::get_sys_dictionary_list).post(dictionary::create_sys_dictionary),
        )
        .route(
            "/dictionaries/import",
            post(dictionary::import_sys_dictionary),
        )
        .route(
            "/dictionaries/{id}",
            get(dictionary::find_sys_dictionary_by_id)
                .put(dictionary::update_sys_dictionary_by_id)
                .delete(dictionary::delete_sys_dictionary_by_id),
        )
        .route(
            "/dictionaries/{id}/export",
            get(dictionary::export_sys_dictionary_by_id),
        )
        .route(
            "/dictionaries/{id}/details/tree",
            get(dictionary_detail::get_dictionary_tree_list_by_id),
        )
        .route(
            "/dictionary-details",
            post(dictionary_detail::create_sys_dictionary_detail),
        )
        .route(
            "/dictionary-details/tree-by-type",
            get(dictionary_detail::get_dictionary_tree_list_by_type),
        )
        .route(
            "/dictionary-details/by-parent",
            get(dictionary_detail::get_dictionary_details_by_parent),
        )
        .route(
            "/dictionary-details/{id}",
            get(dictionary_detail::find_sys_dictionary_detail_by_id)
                .put(dictionary_detail::update_sys_dictionary_detail_by_id)
                .delete(dictionary_detail::delete_sys_dictionary_detail_by_id),
        )
        .route(
            "/dictionary-details/{id}/path",
            get(dictionary_detail::get_dictionary_path_by_id),
        )
        .route(
            "/login-logs",
            get(logs::get_login_log_list).delete(logs::delete_login_log_by_ids),
        )
        .route(
            "/login-logs/{id}",
            get(logs::find_login_log_by_id).delete(logs::delete_login_log_by_id),
        )
        .route(
            "/operation-logs",
            get(logs::get_operation_log_list).delete(logs::delete_operation_log_by_ids),
        )
        .route(
            "/operation-logs/{id}",
            delete(logs::delete_operation_log_by_id),
        )
        .route(
            "/system/config",
            get(system::get_system_config).put(system::set_system_config),
        )
        .route("/system/server-info", get(system::get_server_info))
        .route("/system/reload", post(system::reload_system))
        .route(
            "/users/me",
            get(user::get_user_info).put(user::set_self_info),
        )
        .route("/users/me/password", put(user::change_password))
        .route("/users/me/settings", put(user::set_self_setting))
        .route(
            "/users",
            get(user::get_user_list_by_query).post(user::admin_register),
        )
        .route(
            "/users/{id}",
            put(user::set_user_info_by_id).delete(user::delete_user_by_id),
        )
        .route(
            "/users/{id}/password/reset",
            post(user::reset_password_by_id),
        )
        .route("/users/{id}/roles", put(user::set_user_roles_by_id))
        .route("/menus/current", get(menu::get_menu))
        .route("/menus", get(menu::get_menu_list).post(menu::add_base_menu))
        .route("/menus/tree", get(menu::get_base_menu_tree))
        .route(
            "/menus/{id}",
            get(menu::get_base_menu_by_path_id)
                .put(menu::update_base_menu_by_id)
                .delete(menu::delete_base_menu_by_id),
        )
        .route(
            "/menus/{id}/roles",
            get(menu::get_menu_roles_by_id).put(menu::set_menu_roles_by_id),
        )
        .route("/files", get(file::get_file_list_by_query))
        .route("/files/import-url", post(file::import_url))
        .route(
            "/files/upload",
            post(file::upload_file).layer(DefaultBodyLimit::max(20 * 1024 * 1024)),
        )
        .route("/files/{id}", delete(file::delete_file_by_id))
        .route("/files/{id}/name", patch(file::edit_file_name_by_id))
        .route("/auth/logout", post(session::logout))
}

#[cfg(test)]
mod tests {
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        response::IntoResponse,
        routing::{get, put},
    };
    use tower::ServiceExt;

    async fn ok_marker(marker: &'static str) -> impl IntoResponse {
        marker
    }

    fn role_shape_router() -> Router {
        let role_routes = Router::new()
            .route("/", get(|| ok_marker("roles:list")))
            .route("/{id}", put(|| ok_marker("roles:update")))
            .route("/{id}/permissions", get(|| ok_marker("roles:permissions")))
            .route("/{id}/users", get(|| ok_marker("roles:users")));

        Router::new().nest("/roles", role_routes)
    }

    #[tokio::test]
    async fn role_permission_assignment_route_stays_reachable() {
        let response = role_shape_router()
            .oneshot(
                Request::builder()
                    .uri("/roles/7/permissions")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let body = String::from_utf8(bytes.to_vec()).expect("body should be utf8");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "roles:permissions");
    }

    #[tokio::test]
    async fn role_user_assignment_route_uses_mature_role_endpoint() {
        let response = role_shape_router()
            .oneshot(
                Request::builder()
                    .uri("/roles/7/users")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body should be readable");
        let body = String::from_utf8(bytes.to_vec()).expect("body should be utf8");

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "roles:users");
    }
}
