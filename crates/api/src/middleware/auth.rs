use audit::{
    AuditAction, AuditActor, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult,
    AuditSource,
};
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
    let method = request.method().as_str();
    let path = request.uri().path();
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
    let context = match state.access.evaluate(claims.user_id, method, path).await {
        Ok(context) => context,
        Err(iam::access::AccessEvaluationError::PermissionDenied { path }) => {
            record_access_denied(&state.audits, &audit_context, path.clone()).await;
            return Err(iam::access::AccessEvaluationError::PermissionDenied { path }.into());
        }
        Err(error) => return Err(error.into()),
    };

    request.extensions_mut().insert(AuthenticatedUser {
        id: claims.user_id,
        data_scope: context.data_scope(),
    });
    request.extensions_mut().insert(audit_context);
    Ok(next.run(request).await)
}

async fn record_access_denied(audits: &audit::AuditService, context: &AuditContext, path: String) {
    audits
        .record_best_effort(AuditEvent {
            req_id: context.req_id.clone(),
            actor: context.actor.clone(),
            action: AuditAction::AccessDenied,
            resource: AuditResource::Route(path),
            result: AuditResult::Denied,
            reason_code: Some(AuditReason::PermissionDenied),
            source: context.source.clone(),
            changes: Vec::new(),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use auth::token::TokenService;
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
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
        let mut state = crate::state::test_state(pool);
        state.tokens = tokens.clone();
        (state, tokens, session.access_token)
    }

    async fn protected_response(
        state: AppState,
        access_token: &str,
        path: &str,
    ) -> (StatusCode, Value) {
        let response = crate::router::router(state)
            .oneshot(
                Request::get(path)
                    .header(AUTHORIZATION, format!("Bearer {access_token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    async fn insert_user(pool: &sqlx::PgPool, user_id: i64, username: &str, enabled: bool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values ($1, $2, $2, 'hash', $2, '', 'dashboard', $3, 1, false)
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
    async fn missing_protected_route_binding_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        insert_user(&pool, 98, "unbound-user", true).await;
        let (state, tokens, access_token) = authenticated_state(pool, 98, "unbound-user").await;

        let (status, body) = protected_response(state, &access_token, "/api/roles").await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "AUTHORIZATION_CONFIG_INVALID");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_user_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        let (state, tokens, access_token) = authenticated_state(pool, 99, "missing-user").await;

        let (status, body) = protected_response(state, &access_token, "/api/users/me").await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body["code"], "SESSION_INVALID");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_user_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        insert_user(&pool, 100, "disabled-user", false).await;
        let (state, tokens, access_token) = authenticated_state(pool, 100, "disabled-user").await;

        let (status, body) = protected_response(state, &access_token, "/api/users/me").await;

        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "USER_DISABLED");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ambiguous_protected_route_binding_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        insert_user(&pool, 101, "ambiguous-user", true).await;
        sqlx::query(
            r#"
            insert into sys_menu_apis (menu_id, method, path_pattern)
            values (1106, 'GET', '/api/{area}/{id}/permissions')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let authorization = iam::authorization::Authorization::load(pool.clone())
            .await
            .unwrap();
        let (access, _) = iam::load_access_and_menus(pool.clone(), authorization)
            .await
            .unwrap();
        let (mut state, tokens, access_token) =
            authenticated_state(pool, 101, "ambiguous-user").await;
        state.access = access;

        let (status, body) =
            protected_response(state, &access_token, "/api/roles/2/permissions").await;

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "AUTHORIZATION_CONFIG_INVALID");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn unavailable_authorization_keeps_the_stable_http_contract(pool: sqlx::PgPool) {
        insert_user(&pool, 102, "unavailable-user", true).await;
        let unavailable_pool =
            sqlx::PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1/ava").unwrap();
        unavailable_pool.close().await;
        let authorization = iam::authorization::Authorization::new(unavailable_pool);
        let (access, _) = iam::load_access_and_menus(pool.clone(), authorization)
            .await
            .unwrap();
        let (mut state, tokens, access_token) =
            authenticated_state(pool, 102, "unavailable-user").await;
        state.access = access;

        let (status, body) = protected_response(state, &access_token, "/api/roles").await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body["code"], "AUTHORIZATION_UNAVAILABLE");
        tokens.revoke(&access_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn permission_denial_records_the_expected_audit_classification(pool: sqlx::PgPool) {
        record_access_denied(
            &audit::AuditService::new(pool.clone()),
            &AuditContext {
                req_id: "req-access-denied-1".to_string(),
                actor: AuditActor {
                    id: Some(1),
                    label: "admin".to_string(),
                },
                source: AuditSource {
                    ip: "127.0.0.1".to_string(),
                    user_agent: "auth-middleware-test".to_string(),
                },
            },
            "/api/users".to_string(),
        )
        .await;

        let event: (String, String, String, String, String) = sqlx::query_as(
            r#"
            select req_id, action, resource_id, result, reason_code
            from sys_audit_events
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            event,
            (
                "req-access-denied-1".to_string(),
                "auth.access_denied".to_string(),
                "/api/users".to_string(),
                "denied".to_string(),
                "permission_denied".to_string(),
            )
        );
    }
}
