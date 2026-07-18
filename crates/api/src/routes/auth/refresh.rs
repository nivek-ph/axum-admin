use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{ApiResponse, AppResult, state::AppState};

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[utoipa::path(
    post,
    path = "/auth/refresh",
    tag = "auth",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Refresh success", body = ApiResponse<RefreshResponse>),
        (status = 401, description = "Invalid refresh token"),
        (status = 403, description = "User disabled"),
        (status = 503, description = "Authorization unavailable")
    )
)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(payload): Json<RefreshRequest>,
) -> AppResult<Json<ApiResponse<RefreshResponse>>> {
    let grant = state.tokens.inspect_refresh(&payload.refresh_token).await?;
    let identity = match state.users.refresh_identity(grant.user_id()).await {
        Ok(identity) => identity,
        Err(
            error @ (iam::users::RefreshIdentityError::NotFound
            | iam::users::RefreshIdentityError::Disabled),
        ) => {
            if let Err(revoke_error) = state.tokens.revoke_refresh_grant(&grant).await {
                tracing::error!(
                    error = ?revoke_error,
                    user_id = grant.user_id(),
                    "failed to revoke refresh grant after identity check failure"
                );
            }
            return Err(error.into());
        }
        Err(error @ iam::users::RefreshIdentityError::Database(_)) => return Err(error.into()),
    };
    let pair = state
        .tokens
        .rotate_refresh(grant, &identity.username)
        .await?;
    Ok(Json(ApiResponse::ok(RefreshResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    })))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Method, Request, StatusCode, header},
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;

    async fn refresh_request(state: AppState, refresh_token: &str) -> (StatusCode, Value) {
        let response = crate::router(state)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/auth/refresh")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "refreshToken": refresh_token }).to_string(),
                    ))
                    .expect("request should build"),
            )
            .await
            .expect("router should produce a response");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        let body = serde_json::from_slice(&body).expect("response should be JSON");
        (status, body)
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn refresh_route_rotates_tokens_and_revalidates_user_state(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values
                (401, 'refresh-route-enabled', 'enabled-user', 'hash', 'Enabled', '', 'dashboard', true, 1, false),
                (402, 'refresh-route-disabled', 'disabled-user', 'hash', 'Disabled', '', 'dashboard', false, 1, false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let tokens = auth::token::TokenService::new("test-secret", redis);
        let enabled = tokens
            .create_session(401, "enabled-user")
            .await
            .expect("enabled session should be issued");
        let disabled = tokens
            .create_session(402, "disabled-user")
            .await
            .expect("disabled session should be issued");
        let missing = tokens
            .create_session(999, "missing-user")
            .await
            .expect("missing-user session should be issued");
        let mut state = crate::state::test_state(pool);
        state.tokens = tokens.clone();

        let (status, body) = refresh_request(state.clone(), &enabled.refresh_token).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], "OK");
        assert!(body["data"]["accessToken"].is_string());
        assert!(body["data"]["refreshToken"].is_string());
        assert_eq!(body["data"].as_object().map(|data| data.len()), Some(2));
        let rotated_access = body["data"]["accessToken"]
            .as_str()
            .expect("rotated access token should be a string");

        let (status, body) = refresh_request(state.clone(), &enabled.refresh_token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "REFRESH_TOKEN_INVALID");

        let (status, body) = refresh_request(state.clone(), &disabled.refresh_token).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "USER_DISABLED");
        assert!(matches!(
            tokens
                .inspect_refresh(&disabled.refresh_token)
                .await
                .expect_err("disabled user session should be removed"),
            auth::token::RefreshError::SessionInvalid
        ));

        let (status, body) = refresh_request(state, &missing.refresh_token).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "SESSION_INVALID");
        assert!(matches!(
            tokens
                .inspect_refresh(&missing.refresh_token)
                .await
                .expect_err("missing user session should be removed"),
            auth::token::RefreshError::SessionInvalid
        ));

        tokens
            .revoke(rotated_access)
            .await
            .expect("rotated session should be cleaned up");
        tokens
            .revoke(&disabled.access_token)
            .await
            .expect("disabled session should be cleaned up");
        tokens
            .revoke(&missing.access_token)
            .await
            .expect("missing-user session should be cleaned up");
    }
}
