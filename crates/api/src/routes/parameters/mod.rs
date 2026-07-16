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

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use tower::ServiceExt;

    use super::*;

    async fn json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn parameter_routes_keep_list_detail_key_and_path_body_contract(pool: sqlx::PgPool) {
        let app = routes().with_state(crate::state::test_state(pool));
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
