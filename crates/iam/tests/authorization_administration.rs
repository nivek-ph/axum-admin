use audit::{AuditActor, AuditContext, AuditSource};
use iam::{
    access::ResolvedDataScope,
    authorization::{Authorization, ReplaceUserRoles},
};

fn audit_context() -> AuditContext {
    AuditContext {
        req_id: "req-authorization-user-roles".to_string(),
        actor: AuditActor {
            id: Some(100),
            label: "actor".to_string(),
        },
        source: AuditSource {
            ip: "127.0.0.1".to_string(),
            user_agent: "authorization-test".to_string(),
        },
    }
}

async fn seed_membership_accounts(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_users (
            id, uuid, username, password_hash, nick_name, header_img, home_route,
            enable, dept_id, is_system
        ) values
            (100, 'authorization-actor', 'authorization-actor', 'hash', 'Actor', '',
             'dashboard', true, 1, false),
            (101, 'authorization-target', 'authorization-target', 'hash', 'Target', '',
             'dashboard', true, 1, false)
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
         values (2, 'operator', 'Operator', 'enabled', 10, 'self', false)",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_role_replacement_and_success_audit_commit_together(pool: sqlx::PgPool) {
    seed_membership_accounts(&pool).await;
    let authorization = Authorization::new(pool.clone());

    authorization
        .replace_user_roles(ReplaceUserRoles {
            actor_user_id: 100,
            user_id: 101,
            role_ids: vec![2],
            data_scope: ResolvedDataScope::All,
            audit_context: audit_context(),
        })
        .await
        .unwrap();

    assert_eq!(authorization.user_role_ids(101).await.unwrap(), vec![2]);
    let event = sqlx::query_as::<_, (String, String, String)>(
        "select action, result, changes::text from sys_audit_events where req_id = $1",
    )
    .bind("req-authorization-user-roles")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(event.0, "user.assign_roles");
    assert_eq!(event.1, "succeeded");
    assert!(event.2.contains("\"after\""));
}
