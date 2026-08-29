use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    middleware,
    response::{IntoResponse, Response},
    routing::get,
};
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
    AppResult,
    docs::ApiDoc,
    middleware::{auth::require_auth, rate_limit},
    routes,
    state::AppState,
};

pub fn router(state: AppState) -> Router {
    let captcha = rate_limit::apply_captcha(routes::captcha_routes(), &state.redis);
    let api_router = Router::new()
        .merge(routes::public_routes())
        .merge(captcha)
        .merge(
            routes::protected_routes()
                .route_layer(middleware::from_fn_with_state(state.clone(), require_auth)),
        );
    let api_router = rate_limit::apply_global(api_router, &state.redis);

    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/uploads/{*object}", get(serve_local_upload))
        .nest("/api", api_router)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_headers(Any)
                .allow_methods(Any),
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

async fn serve_local_upload(
    State(state): State<AppState>,
    Path(object): Path<String>,
) -> AppResult<Response> {
    let Some(bytes) = state.files.read_local_object(&object).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let content_type = mime_guess::from_path(&object).first_or_octet_stream();
    Ok(([(header::CONTENT_TYPE, content_type.as_ref())], bytes).into_response())
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request};
    use file_storage::files::{FileService, FileStorage};
    use tower::ServiceExt;
    use uuid::Uuid;

    #[sqlx::test(migrations = "../../migrations")]
    async fn local_uploads_are_served_only_for_the_local_adapter(pool: sqlx::PgPool) {
        let upload_dir =
            std::env::temp_dir().join(format!("ava-static-upload-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&upload_dir)
            .await
            .expect("upload directory should be created");
        tokio::fs::write(upload_dir.join("local.txt"), b"local")
            .await
            .expect("local upload should be written");
        let mut local_state = crate::state::tests::test_state(pool.clone()).await;
        local_state.files = FileService::new(pool.clone(), upload_dir.to_string_lossy());
        let response = super::router(local_state)
            .oneshot(
                Request::get("/uploads/local.txt")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 200);

        let mut s3_state = crate::state::tests::test_state(pool.clone()).await;
        s3_state.files = FileService::from_config(
            pool,
            &FileStorage {
                driver: "s3".to_string(),
                s3_bucket: "files".to_string(),
                s3_region: "us-east-1".to_string(),
                s3_public_base_url: "https://cdn.example.test".to_string(),
                ..FileStorage::default()
            },
        )
        .expect("S3 adapter should initialize from valid configuration");
        let response = super::router(s3_state)
            .oneshot(
                Request::get("/uploads/local.txt")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), 404);

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("upload directory should be removed");
    }
}
