use iam::{
    Iam,
    accounts::{AccountError, CreateAccountInput, GetUserListRequest, UpdateUserInput},
};

fn list_request() -> GetUserListRequest {
    GetUserListRequest {
        page: 1,
        page_size: 20,
        keyword: None,
        username: None,
        nick_name: None,
        phone: None,
        email: None,
        order_key: None,
        desc: None,
    }
}

fn update_input(enable: i32) -> UpdateUserInput {
    UpdateUserInput {
        nick_name: "Updated".to_string(),
        header_img: String::new(),
        enable,
        phone: None,
        email: None,
        dept_id: Some(1),
    }
}

async fn insert_user(pool: &sqlx::PgPool, id: i64, name: &str, dept_id: Option<i64>) {
    sqlx::query(
        r#"
        insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
        values ($1, $2, $2, 'hash', $2, '', $3)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(dept_id)
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_administrator_lists_only_exact_department(pool: sqlx::PgPool) {
    sqlx::query(
        "insert into sys_depts (id, parent_id, name, code) values (2, 1, 'Child', 'child')",
    )
    .execute(&pool)
    .await
    .unwrap();
    insert_user(&pool, 200, "actor", Some(1)).await;
    insert_user(&pool, 201, "same-dept", Some(1)).await;
    insert_user(&pool, 202, "child-dept", Some(2)).await;
    let iam = Iam::load(pool).await.unwrap();

    let (users, total) = iam.accounts.list(200, list_request()).await.unwrap();

    assert_eq!(total, 2);
    assert_eq!(
        users
            .into_iter()
            .map(|user| user.user_name)
            .collect::<Vec<_>>(),
        vec!["same-dept", "actor"]
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn no_department_administrator_is_self_only_and_cannot_create(pool: sqlx::PgPool) {
    insert_user(&pool, 203, "self-only", None).await;
    let iam = Iam::load(pool).await.unwrap();

    let (users, total) = iam.accounts.list(203, list_request()).await.unwrap();
    assert_eq!(total, 1);
    assert_eq!(users[0].id, 203);
    assert!(matches!(
        iam.accounts
            .create(
                203,
                CreateAccountInput {
                    user_name: "forbidden".to_string(),
                    password_hash: "hash".to_string(),
                    nick_name: "Forbidden".to_string(),
                    header_img: None,
                    role_ids: None,
                    dept_id: None,
                    enable: None,
                    phone: None,
                    email: None,
                },
            )
            .await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_administrator_may_edit_super_profile_but_not_status(pool: sqlx::PgPool) {
    insert_user(&pool, 204, "department-admin", Some(1)).await;
    insert_user(&pool, 205, "active-super", Some(1)).await;
    sqlx::query(
        "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:205', 'role:1', '', '', '', '')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool).await.unwrap();

    iam.accounts
        .update(204, 205, update_input(1))
        .await
        .unwrap();
    assert!(matches!(
        iam.accounts.update(204, 205, update_input(0)).await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_administrator_cannot_move_account_outside_exact_department(pool: sqlx::PgPool) {
    sqlx::query(
        "insert into sys_depts (id, parent_id, name, code) values (2, 1, 'Other', 'other')",
    )
    .execute(&pool)
    .await
    .unwrap();
    insert_user(&pool, 206, "department-admin", Some(1)).await;
    insert_user(&pool, 207, "same-dept", Some(1)).await;
    let iam = Iam::load(pool.clone()).await.unwrap();
    let mut payload = update_input(1);
    payload.dept_id = Some(2);

    assert!(matches!(
        iam.accounts.update(206, 207, payload).await,
        Err(AccountError::AccessDenied)
    ));
    assert_eq!(iam.accounts.info(207).await.unwrap().dept_id, Some(1));
}
