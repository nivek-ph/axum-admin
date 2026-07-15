use audit::{
    AuditAction, AuditActor, AuditEvent, AuditQuery, AuditResource, AuditResult, AuditService,
    AuditSource,
};
use sqlx::PgPool;

fn login_event(result: AuditResult) -> AuditEvent {
    AuditEvent {
        actor: AuditActor {
            id: Some(7),
            label: "admin".to_string(),
        },
        action: AuditAction::Login,
        resource: AuditResource::Account("admin".to_string()),
        result,
        reason_code: None,
        source: AuditSource {
            ip: "127.0.0.1".to_string(),
            user_agent: "audit-test".to_string(),
        },
        changes: Vec::new(),
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn unified_audit_store_records_and_filters_structured_events(pool: PgPool) {
    let service = AuditService::new(pool.clone());
    service
        .record(login_event(AuditResult::Succeeded))
        .await
        .expect("audit event should be recorded");

    let (events, total, page, page_size) = service
        .list(AuditQuery {
            page: 1,
            page_size: 10,
            actor: Some("admin".to_string()),
            action: Some("auth.login".to_string()),
            resource_type: Some("account".to_string()),
            result: Some("succeeded".to_string()),
            started_at: None,
            ended_at: None,
        })
        .await
        .expect("audit events should be queryable");

    assert_eq!(total, 1);
    assert_eq!(page, 1);
    assert_eq!(page_size, 10);
    assert_eq!(events[0].actor_label, "admin");
    assert_eq!(events[0].action, "auth.login");
    assert_eq!(events[0].resource_type, "account");
    assert_eq!(events[0].resource_id.as_deref(), Some("admin"));
    assert_eq!(events[0].result, "succeeded");
    assert_eq!(events[0].source_ip, "127.0.0.1");

    let item = service
        .find(events[0].id)
        .await
        .expect("audit detail query should succeed")
        .expect("audit event should exist");
    assert_eq!(item.id, events[0].id);

    let old_tables: (Option<String>, Option<String>) = sqlx::query_as(
        "select to_regclass('sys_login_logs')::text, to_regclass('sys_operation_records')::text",
    )
    .fetch_one(&pool)
    .await
    .expect("table names should be inspectable");
    assert_eq!(old_tables, (None, None));
}
