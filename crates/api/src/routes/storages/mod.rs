mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};
pub(crate) use handler::*;

use crate::{middleware::permission::PermissionRouteExt, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::list).permission("system:storage:list"))
        .route(
            "/",
            post(handler::create).permission("system:storage:create"),
        )
        .route(
            "/{id}",
            get(handler::find).permission("system:storage:list"),
        )
        .route(
            "/{id}",
            put(handler::update).permission("system:storage:update"),
        )
        .route(
            "/{id}",
            delete(handler::delete).permission("system:storage:delete"),
        )
        .route(
            "/{id}/status",
            patch(handler::set_status).permission("system:storage:update-status"),
        )
        .route(
            "/{id}/default",
            put(handler::set_default).permission("system:storage:set-default"),
        )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;

    fn handler_routes() -> Router<AppState> {
        Router::new()
            .route("/", get(handler::list).post(handler::create))
            .route("/{id}/status", patch(handler::set_status))
    }

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&bytes).expect("response body should be JSON")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn routes_create_list_and_protect_the_default_without_exposing_secrets(
        pool: sqlx::PgPool,
    ) {
        let app = handler_routes().with_state(crate::state::tests::test_state(pool).await);
        let root = std::env::temp_dir().join(format!("ava-api-storage-{}", uuid::Uuid::new_v4()));
        let response = app
            .clone()
            .oneshot(
                Request::post("/")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({
                            "name": "Archive storage",
                            "code": "archive",
                            "driver": "local",
                            "root": root,
                            "enabled": true,
                            "sort": 20,
                            "description": "Archive files"
                        })
                        .to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["data"]["code"], "archive");
        assert!(body["data"].get("accessKey").is_none());
        assert!(body["data"].get("secretKey").is_none());

        let response = app
            .clone()
            .oneshot(Request::get("/").body(Body::empty()).unwrap())
            .await
            .expect("router should respond");
        let body = json_body(response).await;
        assert_eq!(body["data"]["list"].as_array().unwrap().len(), 2);
        let default_id = body["data"]["list"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["isDefault"] == true)
            .and_then(|item| item["id"].as_i64())
            .expect("default configuration should exist");

        let response = app
            .oneshot(
                Request::patch(format!("/{default_id}/status"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"enabled":false}"#))
                    .unwrap(),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(
            json_body(response).await["code"],
            "DEFAULT_STORAGE_PROTECTED"
        );

        assert!(
            !root.exists(),
            "creating a local storage configuration should not prepare its root"
        );
    }
}
