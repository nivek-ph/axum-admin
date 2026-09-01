#[sqlx::test(migrations = "../../migrations")]
async fn initial_migrations_create_role_only_access_model(pool: sqlx::PgPool) {
    let legacy_relation = sqlx::query_scalar::<_, Option<String>>(
        "select to_regclass('public.sys_role_menus')::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(legacy_relation.is_none());

    let api_bindings =
        sqlx::query_scalar::<_, Option<String>>("select to_regclass('public.sys_menu_apis')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(api_bindings.is_none());

    let missing_super_admin_permissions = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from sys_menus menu
        where menu.menu_type in ('page', 'action')
          and not exists (
              select 1
              from casbin_rule policy
              join sys_roles role on policy.v0 = 'role:' || role.id::text
              where role.code = 'super_admin'
                and policy.ptype = 'p'
                and policy.v1 = menu.permission
          )
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(missing_super_admin_permissions, 0);
}

#[sqlx::test]
async fn menu_api_binding_table_is_removed_when_upgrading_an_existing_schema(pool: sqlx::PgPool) {
    sqlx::raw_sql(include_str!("../../../migrations/0001_schema.sql"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../../migrations/0002_seed.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let before =
        sqlx::query_scalar::<_, Option<String>>("select to_regclass('public.sys_menu_apis')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(before.is_some());

    sqlx::raw_sql(include_str!(
        "../../../migrations/0003_drop_menu_api_bindings.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after =
        sqlx::query_scalar::<_, Option<String>>("select to_regclass('public.sys_menu_apis')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(after.is_none());
}
