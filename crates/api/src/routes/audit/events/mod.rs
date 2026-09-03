mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, post},
};
pub use handler::*;

use crate::{middleware::permission::PermissionRouteExt, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_audit_events).permission("system:audit-event:list"),
        )
        .route(
            "/stats",
            get(get_audit_stats).permission("system:audit-event:list"),
        )
        .route(
            "/analyze",
            post(analyze_audit_events).permission("system:audit-event:list"),
        )
        .route(
            "/{id}",
            get(find_audit_event).permission("system:audit-event:list"),
        )
}

#[cfg(test)]
mod tests {
    use audit::{
        AuditAction, AuditActor, AuditEvent, AuditResource, AuditResult, AuditService, AuditSource,
    };
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    use super::*;

    fn handler_routes() -> Router<AppState> {
        Router::new()
            .route("/", get(get_audit_events))
            .route("/stats", get(get_audit_stats))
            .route("/analyze", post(analyze_audit_events))
            .route("/{id}", get(find_audit_event))
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn list_and_detail_routes_return_the_filtered_audit_event(pool: sqlx::PgPool) {
        AuditService::new(pool.clone())
            .record(AuditEvent {
                req_id: "req-api-list-1".to_string(),
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
        let app = handler_routes().with_state(crate::state::tests::test_state(pool.clone()).await);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/?page=1&pageSize=10&reqId=api-list&actor=admin&action=user.assign_roles&resourceType=user&resourceId=7&result=succeeded&startedAt=2000-01-01T00%3A00%3A00Z&endedAt=2100-01-01T00%3A00%3A00Z")
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
        assert_eq!(body["data"]["list"][0]["reqId"], "req-api-list-1");
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
            .clone()
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
        let app = handler_routes().with_state(crate::state::tests::test_state(pool).await);

        let response = app
            .clone()
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

    #[sqlx::test(migrations = "../../migrations")]
    async fn stats_route_returns_login_and_security_trends_for_utc_days(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_audit_events (
                req_id, actor_label, action, resource_type, resource_id,
                result, source_ip, user_agent
            ) values
                ('today-login-1', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.10', 'api-test'),
                ('today-login-2', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.10', 'api-test'),
                ('today-login-3', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.11', 'api-test'),
                ('today-login-empty-ip', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '', 'api-test'),
                ('today-login-denied', 'admin', 'auth.login', 'account', 'admin', 'denied', '203.0.113.11', 'api-test'),
                ('today-login-failed', 'admin', 'auth.login', 'account', 'admin', 'failed', '203.0.113.12', 'api-test'),
                ('today-access-denied', 'admin', 'auth.access_denied', 'route', '/api/users', 'denied', '203.0.113.13', 'api-test'),
                ('today-role-failed', 'admin', 'user.assign_roles', 'user', '7', 'failed', '203.0.113.14', 'api-test'),
                ('oldest-login', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.15', 'api-test'),
                ('outside-login', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.16', 'api-test'),
                ('at-window-end', 'admin', 'auth.login', 'account', 'admin', 'succeeded', '203.0.113.17', 'api-test')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            with bounds as (
                select
                    (date_trunc('day', now() at time zone 'UTC') at time zone 'UTC')
                        - interval '13 days' as start_at
            )
            update sys_audit_events
            set created_at = case req_id
                when 'oldest-login' then bounds.start_at
                when 'outside-login' then bounds.start_at - interval '1 microsecond'
                when 'at-window-end' then bounds.start_at + interval '14 days'
                else created_at
            end
            from bounds
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        let app = handler_routes().with_state(crate::state::tests::test_state(pool.clone()).await);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/stats?days=14")
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
        let data = &body["data"];
        assert_eq!(data["days"], 14);
        assert_eq!(data["eventCount"], 11);
        assert_eq!(data["todayLogins"], 4);
        assert_eq!(data["todayIps"], 2);
        let daily = data["daily"].as_array().unwrap();
        assert_eq!(daily.len(), 14);
        let expected_dates = sqlx::query_scalar::<_, String>(
            r#"
            select to_char(
                (now() at time zone 'UTC')::date - offset_days,
                'YYYY-MM-DD'
            )
            from generate_series(13, 0, -1) as offsets(offset_days)
            "#,
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let actual_dates = daily
            .iter()
            .map(|row| row["date"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(actual_dates, expected_dates);
        assert_eq!(daily[0]["logins"], 1);
        assert_eq!(daily[0]["ips"], 1);
        for row in &daily[1..13] {
            assert_eq!(row["logins"], 0);
            assert_eq!(row["ips"], 0);
            assert_eq!(row["loginFailures"], 0);
            assert_eq!(row["accessDenials"], 0);
        }
        assert_eq!(daily[13]["logins"], 4);
        assert_eq!(daily[13]["ips"], 2);
        assert_eq!(daily[13]["loginFailures"], 2);
        assert_eq!(daily[13]["accessDenials"], 1);
        assert_eq!(data["todayLogins"], daily[13]["logins"]);
        assert_eq!(data["todayIps"], daily[13]["ips"]);
        assert!(data.get("loginCount").is_none());
        assert!(data.get("ips").is_none());
        assert!(data.get("byHour").is_none());
        assert!(data.get("topActions").is_none());
        assert!(data.get("topIps").is_none());

        for (days, expected_logins) in [(1, 4), (90, 6)] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/stats?days={days}"))
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
            assert_eq!(body["data"]["days"], days);
            assert_eq!(body["data"]["eventCount"], 11);
            let daily = body["data"]["daily"].as_array().unwrap();
            assert_eq!(daily.len(), days as usize);
            assert_eq!(daily.last().unwrap()["date"], expected_dates[13]);
            assert_eq!(
                daily
                    .iter()
                    .map(|row| row["logins"].as_i64().unwrap())
                    .sum::<i64>(),
                expected_logins
            );
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn stats_route_defaults_and_clamps_days(pool: sqlx::PgPool) {
        let app = handler_routes().with_state(crate::state::tests::test_state(pool).await);

        for (uri, expected_days) in [("/stats", 14), ("/stats?days=0", 1), ("/stats?days=91", 90)] {
            let response = app
                .clone()
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(body["data"]["days"], expected_days);
            assert_eq!(
                body["data"]["daily"].as_array().unwrap().len(),
                expected_days as usize
            );
        }
    }
}
