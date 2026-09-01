mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub(crate) use handler::*;

use crate::{middleware::permission::permission, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            permission(
                "system:dictionary:list",
                get(handler::get_sys_dictionary_list),
            ),
        )
        .route(
            "/",
            permission(
                "system:dictionary:create",
                post(handler::create_sys_dictionary),
            ),
        )
        .route(
            "/import",
            permission(
                "system:dictionary:import",
                post(handler::import_sys_dictionary),
            ),
        )
        .route(
            "/by-type/{dictionary_type}/tree",
            permission(
                "system:dictionary:list",
                get(handler::get_dictionary_tree_by_type),
            ),
        )
        .route(
            "/{id}",
            permission(
                "system:dictionary:list",
                get(handler::find_sys_dictionary_by_id),
            ),
        )
        .route(
            "/{id}",
            permission(
                "system:dictionary:update",
                put(handler::update_sys_dictionary_by_id),
            ),
        )
        .route(
            "/{id}",
            permission(
                "system:dictionary:delete",
                delete(handler::delete_sys_dictionary_by_id),
            ),
        )
        .route(
            "/{id}/export",
            permission(
                "system:dictionary:export",
                get(handler::export_sys_dictionary_by_id),
            ),
        )
        .route(
            "/{id}/tree",
            permission("system:dictionary:list", get(handler::get_dictionary_tree)),
        )
        .route(
            "/{id}/tree",
            permission(
                "system:dictionary-detail:create",
                post(handler::create_dictionary_tree_node),
            ),
        )
        .route(
            "/{id}/tree/{node_id}",
            permission(
                "system:dictionary:list",
                get(handler::find_dictionary_tree_node),
            ),
        )
        .route(
            "/{id}/tree/{node_id}",
            permission(
                "system:dictionary-detail:update",
                put(handler::update_dictionary_tree_node),
            ),
        )
        .route(
            "/{id}/tree/{node_id}",
            permission(
                "system:dictionary-detail:delete",
                delete(handler::delete_dictionary_tree_node),
            ),
        )
        .route(
            "/{id}/tree/{node_id}/children",
            permission(
                "system:dictionary:list",
                get(handler::get_dictionary_tree_node_children),
            ),
        )
        .route(
            "/{id}/tree/{node_id}/path",
            permission(
                "system:dictionary:list",
                get(handler::get_dictionary_tree_node_path),
            ),
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
                get(handler::get_sys_dictionary_list).post(handler::create_sys_dictionary),
            )
            .route("/import", post(handler::import_sys_dictionary))
            .route(
                "/{id}",
                get(handler::find_sys_dictionary_by_id).put(handler::update_sys_dictionary_by_id),
            )
            .route(
                "/{id}/tree",
                get(handler::get_dictionary_tree).post(handler::create_dictionary_tree_node),
            )
            .route("/{id}/export", get(handler::export_sys_dictionary_by_id))
    }

    async fn json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response should be JSON")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn dictionary_routes_keep_transport_shape_and_derive_tree_fields(pool: sqlx::PgPool) {
        let app = handler_routes().with_state(crate::state::tests::test_state(pool).await);
        let response = app
            .clone()
            .oneshot(
                Request::post("/")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"id":999,"name":"Status","type":"status","status":true,"desc":"Status values","parentId":null}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::get("/?page=1&pageSize=10")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let body = json(response).await;
        assert_eq!(body["data"][0]["type"], "status");
        let id = body["data"][0]["id"]
            .as_i64()
            .expect("dictionary should have an ID");

        let response = app
            .clone()
            .oneshot(
                Request::put(format!("/{id}"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"id":999,"name":"Updated status","type":"status","status":true,"desc":"Status values","parentId":null}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::post(format!("/{id}/tree"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"label":"Enabled","value":"enabled","extend":"","status":true,"sort":1,"parentId":null}"#,
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{id}/tree"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let body = json(response).await;
        assert_eq!(body["data"]["list"][0]["sysDictionaryId"], id);
        assert_eq!(body["data"]["list"][0]["level"], 0);

        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/{id}/export"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let body = json(response).await;
        assert_eq!(body["data"]["dictionary"]["id"], id);
        assert_eq!(body["data"]["dictionary"]["name"], "Updated status");
        assert_eq!(body["data"]["details"][0]["label"], "Enabled");

        let mut import_data = body["data"].clone();
        import_data["dictionary"]["name"] = serde_json::json!("Imported status");
        import_data["dictionary"]["type"] = serde_json::json!("imported_status");
        import_data
            .as_object_mut()
            .expect("export data should be an object")
            .remove("details");
        let import_body = serde_json::json!({ "json": import_data.to_string() }).to_string();
        let response = app
            .clone()
            .oneshot(
                Request::post("/import")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(import_body))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let body = json(response).await;
        assert_eq!(body["message"], "imported");

        let response = app
            .oneshot(
                Request::get("/9223372036854775807")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        let body = json(response).await;
        assert_eq!(body["data"]["resysDictionary"], serde_json::json!({}));
    }
}
