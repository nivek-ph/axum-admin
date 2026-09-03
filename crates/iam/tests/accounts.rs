use audit::{AuditActor, AuditContext, AuditSource};
use iam::{
    Iam,
    access::AccessEvaluationError,
    accounts::{AccountError, CreateAccountInput, GetUserListRequest, UpdateUserInput},
};

fn audit_context(actor_id: i64) -> AuditContext {
    AuditContext {
        req_id: format!("account-{actor_id}"),
        actor: AuditActor {
            id: Some(actor_id),
            label: format!("user-{actor_id}"),
        },
        source: AuditSource {
            ip: "127.0.0.1".to_string(),
            user_agent: "account-test".to_string(),
        },
    }
}

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
                audit_context(203),
            )
            .await,
        Err(AccountError::AccessDenied)
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_administrator_may_edit_and_disable_final_super_account(pool: sqlx::PgPool) {
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
    iam.accounts
        .update(204, 205, update_input(0))
        .await
        .unwrap();
    assert_eq!(iam.accounts.info(205).await.unwrap().enable, 2);
    assert!(matches!(
        iam.access.require_active_user(205).await,
        Err(AccessEvaluationError::UserDisabled)
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

#[sqlx::test(migrations = "../../migrations")]
async fn ordinary_administrator_user_views_do_not_expose_assigned_roles(pool: sqlx::PgPool) {
    insert_user(&pool, 208, "department-admin", Some(1)).await;
    insert_user(&pool, 209, "same-dept", Some(1)).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 209, 2).await;
    let iam = Iam::load(pool).await.unwrap();

    let (users, _) = iam.accounts.list(208, list_request()).await.unwrap();
    let target = users.iter().find(|user| user.id == 209).unwrap();
    assert!(target.roles.is_empty());
    assert!(target.role_ids.is_empty());

    let self_view = iam.accounts.info(209).await.unwrap();
    assert!(self_view.roles.is_empty());
    assert!(self_view.role_ids.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn account_creation_allows_an_empty_initial_role_set(pool: sqlx::PgPool) {
    insert_user(&pool, 210, "department-admin", Some(1)).await;
    let iam = Iam::load(pool.clone()).await.unwrap();

    iam.accounts
        .create(
            210,
            CreateAccountInput {
                user_name: "zero-role".to_string(),
                password_hash: "hash".to_string(),
                nick_name: "Zero Role".to_string(),
                header_img: None,
                role_ids: None,
                dept_id: Some(1),
                enable: None,
                phone: None,
                email: None,
            },
            audit_context(210),
        )
        .await
        .unwrap();

    let user_id =
        sqlx::query_scalar::<_, i64>("select id from sys_users where username = 'zero-role'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let membership_count = sqlx::query_scalar::<_, i64>(
        "select count(*) from casbin_rule where ptype = 'g' and v0 = 'user:' || $1::text",
    )
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(membership_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn invalid_initial_role_is_rejected_before_account_insert(pool: sqlx::PgPool) {
    insert_user(&pool, 211, "creation-super", Some(1)).await;
    assign_role(&pool, 211, 1).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status) values (2, 'disabled-role', 'Disabled Role', 'disabled')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool.clone()).await.unwrap();

    let result = iam
        .accounts
        .create(
            211,
            CreateAccountInput {
                user_name: "must-not-exist".to_string(),
                password_hash: "hash".to_string(),
                nick_name: "Must Not Exist".to_string(),
                header_img: None,
                role_ids: Some(vec![2]),
                dept_id: Some(1),
                enable: None,
                phone: None,
                email: None,
            },
            audit_context(211),
        )
        .await;

    assert!(matches!(result, Err(AccountError::InvalidRoles)));
    let exists = sqlx::query_scalar::<_, bool>(
        "select exists(select 1 from sys_users where username = 'must-not-exist')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!exists);
}

#[sqlx::test(migrations = "../../migrations")]
async fn initial_role_assignment_is_audited_after_policy_commit(pool: sqlx::PgPool) {
    insert_user(&pool, 212, "audit-super", Some(1)).await;
    assign_role(&pool, 212, 1).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status) values (2, 'audited-role', 'Audited Role', 'enabled')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let iam = Iam::load(pool.clone()).await.unwrap();

    iam.accounts
        .create(
            212,
            CreateAccountInput {
                user_name: "audited-user".to_string(),
                password_hash: "hash".to_string(),
                nick_name: "Audited User".to_string(),
                header_img: None,
                role_ids: Some(vec![2]),
                dept_id: Some(1),
                enable: None,
                phone: None,
                email: None,
            },
            audit_context(212),
        )
        .await
        .unwrap();

    let action = sqlx::query_scalar::<_, String>(
        "select action from sys_audit_events where action = 'user.assign_roles'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(action, "user.assign_roles");
}

#[sqlx::test(migrations = "../../migrations")]
async fn deleting_an_account_cleans_authorization_rows(pool: sqlx::PgPool) {
    insert_user(&pool, 211, "super-user", Some(1)).await;
    insert_user(&pool, 212, "target-user", Some(1)).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 211, 1).await;
    assign_role(&pool, 212, 2).await;
    let iam = Iam::load(pool.clone()).await.unwrap();

    iam.accounts.delete(211, 212).await.unwrap();

    let policy_count =
        sqlx::query_scalar::<_, i64>("select count(*) from casbin_rule where v0 = 'user:212'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(policy_count, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn ensure_admin_preserves_existing_role_memberships(pool: sqlx::PgPool) {
    insert_user(&pool, 213, "bootstrap-admin", Some(1)).await;
    sqlx::query(
        "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
    )
    .execute(&pool)
    .await
    .unwrap();
    assign_role(&pool, 213, 2).await;
    let iam = Iam::load(pool).await.unwrap();

    iam.accounts
        .ensure_admin("bootstrap-admin", "new-hash".to_string(), "Bootstrap Admin")
        .await
        .unwrap();

    assert_eq!(
        iam.accounts
            .access(213, 213)
            .await
            .unwrap()
            .assigned_roles
            .into_iter()
            .map(|role| role.id)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}
