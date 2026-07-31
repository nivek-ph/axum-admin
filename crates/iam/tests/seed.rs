use iam::Iam;

#[sqlx::test(migrations = "../../migrations")]
async fn super_admin_seed_has_only_current_concrete_catalog_grants(pool: sqlx::PgPool) {
    let missing_page_access = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from sys_menus menu
        where menu.status = 'enabled'
          and menu.menu_type in ('directory', 'page')
          and not exists (
              select 1
              from sys_role_menus access
              where access.role_id = 1 and access.menu_id = menu.id
          )
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(missing_page_access, 0);

    let missing_permissions = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from sys_menus menu
        where menu.status = 'enabled'
          and menu.permission is not null
          and not exists (
              select 1
              from casbin_rule policy
              where policy.ptype = 'p'
                and policy.v0 = 'role:1'
                and policy.v1 = menu.permission
          )
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(missing_permissions, 0);

    let wildcard_count =
        sqlx::query_scalar::<_, i64>("select count(*) from casbin_rule where v0 = '*' or v1 = '*'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(wildcard_count, 0);

    let removed_flag_count = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from information_schema.columns
        where table_schema = 'public'
          and table_name in ('sys_users', 'sys_roles')
          and column_name = 'is_system'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(removed_flag_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn super_admin_does_not_derive_new_catalog_permissions_at_runtime(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values (500, 'seed-super', 'seed-super', 'hash', 'Seed Super', '', 1)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:500', 'role:1', '', '', '', '')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into sys_menus (
            id, parent_id, name, title, menu_type, status, permission
        ) values (
            1199, 11, 'users:future-operation', 'Future operation',
            'action', 'enabled', 'system:user:future-operation'
        )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    let (_, permissions) = iam.menus.current(500).await.unwrap();

    assert!(
        !permissions
            .iter()
            .any(|permission| permission == "system:user:future-operation")
    );
}
