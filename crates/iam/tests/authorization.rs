use audit::{AuditActor, AuditContext, AuditSource};
use iam::{
    access::ResolvedDataScope,
    authorization::{Authorization, PolicyAdministrationError, ReplaceUserRoles},
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

#[sqlx::test(migrations = "../../migrations")]
async fn user_role_replacement_rolls_back_when_success_audit_fails(pool: sqlx::PgPool) {
    seed_membership_accounts(&pool).await;
    sqlx::query(
        "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
         values ('g', 'user:101', 'role:1', '', '', '', '')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("drop table sys_audit_events")
        .execute(&pool)
        .await
        .unwrap();
    let authorization = Authorization::new(pool.clone());

    let error = authorization
        .replace_user_roles(ReplaceUserRoles {
            actor_user_id: 100,
            user_id: 101,
            role_ids: vec![2],
            data_scope: ResolvedDataScope::All,
            audit_context: audit_context(),
        })
        .await
        .expect_err("audit persistence failure should fail membership replacement");

    assert!(matches!(error, PolicyAdministrationError::Audit(_)));
    assert_eq!(authorization.user_role_ids(101).await.unwrap(), vec![1]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn direct_user_permission_is_enforceable_without_role_membership(pool: sqlx::PgPool) {
    seed_membership_accounts(&pool).await;
    sqlx::query(
        "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
         values ('p', 'user:101', 'system:user:list', '', '', '', '')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let authorization = Authorization::load(pool).await.unwrap();

    assert!(authorization.user_role_ids(101).await.unwrap().is_empty());
    assert!(
        authorization
            .effective_permissions(101)
            .await
            .unwrap()
            .contains("system:user:list")
    );
    assert!(
        authorization
            .enforce(101, "system:user:list")
            .await
            .unwrap()
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn role_user_replacement_is_a_validated_final_set(pool: sqlx::PgPool) {
    seed_membership_accounts(&pool).await;
    let authorization = Authorization::new(pool);

    authorization
        .replace_role_users(2, vec![101, 101, 0])
        .await
        .unwrap();
    assert_eq!(authorization.role_user_ids(2).await.unwrap(), vec![101]);

    let error = authorization
        .replace_role_users(2, vec![999])
        .await
        .expect_err("unknown users should not replace the current final set");
    assert!(matches!(
        error,
        PolicyAdministrationError::InvalidUserAssignment
    ));
    assert_eq!(authorization.role_user_ids(2).await.unwrap(), vec![101]);

    authorization
        .replace_role_users(2, Vec::new())
        .await
        .unwrap();
    assert!(authorization.role_user_ids(2).await.unwrap().is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn concurrent_membership_final_sets_share_one_lock_order(pool: sqlx::PgPool) {
    seed_membership_accounts(&pool).await;
    let authorization = Authorization::new(pool);
    let user_roles = authorization.clone();
    let role_users = authorization.clone();

    let user_write = tokio::spawn(async move {
        user_roles
            .replace_user_roles(ReplaceUserRoles {
                actor_user_id: 100,
                user_id: 101,
                role_ids: vec![2],
                data_scope: ResolvedDataScope::All,
                audit_context: audit_context(),
            })
            .await
    });
    let role_write = tokio::spawn(async move { role_users.replace_role_users(2, vec![101]).await });

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        user_write.await.unwrap().unwrap();
        role_write.await.unwrap().unwrap();
    })
    .await
    .expect("membership final-set mutations should not deadlock");
    assert_eq!(authorization.user_role_ids(101).await.unwrap(), vec![2]);
    assert_eq!(authorization.role_user_ids(2).await.unwrap(), vec![101]);
}
