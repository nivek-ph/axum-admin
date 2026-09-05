mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub(crate) use handler::*;

use crate::{middleware::permission::PermissionRouteExt, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(handler::list_parameters).permission("system:param:list"),
        )
        .route(
            "/",
            post(handler::create_parameter).permission("system:param:create"),
        )
        .route(
            "/by-key",
            get(handler::get_parameter_by_key).permission("system:param:list"),
        )
        .route(
            "/batch",
            delete(handler::delete_parameters).permission("system:param:batch-delete"),
        )
        .route(
            "/{id}",
            get(handler::find_parameter).permission("system:param:get"),
        )
        .route(
            "/{id}",
            put(handler::update_parameter).permission("system:param:update"),
        )
        .route(
            "/{id}",
            delete(handler::delete_parameter).permission("system:param:delete"),
        )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use tower::ServiceExt;

    use super::*;

    fn handler_routes() -> Router<AppState> {
        Router::new()
            .route(
                "/",
                get(handler::list_parameters).post(handler::create_parameter),
            )
            .route("/by-key", get(handler::get_parameter_by_key))
            .route(
                "/{id}",
                get(handler::find_parameter)
                    .put(handler::update_parameter)
                    .delete(handler::delete_parameter),
            )
    }

    async fn json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn parameter_routes_keep_list_detail_key_and_path_body_contract(pool: sqlx::PgPool) {
        let app = handler_routes().with_state(crate::state::tests::test_state(pool).await);
        let response = app
            .clone()
            .oneshot(
                Request::post("/")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"id":999,"name":"Site name","key":"site.name","value":"AVA","desc":"Display name"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::get("/?page=1&pageSize=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json(response).await;
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["pageSize"], 10);
        let id = body["data"]["list"][0]["id"].as_i64().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::put(format!("/{id}"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"id":999,"name":"Site title","key":"site.name","value":"Admin","desc":"Display name"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(Request::get(format!("/{id}")).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let body = json(response).await;
        assert_eq!(body["data"]["id"], id);
        assert_eq!(body["data"]["value"], "Admin");

        let response = app
            .oneshot(
                Request::get("/by-key?key=site.name")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = json(response).await;
        assert_eq!(body["data"]["sysParam"]["id"], id);
    }
}
