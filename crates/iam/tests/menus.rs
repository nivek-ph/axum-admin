use iam::Iam;

#[sqlx::test(migrations = "../../migrations")]
async fn action_permission_without_owning_page_is_rejected_at_startup(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values (400, 'action-only', 'action-only', 'hash', 'Action Only', '', 1)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into sys_roles (id, name, code, status)
        values (400, 'Action Role', 'action-role', 'enabled')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values
            ('g', 'user:400', 'role:400', '', '', '', ''),
            ('p', 'role:400', 'system:user:create', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(Iam::load(pool).await.is_err());
}
