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
            user_agent: "iam-integration-test".to_string(),
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
async fn direct_permission_is_enforced_without_role_or_page_access(pool: sqlx::PgPool) {
    insert_user(&pool, 100, "direct-user").await;
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'user:100', 'system:user:list', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    assert!(iam.access.evaluate(100, "GET", "/api/users").await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn disabled_role_grant_is_dormant(pool: sqlx::PgPool) {
    insert_user(&pool, 101, "dormant-user").await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'dormant', 'Dormant', 'disabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 101, 2).await;
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'role:2', 'system:user:list', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.access.evaluate(101, "GET", "/api/users").await,
        Err(AccessEvaluationError::PermissionDenied { .. })
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn only_active_super_admin_can_manage_employee_access(pool: sqlx::PgPool) {
    insert_user(&pool, 102, "super-user").await;
    insert_user(&pool, 103, "ordinary-user").await;
    insert_user(&pool, 104, "target-user").await;
    assign_role(&pool, 102, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    iam.accounts
        .replace_roles(102, 104, Vec::new(), audit_context(102))
        .await
        .unwrap();
    assert!(matches!(
        iam.accounts
            .replace_roles(103, 104, Vec::new(), audit_context(103))
            .await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn final_active_super_admin_membership_cannot_be_removed(pool: sqlx::PgPool) {
    insert_user(&pool, 105, "last-super").await;
    assign_role(&pool, 105, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.accounts
            .replace_roles(105, 105, Vec::new(), audit_context(105))
            .await,
        Err(AccountError::LastSuperAdmin)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_employee_access_target_is_not_found(pool: sqlx::PgPool) {
    insert_user(&pool, 106, "super-user").await;
    assign_role(&pool, 106, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.accounts
            .replace_roles(106, 999, Vec::new(), audit_context(106))
            .await,
        Err(AccountError::NotFound)
    ));
    assert!(matches!(
        iam.accounts
            .replace_direct_permissions(106, 999, Vec::new(), audit_context(106))
            .await,
        Err(AccountError::NotFound)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn dormant_role_membership_is_visible_and_replaceable(pool: sqlx::PgPool) {
    insert_user(&pool, 107, "super-user").await;
    insert_user(&pool, 108, "dormant-target").await;
    assign_role(&pool, 107, 1).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'dormant', 'Dormant', 'disabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 108, 2).await;
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'role:2', 'system:user:list', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    let access = iam.accounts.access(107, 108).await.unwrap();
    assert_eq!(access.role_ids, vec![2]);
    assert!(access.effective_permissions.is_empty());
    iam.accounts
        .replace_roles(107, 108, vec![2], audit_context(107))
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn malformed_policy_prevents_authorization_startup(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'user:999', '*', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(Iam::load(pool).await.is_err());
}

#[sqlx::test(migrations = "../../migrations")]
async fn overlapping_catalog_binding_follows_matchit_priority(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_menu_apis (menu_id, method, path_pattern)
        values (1106, 'GET', '/api/{area}/{id}/permissions')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    assert!(Iam::load(pool).await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn employee_access_survives_authorization_restart(pool: sqlx::PgPool) {
    insert_user(&pool, 109, "super-user").await;
    insert_user(&pool, 110, "restart-target").await;
    assign_role(&pool, 109, 1).await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    iam.accounts
        .replace_direct_permissions(
            109,
            110,
            vec!["system:user:list".to_string()],
            audit_context(109),
        )
        .await
        .unwrap();

    let restarted = Iam::load(pool).await.unwrap();
    assert!(
        restarted
            .access
            .evaluate(110, "GET", "/api/users")
            .await
            .is_ok()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn audit_failure_rolls_back_membership_final_set(pool: sqlx::PgPool) {
    insert_user(&pool, 111, "super-user").await;
    insert_user(&pool, 112, "rollback-target").await;
    assign_role(&pool, 111, 1).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 112, 2).await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    sqlx::query("drop table sys_audit_events")
        .execute(&pool)
        .await
        .unwrap();

    assert!(matches!(
        iam.accounts
            .replace_roles(111, 112, Vec::new(), audit_context(111))
            .await,
        Err(AccountError::Audit(_))
    ));
    assert_eq!(
        iam.accounts.access(111, 112).await.unwrap().role_ids,
        vec![2]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn post_commit_reload_failure_is_successful_and_periodic_reload_recovers(pool: sqlx::PgPool) {
    insert_user(&pool, 113, "super-user").await;
    insert_user(&pool, 114, "reload-target").await;
    assign_role(&pool, 113, 1).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'role:2', 'system:user:list', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool.clone()).await.unwrap();
    sqlx::query(
        r#"
        create function inject_invalid_policy() returns trigger language plpgsql as $$
        begin
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ('p', 'role:2', '*', '', '', '', '');
            return new;
        end
        $$
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        create trigger inject_invalid_policy
        after insert on sys_audit_events
        for each row execute function inject_invalid_policy()
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    iam.accounts
        .replace_roles(113, 114, vec![2], audit_context(113))
        .await
        .unwrap();
    assert!(matches!(
        iam.access.evaluate(114, "GET", "/api/users").await,
        Err(AccessEvaluationError::PermissionDenied { .. })
    ));

    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = 'role:2' and v1 = '*'")
        .execute(&pool)
        .await
        .unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let _ = iam.start_policy_sync(&redis_url, Duration::from_millis(20));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if iam.access.evaluate(114, "GET", "/api/users").await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("periodic reload should recover the failed state");
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_removals_preserve_one_active_super_admin(pool: sqlx::PgPool) {
    insert_user(&pool, 115, "first-super").await;
    insert_user(&pool, 116, "second-super").await;
    assign_role(&pool, 115, 1).await;
    assign_role(&pool, 116, 1).await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    let first = iam.accounts.clone();
    let second = iam.accounts.clone();

    let first_remove = tokio::spawn(async move {
        first
            .replace_roles(115, 115, Vec::new(), audit_context(115))
            .await
    });
    let second_remove = tokio::spawn(async move {
        second
            .replace_roles(116, 116, Vec::new(), audit_context(116))
            .await
    });
    let (first_result, second_result) = tokio::time::timeout(Duration::from_secs(3), async {
        (first_remove.await.unwrap(), second_remove.await.unwrap())
    })
    .await
    .expect("concurrent membership final sets should not deadlock");

    assert!(
        matches!(
            (&first_result, &second_result),
            (Ok(()), Err(AccountError::LastSuperAdmin))
                | (Err(AccountError::LastSuperAdmin), Ok(()))
        ),
        "exactly one final super_admin removal should succeed"
    );
    let active = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from sys_users account
        join casbin_rule membership
          on membership.ptype = 'g'
         and membership.v0 = 'user:' || account.id::text
        where account.enable and membership.v1 = 'role:1'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn watcher_propagates_employee_access_changes(pool: sqlx::PgPool) {
    insert_user(&pool, 117, "super-user").await;
    insert_user(&pool, 118, "watcher-target").await;
    assign_role(&pool, 117, 1).await;
    let publisher = Iam::load(pool.clone()).await.unwrap();
    let subscriber = Iam::load(pool).await.unwrap();
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    publisher
        .start_policy_sync(&redis_url, Duration::from_secs(60))
        .unwrap();
    subscriber
        .start_policy_sync(&redis_url, Duration::from_secs(60))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    publisher
        .accounts
        .replace_direct_permissions(
            117,
            118,
            vec!["system:user:list".to_string()],
            audit_context(117),
        )
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if subscriber
                .access
                .evaluate(118, "GET", "/api/users")
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("watcher should reload the subscriber");
}
