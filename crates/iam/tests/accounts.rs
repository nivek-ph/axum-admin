use iam::{
    accounts::{Accounts, CreateAccountInput, PreparedPasswordUpdate},
    authorization::Authorization,
};

#[sqlx::test(migrations = "../../migrations")]
async fn accounts_persists_a_prepared_password_hash(pool: sqlx::PgPool) {
    sqlx::query(
        r#"
        insert into sys_users (
            id, uuid, username, password_hash, nick_name, header_img, home_route,
            enable, dept_id, is_system
        )
        values (701, 'prepared-password-user', 'prepared-password-user', 'old-hash',
                'Prepared Password User', '', 'dashboard', true, 1, false)
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to insert prepared password user");
    let accounts = Accounts::new(pool.clone(), Authorization::new(pool.clone()));

    accounts
        .persist_password_update(PreparedPasswordUpdate::new(
            701,
            "prepared-password-hash".to_string(),
        ))
        .await
        .unwrap();

    let stored_hash =
        sqlx::query_scalar::<_, String>("select password_hash from sys_users where id = 701")
            .fetch_one(&pool)
            .await
            .expect("failed to fetch stored hash");
    assert_eq!(stored_hash, "prepared-password-hash");
}

fn create_account_input(username: &str) -> CreateAccountInput {
    CreateAccountInput {
        user_name: username.to_string(),
        password_hash: "prepared-create-hash".to_string(),
        nick_name: "Created Account".to_string(),
        header_img: None,
        role_ids: Some(vec![2]),
        dept_id: Some(1),
        enable: Some(1),
        phone: None,
        email: None,
    }
}

#[sqlx::test(migrations = "../../migrations")]
async fn account_creation_persists_the_prepared_hash_and_initial_membership_atomically(
    pool: sqlx::PgPool,
) {
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
         values (2, 'account_creator', 'Account Creator', 'enabled', 10, 'self', false)",
    )
    .execute(&pool)
    .await
    .expect("failed to insert account creator role");
    let authorization = Authorization::new(pool.clone());
    let accounts = Accounts::new(pool.clone(), authorization.clone());

    accounts
        .create(1, create_account_input("created-account"))
        .await
        .expect("failed to create account");

    let created = sqlx::query_as::<_, (i64, String)>(
        "select id, password_hash from sys_users where username = 'created-account'",
    )
    .fetch_one(&pool)
    .await
    .expect("failed to fetch created account");
    assert_eq!(created.1, "prepared-create-hash");
    assert_eq!(
        authorization
            .user_role_ids(created.0)
            .await
            .expect("failed to fetch user role ids"),
        vec![2]
    );

    let mut system_role_input = create_account_input("unauthorized-system-account");
    system_role_input.role_ids = Some(vec![1]);
    let error = accounts
        .create(created.0, system_role_input)
        .await
        .expect_err("ordinary role members must not assign the system role");
    assert!(matches!(error, iam::accounts::AccountError::InvalidRoles));
    let exists = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from sys_users where username = 'unauthorized-system-account')",
    )
    .fetch_one(&pool)
    .await
    .expect("failed to fetch exists");
    assert!(!exists);

    sqlx::query(
        r#"
        create function fail_initial_membership() returns trigger language plpgsql as $$
        begin
            if new.ptype = 'g' then
                raise exception 'membership insert failed';
            end if;
            return new;
        end;
        $$
        "#,
    )
    .execute(&pool)
    .await
    .expect("failed to create fail_initial_membership function");
    sqlx::query(
        "create trigger fail_initial_membership before insert on casbin_rule
         for each row execute function fail_initial_membership()",
    )
    .execute(&pool)
    .await
    .expect("failed to create fail_initial_membership trigger");

    accounts
        .create(1, create_account_input("rolled-back-account"))
        .await
        .expect_err("membership failure should fail account creation");
    let exists = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from sys_users where username = 'rolled-back-account')",
    )
    .fetch_one(&pool)
    .await
    .expect("failed to fetch exists");
    assert!(!exists);
}
