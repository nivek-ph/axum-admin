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

        let mut upload = state
            .files
            .begin_upload("tracked.txt", "", "")
            .await
            .expect("managed upload should start");
        upload
            .write_chunk(b"tracked")
            .await
            .expect("managed upload should write");
        let stored = upload.finish().await.expect("managed upload should finish");
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
}
