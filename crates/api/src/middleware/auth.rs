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
    extractors::{client_ip::ClientIp, user_agent::UserAgent},
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

    request
        .extensions_mut()
        .insert(iam::users::AuthenticatedUser {
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
    use super::*;

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
