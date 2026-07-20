mod dto;
mod handler;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
};
use file_storage::files::MAX_UPLOAD_BYTES;
pub(crate) use handler::*;

use crate::state::AppState;

const MAX_UPLOAD_REQUEST_BYTES: usize = MAX_UPLOAD_BYTES + 1024 * 1024;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_file_list_by_query))
        .route("/import-url", post(handler::import_url))
        .route(
            "/upload",
            post(handler::upload_file).layer(DefaultBodyLimit::max(MAX_UPLOAD_REQUEST_BYTES)),
        )
        .route("/{id}", delete(handler::delete_file_by_id))
        .route("/{id}/name", patch(handler::edit_file_name_by_id))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::CONTENT_TYPE},
    };
    use file_storage::files::{FileService, MAX_UPLOAD_BYTES};
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    fn upload_dir() -> PathBuf {
        std::env::temp_dir().join(format!("ava-api-upload-test-{}", Uuid::new_v4()))
    }

    fn multipart_body(parts: Vec<(&str, Vec<u8>)>) -> (String, Body) {
        let boundary = format!("ava-upload-{}", Uuid::new_v4());
        let mut body = Vec::new();
        for (name, bytes) in parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(&bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        (
            format!("multipart/form-data; boundary={boundary}"),
            Body::from(body),
        )
    }

    fn multipart_value_body(bytes: Vec<u8>) -> (String, Body) {
        let boundary = format!("ava-upload-{}", Uuid::new_v4());
        let mut body = Vec::new();
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(b"Content-Disposition: form-data; name=\"note\"\r\n\r\n");
        body.extend_from_slice(&bytes);
        body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
        (
            format!("multipart/form-data; boundary={boundary}"),
            Body::from(body),
        )
    }

    async fn response_json(response: axum::response::Response) -> serde_json::Value {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body should be readable");
        serde_json::from_slice(&body).expect("response should be JSON")
    }

    async fn assert_no_upload_state(pool: &sqlx::PgPool, upload_dir: &Path) {
        let count: i64 = sqlx::query_scalar("select count(*) from uploaded_files")
            .fetch_one(pool)
            .await
            .expect("stored file count should be readable");
        assert_eq!(count, 0);

        let mut entries = tokio::fs::read_dir(upload_dir)
            .await
            .expect("upload directory should exist");
        assert!(
            entries
                .next_entry()
                .await
                .expect("upload directory should be readable")
                .is_none(),
            "failed request should not leave a file"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_route_streams_a_file_to_safe_storage(pool: sqlx::PgPool) {
        let upload_dir = upload_dir();
        let mut state = crate::state::test_state(pool.clone());
        state.files = FileService::new(pool, upload_dir.to_string_lossy());
        let app = routes().with_state(state);
        let (content_type, body) = multipart_body(vec![(
            "../../Quarterly Report.PDF",
            b"quarterly results".to_vec(),
        )]);

        let response = app
            .oneshot(
                Request::post("/upload?tag=finance&category=report")
                    .header(CONTENT_TYPE, content_type)
                    .body(body)
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["code"], "OK");
        assert_eq!(body["data"]["file"]["name"], "../../Quarterly Report.PDF");
        let url = body["data"]["url"]
            .as_str()
            .expect("upload response should contain a URL");
        assert!(url.starts_with("http://127.0.0.1:3000/uploads/"));
        assert!(!url.contains("Quarterly"));
        assert!(!url.contains(".."));
        let stored_name = Path::new(url)
            .file_name()
            .expect("stored URL should contain a file name");
        assert_eq!(
            tokio::fs::read(upload_dir.join(stored_name))
                .await
                .expect("stored file should be readable"),
            b"quarterly results"
        );

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("test upload directory should be removed");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_route_returns_stable_error_and_cleans_up_oversized_file(pool: sqlx::PgPool) {
        let upload_dir = upload_dir();
        let mut state = crate::state::test_state(pool.clone());
        state.files = FileService::new(pool.clone(), upload_dir.to_string_lossy());
        let app = routes().with_state(state);
        let (content_type, body) =
            multipart_body(vec![("large.bin", vec![0; MAX_UPLOAD_BYTES + 1])]);

        let response = app
            .oneshot(
                Request::post("/upload")
                    .header(CONTENT_TYPE, content_type)
                    .body(body)
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = response_json(response).await;
        assert_eq!(body["code"], "FILE_TOO_LARGE");
        assert_no_upload_state(&pool, &upload_dir).await;

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("test upload directory should be removed");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_route_rejects_multiple_files_without_partial_state(pool: sqlx::PgPool) {
        let upload_dir = upload_dir();
        let mut state = crate::state::test_state(pool.clone());
        state.files = FileService::new(pool.clone(), upload_dir.to_string_lossy());
        let app = routes().with_state(state);
        let (content_type, body) = multipart_body(vec![
            ("first.txt", b"first".to_vec()),
            ("second.txt", b"second".to_vec()),
        ]);

        let response = app
            .oneshot(
                Request::post("/upload")
                    .header(CONTENT_TYPE, content_type)
                    .body(body)
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = response_json(response).await;
        assert_eq!(body["code"], "MULTIPLE_FILES_NOT_SUPPORTED");
        assert_no_upload_state(&pool, &upload_dir).await;

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("test upload directory should be removed");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn upload_route_rejects_an_oversized_non_file_part(pool: sqlx::PgPool) {
        let upload_dir = upload_dir();
        let mut state = crate::state::test_state(pool.clone());
        state.files = FileService::new(pool.clone(), upload_dir.to_string_lossy());
        let app = routes().with_state(state);
        let (content_type, body) =
            multipart_value_body(vec![0; MAX_UPLOAD_BYTES + 1024 * 1024 + 1]);

        let response = app
            .oneshot(
                Request::post("/upload")
                    .header(CONTENT_TYPE, content_type)
                    .body(body)
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = response_json(response).await;
        assert_eq!(body["code"], "FILE_TOO_LARGE");

        let stored_count: i64 = sqlx::query_scalar("select count(*) from uploaded_files")
            .fetch_one(&pool)
            .await
            .expect("stored file count should be readable");
        assert_eq!(stored_count, 0);
        assert!(!upload_dir.exists());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn file_management_routes_keep_the_existing_transport_contract(pool: sqlx::PgPool) {
        let app = routes().with_state(crate::state::test_state(pool));
        let response = app
            .clone()
            .oneshot(
                Request::post("/import-url")
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"name":"Remote report","url":"https://example.test/report.pdf","tag":"finance","category":"report"}"#,
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
        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["pageSize"], 10);
        assert_eq!(body["data"]["list"][0]["name"], "Remote report");
        assert_eq!(
            body["data"]["list"][0]["url"],
            "https://example.test/report.pdf"
        );
        let id = body["data"]["list"][0]["id"]
            .as_i64()
            .expect("imported file should have an ID");

        let response = app
            .clone()
            .oneshot(
                Request::patch(format!("/{id}/name"))
                    .header(CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"id":999,"name":"Renamed report"}"#))
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
        let body = response_json(response).await;
        assert_eq!(body["data"]["list"][0]["id"], id);
        assert_eq!(body["data"]["list"][0]["name"], "Renamed report");

        let response = app
            .oneshot(
                Request::delete(format!("/{id}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }
}
