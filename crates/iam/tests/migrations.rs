#[sqlx::test(migrations = "../../migrations")]
async fn initial_migrations_create_role_only_access_model(pool: sqlx::PgPool) {
    let legacy_relation = sqlx::query_scalar::<_, Option<String>>(
        "select to_regclass('public.sys_role_menus')::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(legacy_relation.is_none());

    let access_routes = sqlx::query_as::<_, (i64, String, String)>(
        r#"
        select menu_id, method, path_pattern
        from sys_menu_apis
        where menu_id in (1106, 1204, 1205)
        order by menu_id
        "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        access_routes,
        vec![
            (1106, "GET".to_owned(), "/api/users/{id}/access".to_owned()),
            (1204, "GET".to_owned(), "/api/roles/{id}/access".to_owned()),
            (1205, "PUT".to_owned(), "/api/roles/{id}/access".to_owned()),
        ]
    );

    let legacy_routes = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from sys_menu_apis
        where path_pattern in (
            '/api/users/{id}/permissions',
            '/api/roles/{id}/menus',
            '/api/roles/{id}/permissions'
        )
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(legacy_routes, 0);

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
