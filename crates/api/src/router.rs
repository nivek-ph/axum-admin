use axum::{Router, http::header, middleware};
use axum_otel::{AxumOtelOnFailure, AxumOtelOnResponse, AxumOtelSpanCreator, Level};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{
    docs::ApiDoc,
    middleware::{auth::require_auth, rate_limit},
    routes,
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    let protected_routes = routes::protected_routes()
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));
    let captcha = rate_limit::apply_captcha(routes::captcha_routes(), &state.redis);
    let api_router = Router::new()
        .merge(routes::public_routes())
        .merge(captcha)
        .merge(protected_routes);
    let api_router = rate_limit::apply_global(api_router, &state.redis);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .merge(routes::public_root_routes())
        .nest("/api", api_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any)
                .expose_headers([header::RETRY_AFTER]),
        )
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(AxumOtelSpanCreator::new().level(Level::INFO))
                        .on_response(AxumOtelOnResponse::new().level(Level::INFO))
                        .on_failure(AxumOtelOnFailure::new().level(Level::ERROR)),
                )
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    #[sqlx::test(migrations = "../../migrations")]
    async fn local_uploads_require_persisted_storage_association(pool: sqlx::PgPool) {
        let upload_dir =
            std::env::temp_dir().join(format!("ava-static-upload-test-{}", Uuid::new_v4()));
        sqlx::query("update sys_storages set root = $1 where is_default")
            .bind(upload_dir.to_string_lossy().as_ref())
            .execute(&pool)
            .await
            .expect("default storage root should update");
        let state = crate::state::tests::test_state(pool).await;
        tokio::fs::create_dir_all(&upload_dir)
            .await
            .expect("untracked object fixture directory should be created");
        tokio::fs::write(upload_dir.join("untracked.txt"), b"untracked")
            .await
            .expect("untracked object should be written");
        let response = super::router(state.clone())
            .oneshot(
                Request::get("/uploads/untracked.txt")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 404);

        let upload = state
            .files
            .start_upload(file_storage::files::StartUpload {
                name: "tracked.txt".to_string(),
                size: 7,
                tag: String::new(),
                category: String::new(),
            })
            .await
            .expect("managed upload session should start");
        state
            .files
            .write_upload_chunk(&upload.id, 0, b"tracked")
            .await
            .expect("managed upload should write");
        let stored = state
            .files
            .complete_upload(&upload.id)
            .await
            .expect("managed upload should finish");
        let object = stored
            .url
            .strip_prefix("/uploads/")
            .expect("local upload should use the local URL prefix");
        let response = super::router(state)
            .oneshot(
                Request::get(format!("/uploads/{object}"))
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 200);

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("upload directory should be removed");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn local_uploads_honor_single_byte_ranges(pool: sqlx::PgPool) {
        let upload_dir =
            std::env::temp_dir().join(format!("ava-range-upload-test-{}", Uuid::new_v4()));
        sqlx::query("update sys_storages set root = $1 where is_default")
            .bind(upload_dir.to_string_lossy().as_ref())
            .execute(&pool)
            .await
            .expect("default storage root should update");
        let state = crate::state::tests::test_state(pool).await;
        let upload = state
            .files
            .start_upload(file_storage::files::StartUpload {
                name: "tracked.txt".to_string(),
                size: 7,
                tag: String::new(),
                category: String::new(),
            })
            .await
            .expect("managed upload session should start");
        state
            .files
            .write_upload_chunk(&upload.id, 0, b"tracked")
            .await
            .expect("managed upload should write");
        let stored = state
            .files
            .complete_upload(&upload.id)
            .await
            .expect("managed upload should finish");
        let object = stored
            .url
            .strip_prefix("/uploads/")
            .expect("local upload should use the local URL prefix");

        let app = super::router(state);
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/uploads/{object}"))
                    .header(header::RANGE, "bytes=2-4")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");

        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes 2-4/7");
        assert_eq!(response.headers()[header::CONTENT_LENGTH], "3");
        assert_eq!(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("range body should be readable"),
            "ack"
        );

        let response = app
            .oneshot(
                Request::get(format!("/uploads/{object}"))
                    .header(header::RANGE, "bytes=7-")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(response.headers()[header::ACCEPT_RANGES], "bytes");
        assert_eq!(response.headers()[header::CONTENT_RANGE], "bytes */7");

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("upload directory should be removed");
    }
}
