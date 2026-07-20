mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, post},
};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_audit_events))
        .route("/analyze", post(analyze_audit_events))
        .route("/{id}", get(find_audit_event))
}

#[cfg(test)]
mod tests {
    use audit::{
        AuditAction, AuditActor, AuditEvent, AuditResource, AuditResult, AuditService, AuditSource,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    #[sqlx::test(migrations = "../../migrations")]
    async fn list_and_detail_routes_return_the_filtered_audit_event(pool: sqlx::PgPool) {
        AuditService::new(pool.clone())
            .record(AuditEvent {
                actor: AuditActor {
                    id: Some(1),
                    label: "admin".to_string(),
                },
                action: AuditAction::AssignUserRoles,
                resource: AuditResource::User(7),
                result: AuditResult::Succeeded,
                reason_code: None,
                source: AuditSource {
                    ip: "127.0.0.1".to_string(),
                    user_agent: "api-test".to_string(),
                },
                changes: Vec::new(),
            })
            .await
            .unwrap();
        let app = routes().with_state(crate::state::test_state(pool));

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?page=1&pageSize=10&actor=admin&action=user.assign_roles&resourceType=user&resourceId=7&result=succeeded&startedAt=2000-01-01T00%3A00%3A00Z&endedAt=2100-01-01T00%3A00%3A00Z")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["data"]["total"], 1);
        assert_eq!(body["data"]["list"][0]["resourceId"], "7");
        let id = body["data"]["list"][0]["id"].as_i64().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["data"]["id"], id);
        assert_eq!(body["data"]["action"], "user.assign_roles");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/?page=1&pageSize=10&startedAt=not-a-timestamp")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 400);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], "INVALID_AUDIT_TIME_RANGE");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn analyze_route_returns_a_low_risk_result_when_no_events_match(pool: sqlx::PgPool) {
        let app = routes().with_state(crate::state::test_state(pool));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/analyze")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"does.not.exist"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["data"]["riskLevel"], "low");
        assert_eq!(body["data"]["findings"], serde_json::json!([]));
    }
}
