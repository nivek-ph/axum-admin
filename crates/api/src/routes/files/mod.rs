mod dto;
mod handler;
mod public;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
};
use file_storage::files::UPLOAD_CHUNK_BYTES;
pub(crate) use handler::*;

use crate::{middleware::permission::permission, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            permission("system:file:list", get(handler::get_file_list_by_query)),
        )
        .route(
            "/import-url",
            permission("system:file:import-url", post(handler::import_url)),
        )
        .route(
            "/uploads",
            permission("system:file:upload", post(handler::start_upload)),
        )
        .route(
            "/uploads/{id}",
            permission("system:file:upload", get(handler::upload_status)),
        )
        .route(
            "/uploads/{id}",
            permission(
                "system:file:upload",
                patch(handler::upload_chunk).layer(DefaultBodyLimit::max(UPLOAD_CHUNK_BYTES)),
            ),
        )
        .route(
            "/uploads/{id}/complete",
            permission("system:file:upload", post(handler::complete_upload)),
        )
        .route(
            "/{id}",
            permission("system:file:delete", delete(handler::delete_file_by_id)),
        )
        .route(
            "/{id}/name",
            permission("system:file:rename", patch(handler::edit_file_name_by_id)),
        )
}

pub(crate) fn public_routes() -> Router<AppState> {
    public::routes()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    fn upload_handler_routes() -> Router<AppState> {
        Router::new()
            .route("/uploads", post(handler::start_upload))
            .route("/uploads/{id}", get(handler::upload_status))
            .route(
                "/uploads/{id}",
                patch(handler::upload_chunk).layer(DefaultBodyLimit::max(UPLOAD_CHUNK_BYTES)),
            )
            .route("/uploads/{id}/complete", post(handler::complete_upload))
    }

    fn upload_dir() -> PathBuf {
        std::env::temp_dir().join(format!("ava-api-upload-test-{}", Uuid::new_v4()))
    }

    async fn test_state(pool: sqlx::PgPool, upload_dir: &Path) -> crate::state::AppState {
        sqlx::query("update sys_storages set root = $1 where is_default")
            .bind(upload_dir.to_string_lossy().as_ref())
            .execute(&pool)
            .await
            .expect("default storage root should update");
        crate::state::tests::test_state(pool).await
    }

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response should be JSON")
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn resumable_upload_continues_from_the_persisted_offset(pool: sqlx::PgPool) {
        let upload_dir = upload_dir();
        let app = upload_handler_routes().with_state(test_state(pool, &upload_dir).await);
        let response = app
            .clone()
            .oneshot(
                Request::post("/uploads")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"name":"report.txt","size":17}"#))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["data"]["chunkSize"], 4 * 1024 * 1024);
        let id = body["data"]["id"]
            .as_str()
            .expect("session ID should be returned");

        let response = app
            .clone()
            .oneshot(
                Request::patch(format!("/uploads/{id}"))
                    .header(CONTENT_TYPE, "application/octet-stream")
                    .header("upload-offset", "0")
                    .body(Body::from("quarterly"))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response_json(response).await["data"]["offset"], 9);

        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/uploads/{id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response_json(response).await["data"]["offset"], 9);

        let response = app
            .clone()
            .oneshot(
                Request::patch(format!("/uploads/{id}"))
                    .header(CONTENT_TYPE, "application/octet-stream")
                    .header("upload-offset", "9")
                    .body(Body::from(" results"))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .oneshot(
                Request::post(format!("/uploads/{id}/complete"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        let stored_name = Path::new(
            body["data"]["file"]["url"]
                .as_str()
                .expect("URL should exist"),
        )
        .file_name()
        .expect("URL should contain object name");
        assert_eq!(
            tokio::fs::read(upload_dir.join(stored_name))
                .await
                .expect("completed object should exist"),
            b"quarterly results"
        );
        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("test upload directory should be removed");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_start_rejects_files_larger_than_one_gib(pool: sqlx::PgPool) {
        let app = upload_handler_routes().with_state(crate::state::tests::test_state(pool).await);
        let response = app
            .oneshot(
                Request::post("/uploads")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(format!(
                        r#"{{"name":"large.bin","size":{}}}"#,
                        file_storage::files::MAX_UPLOAD_BYTES + 1
                    )))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(response_json(response).await["code"], "FILE_TOO_LARGE");
    }
}
