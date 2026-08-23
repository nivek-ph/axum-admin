async fn apply(pool: &sqlx::PgPool, sql: &'static str) {
    sqlx::raw_sql(sql).execute(pool).await.unwrap();
}

#[sqlx::test]
async fn role_only_migrations_convert_legacy_access_and_preserve_membership(pool: sqlx::PgPool) {
    apply(&pool, include_str!("../../../migrations/0001_schema.sql")).await;
    apply(&pool, include_str!("../../../migrations/0002_seed.sql")).await;

    sqlx::query("update sys_roles set code = 'legacy-admin' where id = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "insert into sys_roles (id, code, name, status) values (7, 'super_admin', 'Super Admin', 'enabled'), (8, 'legacy-role', 'Legacy Role', 'enabled')",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("update sys_role_menus set role_id = 7 where role_id = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("update casbin_rule set v0 = 'role:7' where ptype = 'p' and v0 = 'role:1'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("delete from sys_roles where id = 1")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values (50, 'legacy-user', 'legacy-user', 'hash', 'Legacy User', '', 1)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("insert into sys_role_menus (role_id, menu_id) values (8, 12)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values
            ('g', 'user:50', 'role:8', '', '', '', ''),
            ('p', 'role:8', 'system:user:create', '', '', '', ''),
            ('p', 'user:50', 'system:dashboard:view', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    apply(
        &pool,
        include_str!("../../../migrations/0003_role_access.sql"),
    )
    .await;

    let role_permissions = sqlx::query_scalar::<_, String>(
        "select v1 from casbin_rule where ptype = 'p' and v0 = 'role:8' order by v1",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        role_permissions,
        vec!["system:role:list", "system:user:create", "system:user:list"]
    );
    let old_relation = sqlx::query_scalar::<_, Option<String>>(
        "select to_regclass('public.sys_role_menus')::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(old_relation.is_none());
    let super_role_access = sqlx::query_scalar::<_, i64>(
        "select count(*) from casbin_rule where ptype = 'p' and v0 = 'role:7' and v1 in ('system:role:access-read', 'system:role:access-update')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(super_role_access, 2);

    apply(
        &pool,
        include_str!("../../../migrations/0004_role_only_user_access.sql"),
    )
    .await;

    let direct_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from casbin_rule where ptype = 'p' and v0 like 'user:%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(direct_count, 0);
    let membership_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from casbin_rule where ptype = 'g' and v0 = 'user:50' and v1 = 'role:8'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(membership_count, 1);
    let super_user_access = sqlx::query_scalar::<_, i64>(
        "select count(*) from casbin_rule where ptype = 'p' and v0 = 'role:7' and v1 = 'system:user:access-read'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(super_user_access, 1);
}
