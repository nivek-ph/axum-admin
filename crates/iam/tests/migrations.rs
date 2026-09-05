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
