use audit::{AuditActor, AuditContext, AuditSource};
use axum::{
    Extension,
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use tower_http::request_id::RequestId;

use crate::{
    AppResult,
    extractors::{client_ip::ClientIp, current_user::AuthenticatedUser, user_agent::UserAgent},
    mappings::LOGIN_REQUIRED,
    middleware::permission::PermissionGuardContext,
    request_id::request_id_text,
    state::AppState,
};

pub(crate) fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?
        .trim();
    (!token.is_empty()).then_some(token)
}

pub async fn require_auth(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Extension(request_id): Extension<RequestId>,
    UserAgent(agent): UserAgent,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let headers = request.headers();
    let token = extract_bearer_token(headers).ok_or(LOGIN_REQUIRED)?;
    let claims = state.tokens.decode_active(token).await?;
    let audit_context = AuditContext {
        req_id: request_id_text(&request_id),
        actor: AuditActor {
            id: Some(claims.user_id),
            label: claims.username.clone(),
        },
        source: AuditSource {
            ip,
            user_agent: agent,
        },
    };
    state.access.require_active_user(claims.user_id).await?;

    request
        .extensions_mut()
        .insert(AuthenticatedUser { id: claims.user_id });
    request.extensions_mut().insert(audit_context);
    request.extensions_mut().insert(PermissionGuardContext::new(
        state.access.clone(),
        state.audits.clone(),
    ));
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use auth::token::TokenService;
    use axum::{
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{Method, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;

    async fn token_service() -> TokenService {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        TokenService::new(
            "test-secret",
            client.get_multiplexed_async_connection().await.unwrap(),
        )
    }

    async fn authenticated_state(
        pool: sqlx::PgPool,
        user_id: i64,
        username: &str,
    ) -> (AppState, TokenService, String) {
        let tokens = token_service().await;
        let session = tokens.create_session(user_id, username).await.unwrap();
        let mut state = crate::state::tests::test_state(pool).await;
        state.tokens = tokens.clone();
        (state, tokens, session.access_token)
    }

    async fn protected_response(
        app: &axum::Router,
        access_token: Option<&str>,
        method: Method,
        path: &str,
    ) -> (StatusCode, Value) {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo("127.0.0.1:3000".parse::<SocketAddr>().unwrap()));
        if let Some(access_token) = access_token {
            request = request.header(AUTHORIZATION, format!("Bearer {access_token}"));
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).unwrap()
        };
        (status, body)
    }

    async fn insert_user(pool: &sqlx::PgPool, user_id: i64, username: &str, enabled: bool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id
            ) values ($1, $2, $2, 'hash', $2, '', 'dashboard', $3, 1)
            "#,
        )
        .bind(user_id)
        .bind(username)
        .bind(enabled)
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_user_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        let (state, tokens, access_token) = authenticated_state(pool, 99, "missing-user").await;
        let app = crate::router::router(state);

        let (status, body) =
            protected_response(&app, Some(&access_token), Method::GET, "/api/users/me").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED, "response body: {body}");
        assert_eq!(body["code"], "SESSION_INVALID");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_user_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        insert_user(&pool, 100, "disabled-user", false).await;
        let (state, tokens, access_token) = authenticated_state(pool, 100, "disabled-user").await;
        let app = crate::router::router(state);

        let (status, body) =
            protected_response(&app, Some(&access_token), Method::GET, "/api/users/me").await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "USER_DISABLED");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn authentication_wraps_method_not_allowed_but_not_unknown_paths(pool: sqlx::PgPool) {
        insert_user(&pool, 101, "layer-order-user", true).await;
        let (state, tokens, access_token) =
            authenticated_state(pool, 101, "layer-order-user").await;
        let app = crate::router::router(state);

        let (status, body) = protected_response(&app, None, Method::POST, "/api/users/me").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "LOGIN_REQUIRED");

        let (status, _) =
            protected_response(&app, Some(&access_token), Method::POST, "/api/users/me").await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

        let (status, _) = protected_response(&app, None, Method::GET, "/api/not-a-route").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn zero_role_user_can_use_self_service_but_not_management_routes(pool: sqlx::PgPool) {
        insert_user(&pool, 102, "zero-role-user", true).await;
        let (state, tokens, access_token) =
            authenticated_state(pool.clone(), 102, "zero-role-user").await;
        let app = crate::router::router(state);

        let (status, body) =
            protected_response(&app, Some(&access_token), Method::GET, "/api/users/me").await;
        assert_eq!(status, StatusCode::OK, "response body: {body}");

        let (status, body) =
            protected_response(&app, Some(&access_token), Method::GET, "/api/users").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "PERMISSION_DENIED");

        let path = sqlx::query_scalar::<_, String>(
            "select resource_id from sys_audit_events where action = 'auth.access_denied'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(path, "/api/users");

        sqlx::query("drop table sys_audit_events")
            .execute(&pool)
            .await
            .unwrap();
        let (status, body) =
            protected_response(&app, Some(&access_token), Method::HEAD, "/api/users").await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert!(body.is_null(), "HEAD response body must be empty");
        tokens.revoke(&access_token).await.unwrap();
    }
}
