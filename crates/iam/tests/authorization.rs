use std::time::Duration;

use audit::{AuditActor, AuditContext, AuditSource};
use iam::{Iam, access::AccessEvaluationError, accounts::AccountError};

fn audit_context(actor_id: i64) -> AuditContext {
    AuditContext {
        req_id: format!("req-{actor_id}"),
        actor: AuditActor {
            id: Some(actor_id),
            label: format!("user-{actor_id}"),
        },
        source: AuditSource {
            ip: "127.0.0.1".to_string(),
            user_agent: "iam-user-agent".to_string(),
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

async fn insert_role(pool: &sqlx::PgPool, id: i64, code: &str, status: &str) {
    sqlx::query("insert into sys_roles (id, code, name, status, sort) values ($1, $2, $2, $3, 10)")
        .bind(id)
        .bind(code)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_policy(pool: &sqlx::PgPool, ptype: &str, left: &str, right: &str) {
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ($1, $2, $3, '', '', '', '')
        "#,
    )
    .bind(ptype)
    .bind(left)
    .bind(right)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_subject_permission_is_rejected_at_startup(pool: sqlx::PgPool) {
    insert_user(&pool, 100, "invalid-user").await;
    insert_policy(&pool, "p", "user:100", "system:user:list").await;

    assert!(Iam::load(pool).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn multiple_roles_are_additive_and_survive_restart(pool: sqlx::PgPool) {
    insert_user(&pool, 101, "super-user").await;
    insert_user(&pool, 102, "target-user").await;
    insert_role(&pool, 2, "reader", "enabled").await;
    insert_role(&pool, 3, "creator", "enabled").await;
    insert_policy(&pool, "g", "user:101", "role:1").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    insert_policy(&pool, "p", "role:3", "system:user:list").await;
    insert_policy(&pool, "p", "role:3", "system:user:create").await;
    let iam = Iam::load(pool.clone()).await.unwrap();

    iam.accounts
        .replace_roles(101, 102, vec![2, 3], audit_context(101))
        .await
        .unwrap();
    assert!(
        iam.access
            .authorize_permission(102, "system:user:list")
            .await
            .is_ok()
    );
    assert!(
        iam.access
            .authorize_permission(102, "system:user:create")
            .await
            .is_ok()
    );

    let restarted = Iam::load(pool).await.unwrap();
    let access = restarted.accounts.access(101, 102).await.unwrap();
    assert_eq!(access.assigned_roles.len(), 2);
    assert_eq!(access.effective_permissions.len(), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn zero_role_user_has_only_explicit_self_service(pool: sqlx::PgPool) {
    insert_user(&pool, 103, "zero-role").await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(iam.access.require_active_user(103).await.is_ok());
    assert!(matches!(
        iam.access
            .authorize_permission(103, "system:user:list")
            .await,
        Err(AccessEvaluationError::PermissionDenied)
    ));
    let (menus, permissions) = iam.menus.current(103).await.unwrap();
    assert!(menus.is_empty());
    assert!(permissions.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn request_access_uses_the_last_good_snapshot_when_postgres_is_unavailable(
    pool: sqlx::PgPool,
) {
    insert_user(&pool, 115, "snapshot-user").await;
    insert_user(&pool, 116, "disabled-snapshot-user").await;
    sqlx::query("update sys_users set enable = false where id = 116")
        .execute(&pool)
        .await
        .unwrap();
    insert_role(&pool, 2, "snapshot-reader", "enabled").await;
    insert_policy(&pool, "g", "user:115", "role:2").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    let iam = Iam::load(pool.clone()).await.unwrap();

    pool.close().await;

    assert!(iam.access.require_active_user(115).await.is_ok());
    assert!(matches!(
        iam.access.require_active_user(116).await,
        Err(AccessEvaluationError::UserDisabled)
    ));
    assert!(
        iam.access
            .authorize_permission(115, "system:user:list")
            .await
            .is_ok()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn dormant_membership_is_visible_retainable_and_not_effective(pool: sqlx::PgPool) {
    insert_user(&pool, 104, "super-user").await;
    insert_user(&pool, 105, "target-user").await;
    insert_user(&pool, 106, "other-target").await;
    insert_role(&pool, 2, "dormant", "disabled").await;
    insert_policy(&pool, "g", "user:104", "role:1").await;
    insert_policy(&pool, "g", "user:105", "role:2").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    let iam = Iam::load(pool).await.unwrap();

    let access = iam.accounts.access(104, 105).await.unwrap();
    assert_eq!(access.assigned_roles[0].status, "disabled");
    assert!(access.effective_permissions.is_empty());
    iam.accounts
        .replace_roles(104, 105, vec![2], audit_context(104))
        .await
        .unwrap();
    assert!(matches!(
        iam.accounts
            .replace_roles(104, 106, vec![2], audit_context(104))
            .await,
        Err(AccountError::InvalidRoles)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_user_cannot_administer_access(pool: sqlx::PgPool) {
    insert_user(&pool, 107, "ordinary").await;
    insert_user(&pool, 108, "target").await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.accounts
            .replace_roles(107, 108, Vec::new(), audit_context(107))
            .await,
        Err(AccountError::AccessDenied)
    ));
    assert!(matches!(
        iam.accounts.access(107, 108).await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn final_super_admin_membership_can_be_removed(pool: sqlx::PgPool) {
    insert_user(&pool, 109, "last-super").await;
    insert_policy(&pool, "g", "user:109", "role:1").await;
    let iam = Iam::load(pool).await.unwrap();

    iam.accounts
        .replace_roles(109, 109, Vec::new(), audit_context(109))
        .await
        .unwrap();
    assert!(matches!(
        iam.accounts.access(109, 109).await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_failure_does_not_roll_back_successful_membership(pool: sqlx::PgPool) {
    insert_user(&pool, 110, "super-user").await;
    insert_user(&pool, 111, "target-user").await;
    insert_role(&pool, 2, "operator", "enabled").await;
    insert_policy(&pool, "g", "user:110", "role:1").await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    sqlx::query("drop table sys_audit_events")
        .execute(&pool)
        .await
        .unwrap();

    iam.accounts
        .replace_roles(110, 111, vec![2], audit_context(110))
        .await
        .unwrap();
    assert_eq!(
        iam.accounts.access(110, 111).await.unwrap().assigned_roles[0].id,
        2
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn failed_periodic_reload_retains_last_good_policy_then_recovers(pool: sqlx::PgPool) {
    insert_user(&pool, 112, "target-user").await;
    insert_role(&pool, 2, "reader", "enabled").await;
    insert_policy(&pool, "g", "user:112", "role:2").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let _ = iam
        .start_policy_sync(&redis_url, Duration::from_millis(20))
        .await;

    insert_policy(&pool, "p", "user:112", "system:user:create").await;
    tokio::time::sleep(Duration::from_millis(80)).await;
    assert!(
        iam.access
            .authorize_permission(112, "system:user:list")
            .await
            .is_ok()
    );
    assert!(matches!(
        iam.access
            .authorize_permission(112, "system:user:create")
            .await,
        Err(AccessEvaluationError::PermissionDenied)
    ));

    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = 'user:112'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = 'role:2'")
        .execute(&pool)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if matches!(
                iam.access
                    .authorize_permission(112, "system:user:list")
                    .await,
                Err(AccessEvaluationError::PermissionDenied)
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn watcher_propagates_membership_changes(pool: sqlx::PgPool) {
    insert_user(&pool, 113, "super-user").await;
    insert_user(&pool, 114, "target-user").await;
    insert_role(&pool, 2, "reader", "enabled").await;
    insert_policy(&pool, "g", "user:113", "role:1").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    let publisher = Iam::load(pool.clone()).await.unwrap();
    let subscriber = Iam::load(pool).await.unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    publisher
        .start_policy_sync(&redis_url, Duration::from_secs(60))
        .await
        .unwrap();
    subscriber
        .start_policy_sync(&redis_url, Duration::from_secs(60))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    publisher
        .accounts
        .replace_roles(113, 114, vec![2], audit_context(113))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if subscriber
                .access
                .authorize_permission(114, "system:user:list")
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_reload_does_not_overwrite_policy_mutation(pool: sqlx::PgPool) {
    // Arrange: a super_admin user and a target user with a role that has a permission.
    insert_user(&pool, 117, "super-user").await;
    insert_user(&pool, 118, "target-user").await;
    insert_role(&pool, 2, "reader", "enabled").await;
    insert_policy(&pool, "g", "user:117", "role:1").await;
    insert_policy(&pool, "p", "role:2", "system:user:list").await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    // Use a short periodic interval so reloads fire continuously during the mutation.
    let _ = iam
        .start_policy_sync(&redis_url, Duration::from_millis(10))
        .await;

    // Act: assign role while reloads are firing.
    iam.accounts
        .replace_roles(117, 118, vec![2], audit_context(117))
        .await
        .unwrap();

    // Allow a few reload cycles to complete after the mutation.
    tokio::time::sleep(Duration::from_millis(80)).await;

    // Assert: the mutation must still be visible — a reload must never have
    // swapped in a stale snapshot that erased the role assignment.
    assert!(
        iam.access
            .authorize_permission(118, "system:user:list")
            .await
            .is_ok(),
        "role assignment was erased by a concurrent reload"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_reload_does_not_overwrite_role_permission_mutation(pool: sqlx::PgPool) {
    // Arrange: a super_admin user and a target user already assigned to a role.
    insert_user(&pool, 119, "super-user").await;
    insert_user(&pool, 120, "target-user").await;
    insert_role(&pool, 2, "reader", "enabled").await;
    insert_policy(&pool, "g", "user:119", "role:1").await;
    insert_policy(&pool, "g", "user:120", "role:2").await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let _ = iam
        .start_policy_sync(&redis_url, Duration::from_millis(10))
        .await;

    // Act: grant a permission to the role while reloads are firing.
    iam.roles
        .replace_access(
            119,
            2,
            vec!["system:user:list".to_string()],
            audit_context(119),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(80)).await;

    assert!(
        iam.access
            .authorize_permission(120, "system:user:list")
            .await
            .is_ok(),
        "role permission grant was erased by a concurrent reload"
    );
}
