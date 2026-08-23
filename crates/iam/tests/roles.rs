use audit::{AuditActor, AuditContext, AuditSource};
use iam::{Iam, access::AccessEvaluationError, roles::RoleError};

fn audit_context(actor_id: i64) -> AuditContext {
    AuditContext {
        req_id: format!("role-{actor_id}"),
        actor: AuditActor {
            id: Some(actor_id),
            label: format!("user-{actor_id}"),
        },
        source: AuditSource {
            ip: "127.0.0.1".to_string(),
            user_agent: "role-integration-test".to_string(),
        },
    }
}

async fn insert_user(pool: &sqlx::PgPool, id: i64, name: &str) {
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values ($1, $2, $2, 'hash', $2, '', 1)
        "#,
    )
    .bind(id)
    .bind(name)
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_role(pool: &sqlx::PgPool, id: i64) {
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values ($1, 'operator', 'Operator', 'enabled', 10)",
    )
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
}

async fn assign_role(pool: &sqlx::PgPool, user_id: i64, role_id: i64) {
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('g', 'user:' || $1::text, 'role:' || $2::text, '', '', '', '')
        "#,
    )
    .bind(user_id)
    .bind(role_id)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn only_super_admin_can_read_or_replace_role_access(pool: sqlx::PgPool) {
    insert_user(&pool, 300, "super-user").await;
    insert_user(&pool, 301, "ordinary-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 300, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.roles.access(301, 2).await,
        Err(RoleError::AccessDenied)
    ));
    assert!(matches!(
        iam.roles
            .replace_access(
                301,
                2,
                vec!["system:user:list".to_string()],
                audit_context(301),
            )
            .await,
        Err(RoleError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn action_selection_adds_page_and_persists_through_restart(pool: sqlx::PgPool) {
    insert_user(&pool, 302, "super-user").await;
    insert_user(&pool, 303, "operator-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 302, 1).await;
    assign_role(&pool, 303, 2).await;
    let iam = Iam::load(pool.clone()).await.unwrap();

    iam.roles
        .replace_access(
            302,
            2,
            vec!["system:user:create".to_string()],
            audit_context(302),
        )
        .await
        .unwrap();
    let access = iam.roles.access(302, 2).await.unwrap();
    assert_eq!(
        access.permissions,
        vec!["system:user:create", "system:user:list"]
    );
    assert!(iam.access.evaluate(303, "GET", "/api/users").await.is_ok());
    assert!(iam.access.evaluate(303, "POST", "/api/users").await.is_ok());

    let restarted = Iam::load(pool).await.unwrap();
    assert!(
        restarted
            .access
            .evaluate(303, "GET", "/api/users")
            .await
            .is_ok()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn removing_page_and_actions_removes_navigation_and_operations(pool: sqlx::PgPool) {
    insert_user(&pool, 304, "super-user").await;
    insert_user(&pool, 305, "operator-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 304, 1).await;
    assign_role(&pool, 305, 2).await;
    let iam = Iam::load(pool).await.unwrap();
    iam.roles
        .replace_access(
            304,
            2,
            vec!["system:user:create".to_string()],
            audit_context(304),
        )
        .await
        .unwrap();

    iam.roles
        .replace_access(304, 2, Vec::new(), audit_context(304))
        .await
        .unwrap();
    assert!(matches!(
        iam.access.evaluate(305, "GET", "/api/users").await,
        Err(AccessEvaluationError::PermissionDenied { .. })
    ));
    assert!(matches!(
        iam.access.evaluate(305, "POST", "/api/users").await,
        Err(AccessEvaluationError::PermissionDenied { .. })
    ));
    assert!(iam.menus.current(305).await.unwrap().0.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn role_with_members_cannot_be_deleted(pool: sqlx::PgPool) {
    insert_user(&pool, 306, "super-user").await;
    insert_user(&pool, 307, "member-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 306, 1).await;
    assign_role(&pool, 307, 2).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.roles.delete(306, 2, audit_context(306)).await,
        Err(RoleError::HasMembers)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn protected_role_metadata_access_and_delete_are_immutable(pool: sqlx::PgPool) {
    insert_user(&pool, 308, "super-user").await;
    assign_role(&pool, 308, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.roles
            .replace_access(308, 1, Vec::new(), audit_context(308))
            .await,
        Err(RoleError::Immutable)
    ));
    assert!(matches!(
        iam.roles.delete(308, 1, audit_context(308)).await,
        Err(RoleError::Immutable)
    ));
}
