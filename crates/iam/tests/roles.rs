use iam::{Iam, access::AccessEvaluationError, roles::RoleError};

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
async fn only_active_super_admin_can_replace_role_access(pool: sqlx::PgPool) {
    insert_user(&pool, 300, "super-user").await;
    insert_user(&pool, 301, "ordinary-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 300, 1).await;
    let iam = Iam::load(pool).await.unwrap();

    assert!(matches!(
        iam.roles.set_menu_ids(301, 2, vec![10, 11]).await,
        Err(RoleError::AccessDenied)
    ));
    assert!(matches!(
        iam.roles
            .set_permissions(301, 2, vec!["system:user:create".to_string()])
            .await,
        Err(RoleError::AccessDenied)
    ));
    assert!(matches!(
        iam.roles.set_menu_ids(301, 2, vec![1101]).await,
        Err(RoleError::AccessDenied)
    ));
    assert!(matches!(
        iam.roles
            .set_permissions(301, 2, vec!["unknown:permission".to_string()])
            .await,
        Err(RoleError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn page_access_atomically_maintains_entry_permission(pool: sqlx::PgPool) {
    insert_user(&pool, 302, "super-user").await;
    insert_user(&pool, 303, "operator-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 302, 1).await;
    assign_role(&pool, 303, 2).await;
    let iam = Iam::load(pool).await.unwrap();

    iam.roles
        .set_permissions(302, 2, vec!["system:user:create".to_string()])
        .await
        .unwrap();
    iam.roles.set_menu_ids(302, 2, vec![10, 11]).await.unwrap();

    assert!(iam.access.evaluate(303, "GET", "/api/users").await.is_ok());
    assert!(iam.access.evaluate(303, "POST", "/api/users").await.is_ok());
    assert_eq!(
        iam.roles.permissions(2).await.unwrap().permissions,
        vec!["system:user:create"]
    );
    assert!(
        iam.roles
            .permission_catalog(2)
            .await
            .unwrap()
            .iter()
            .all(|item| item.menu_type == "action")
    );

    iam.roles.set_menu_ids(302, 2, Vec::new()).await.unwrap();

    assert!(matches!(
        iam.access.evaluate(303, "GET", "/api/users").await,
        Err(AccessEvaluationError::PermissionDenied { .. })
    ));
    assert!(iam.access.evaluate(303, "POST", "/api/users").await.is_ok());
}

#[sqlx::test(migrations = "../../migrations")]
async fn action_permission_replacement_preserves_required_page_entry(pool: sqlx::PgPool) {
    insert_user(&pool, 304, "super-user").await;
    insert_user(&pool, 305, "operator-user").await;
    insert_role(&pool, 2).await;
    assign_role(&pool, 304, 1).await;
    assign_role(&pool, 305, 2).await;
    let iam = Iam::load(pool).await.unwrap();

    iam.roles.set_menu_ids(304, 2, vec![10, 11]).await.unwrap();
    iam.roles.set_permissions(304, 2, Vec::new()).await.unwrap();

    assert!(iam.access.evaluate(305, "GET", "/api/users").await.is_ok());
}
