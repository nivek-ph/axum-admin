use axum::Router;

use super::{audit, auth, departments, dictionaries, files, menus, parameters, roles, users};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .nest("/depts", departments::routes())
        .nest("/dictionaries", dictionaries::routes())
        .nest("/files", files::routes())
        .nest("/menus", menus::routes())
        .nest("/params", parameters::routes())
        .nest("/roles", roles::routes())
        .nest("/users", users::routes())
        .merge(auth::protected_routes())
        .merge(audit::routes())
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
            .route("/{id}/menus", get(|| ok_marker("roles:menus")))
            .route("/{id}/users", get(|| ok_marker("roles:users")));

        Router::new().nest("/roles", role_routes)
    }

    #[tokio::test]
    async fn role_menu_assignment_route_stays_reachable() {
        let response = role_shape_router()
            .oneshot(
                Request::builder()
                    .uri("/roles/7/menus")
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
        assert_eq!(body, "roles:menus");
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
