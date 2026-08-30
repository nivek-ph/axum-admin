use axum::Router;

use super::{
    audit, auth, departments, dictionaries, files, menus, parameters, roles, storages, users,
};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .nest("/depts", departments::routes())
        .nest("/dictionaries", dictionaries::routes())
        .nest("/files", files::routes())
        .nest("/menus", menus::routes())
        .nest("/params", parameters::routes())
        .nest("/roles", roles::routes())
        .nest("/storages", storages::routes())
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
            .route("/{id}/access", get(|| ok_marker("roles:access")));

        Router::new().nest("/roles", role_routes)
    }

    #[tokio::test]
    async fn unified_role_access_route_stays_reachable() {
        let response = role_shape_router()
            .oneshot(
                Request::builder()
                    .uri("/roles/7/access")
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
        assert_eq!(body, "roles:access");
    }
}
