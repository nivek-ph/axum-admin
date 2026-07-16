use audit::{
    AuditAction, AuditActor, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult,
    AuditSource,
};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

use crate::{
    AppResult,
    mappings::{LOGIN_REQUIRED, PERMISSION_DENIED},
    state::AppState,
};
const X_FORWARDED_FOR: &str = "x-forwarded-for";
const USER_AGENT: &str = "user-agent";

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
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let method = request.method().as_str().to_uppercase();
    let path = permission_registry_path(request.uri().path());
    let headers = request.headers();
    let ip = headers
        .get(X_FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let token = extract_bearer_token(headers).ok_or(LOGIN_REQUIRED)?;
    let claims = state.tokens.decode_active(token).await?;
    let audit_context = AuditContext {
        actor: AuditActor {
            id: Some(claims.user_id),
            label: claims.username.clone(),
        },
        source: AuditSource {
            ip,
            user_agent: agent,
        },
    };
    let snapshot = state.access.snapshot(claims.user_id).await?;

    if !is_self_service_endpoint(&method, &path) {
        let menu_id = state.access.required_menu(&method, &path)?;
        if !snapshot.allows_menu(menu_id) {
            record_access_denied(&state.audits, &audit_context, path).await;
            return Err(PERMISSION_DENIED.into());
        }
    }

    let user_id = claims.user_id;
    request
        .extensions_mut()
        .insert(iam::users::AuthenticatedUser {
            id: user_id,
            data_scope: snapshot.data_scope,
        });
    request.extensions_mut().insert(audit_context);
    Ok(next.run(request).await)
}

async fn record_access_denied(audits: &audit::AuditService, context: &AuditContext, path: String) {
    audits
        .record_best_effort(AuditEvent {
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

fn is_self_service_endpoint(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/api/users/me")
            | ("PUT", "/api/users/me")
            | ("PUT", "/api/users/me/password")
            | ("PUT", "/api/users/me/settings")
            | ("GET", "/api/menus/current")
            | ("POST", "/api/auth/logout")
    )
}

fn permission_registry_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let normalized = if trimmed.is_empty() { "/api" } else { trimmed };
    if normalized == "/api" || normalized.starts_with("/api/") {
        normalized.to_string()
    } else if normalized.starts_with('/') {
        format!("/api{normalized}")
    } else {
        format!("/api/{normalized}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_service_is_explicit() {
        assert!(is_self_service_endpoint("GET", "/api/users/me"));
        assert!(is_self_service_endpoint("GET", "/api/menus/current"));
        assert!(!is_self_service_endpoint("GET", "/api/users"));
    }
    #[test]
    fn restores_api_prefix() {
        assert_eq!(
            permission_registry_path("/roles/1/menus/"),
            "/api/roles/1/menus"
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn permission_denial_records_the_expected_audit_classification(pool: sqlx::PgPool) {
        record_access_denied(
            &audit::AuditService::new(pool.clone()),
            &AuditContext {
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

        let event: (String, String, String, String) = sqlx::query_as(
            r#"
            select action, resource_id, result, reason_code
            from sys_audit_events
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            event,
            (
                "auth.access_denied".to_string(),
                "/api/users".to_string(),
                "denied".to_string(),
                "permission_denied".to_string(),
            )
        );
    }
}
