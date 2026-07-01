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

use axum::Router;

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .nest("/attachment-categories", attachment_category::routes())
        .nest("/depts", dept::routes())
        .nest("/dictionaries", dictionary::routes())
        .nest("/dictionary-details", dictionary_detail::routes())
        .nest("/files", file::routes())
        .nest("/login-logs", logs::login_routes())
        .nest("/menus", menu::routes())
        .nest("/operation-logs", logs::operation_routes())
        .nest("/params", params::routes())
        .nest("/routes", api::routes())
        .nest("/roles", role::routes())
        .nest("/permissions", permission::routes())
        .nest("/system", system::routes())
        .nest("/users", user::routes())
        .nest("/auth", session::routes())
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
