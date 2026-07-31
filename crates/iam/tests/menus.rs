use iam::Iam;

#[sqlx::test(migrations = "../../migrations")]
async fn direct_permission_does_not_create_page_access(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values (400, 'direct-only', 'direct-only', 'hash', 'Direct Only', '', 1)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
        values ('p', 'user:400', 'system:user:list', '', '', '', '')
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    let (menus, permissions) = iam.menus.current(400).await.unwrap();

    assert!(menus.is_empty());
    assert_eq!(permissions, vec!["system:user:list"]);
}
