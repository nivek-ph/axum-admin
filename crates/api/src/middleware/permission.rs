use audit::{
    AuditAction, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
};
use axum::{
    extract::{OriginalUri, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::MethodRouter,
};
use tower_http::request_id::RequestId;

use crate::{
    extractors::current_user::AuthenticatedUser, mappings::INTERNAL_SERVER_ERROR,
    request_id::request_id_text, state::AppState,
};

#[derive(Clone)]
pub(crate) struct PermissionGuardContext {
    access: iam::access::AccessService,
    audits: AuditService,
}

impl PermissionGuardContext {
    pub(crate) fn new(access: iam::access::AccessService, audits: AuditService) -> Self {
        Self { access, audits }
    }
}

pub(crate) fn permission(
    code: &'static str,
    route: MethodRouter<AppState>,
) -> MethodRouter<AppState> {
    route.route_layer(middleware::from_fn_with_state(
        code,
        require_declared_permission,
    ))
}

async fn require_declared_permission(
    State(permission): State<&'static str>,
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let path = request
        .extensions()
        .get::<OriginalUri>()
        .map(|uri| uri.0.path())
        .unwrap_or_else(|| request.uri().path())
        .to_string();
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(request_id_text)
        .unwrap_or_else(|| "missing-request-id".to_string());
    let user = request.extensions().get::<AuthenticatedUser>().cloned();
    let guard = request
        .extensions()
        .get::<PermissionGuardContext>()
        .cloned();
    let audit_context = request.extensions().get::<AuditContext>().cloned();
    let (Some(user), Some(guard), Some(audit_context)) = (user, guard, audit_context) else {
        tracing::error!(
            permission,
            method = %method,
            path,
            request_id,
            "HIGH PRIORITY: Permission guard is missing authenticated request context"
        );
        return INTERNAL_SERVER_ERROR.into_error().into_response();
    };

    match guard.access.authorize_permission(user.id, permission).await {
        Ok(()) => next.run(request).await,
        Err(iam::access::AccessEvaluationError::PermissionDenied) => {
            record_access_denied(&guard.audits, audit_context, path).await;
            crate::AppError::from(iam::access::AccessEvaluationError::PermissionDenied)
                .into_response()
        }
        Err(error) => crate::AppError::from(error).into_response(),
    }
}

async fn record_access_denied(audits: &AuditService, context: AuditContext, path: String) {
    audits
        .record_best_effort(AuditEvent {
            req_id: context.req_id,
            actor: context.actor,
            action: AuditAction::AccessDenied,
            resource: AuditResource::Route(path),
            result: AuditResult::Denied,
            reason_code: Some(AuditReason::PermissionDenied),
            source: context.source,
            changes: Vec::new(),
        })
        .await;
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use audit::{AuditActor, AuditContext, AuditSource};
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::{record_access_denied, require_declared_permission};

    #[tokio::test]
    async fn missing_authenticated_context_fails_closed() {
        let handler_called = Arc::new(AtomicBool::new(false));
        let handler_flag = handler_called.clone();
        let app = Router::new()
            .route(
                "/protected",
                get(move || {
                    let handler_flag = handler_flag.clone();
                    async move {
                        handler_flag.store(true, Ordering::Relaxed);
                        "unexpected"
                    }
                }),
            )
            .route_layer(axum::middleware::from_fn_with_state(
                "system:test:read",
                require_declared_permission,
            ));
        let response = app
            .oneshot(Request::get("/protected").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "INTERNAL_SERVER_ERROR");
        assert!(!handler_called.load(Ordering::Relaxed));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn permission_denial_records_the_expected_audit_classification(pool: sqlx::PgPool) {
        record_access_denied(
            &audit::AuditService::new(pool.clone()),
            AuditContext {
                req_id: "req-access-denied-1".to_string(),
                actor: AuditActor {
                    id: Some(1),
                    label: "admin".to_string(),
                },
                source: AuditSource {
                    ip: "127.0.0.1".to_string(),
                    user_agent: "permission-middleware-test".to_string(),
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
