use axum::{Json, extract::State, http::HeaderMap};

use crate::{
    ApiResponse, AppResult, EmptyData, mappings::LOGIN_REQUIRED,
    middleware::auth::extract_bearer_token, state::AppState,
};

#[utoipa::path(
    post,
    path = "/auth/logout",
    tag = "auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Logout success", body = ApiResponse<EmptyData>),
        (status = 401, description = "Invalid or revoked session")
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    let token = extract_bearer_token(&headers).ok_or(LOGIN_REQUIRED)?;
    state.tokens.revoke(token).await?;

    Ok(Json(ApiResponse::new("OK", "signed out", None)))
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

    use super::*;

    #[tokio::test]
    async fn logout_ends_only_the_current_login_session() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@127.0.0.1/ava")
            .expect("lazy test pool should construct");
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let mut state = crate::state::test_state(pool);
        state.tokens = auth::token::TokenService::new("test-secret", redis);

        let first = state
            .tokens
            .create_session(1, "admin")
            .await
            .expect("first login session should be issued");
        let second = state
            .tokens
            .create_session(1, "admin")
            .await
            .expect("second login session should be issued");
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", first.access_token))
                .expect("authorization header should be valid"),
        );

        let _ = logout(State(state.clone()), headers)
            .await
            .expect("logout should succeed");

        let error = state
            .tokens
            .decode_active(&first.access_token)
            .await
            .expect_err("logged-out session should be rejected");
        assert!(matches!(
            error,
            auth::token::TokenSessionError::SessionInvalid
        ));
        state
            .tokens
            .decode_active(&second.access_token)
            .await
            .expect("other login session should remain active");
        state
            .tokens
            .revoke(&second.access_token)
            .await
            .expect("other login session should be cleaned up");
    }
}
