use admin_httpz::AppError;
use auth::{
    jwt::JwtService,
    password::{AuthError, PasswordService},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::authority;
use crate::data_scope::DataScopeFilter;
use crate::errors;
use crate::roles::RoleSummary;

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: i64,
    pub uuid: String,
    pub username: String,
    pub password_hash: String,
    pub nick_name: String,
    pub header_img: String,
    pub authority_id: i64,
    pub authority_name: String,
    pub default_router: String,
    pub enable: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub origin_setting: Option<serde_json::Value>,
    pub dept_id: Option<i64>,
    pub dept_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterRequest {
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "passWord")]
    pub password: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: Option<String>,
    #[serde(rename = "authorityId")]
    pub authority_id: Option<i64>,
    #[serde(rename = "roleIds")]
    pub role_ids: Option<Vec<i64>>,
    #[serde(rename = "deptId", alias = "dept_id")]
    pub dept_id: Option<i64>,
    pub enable: Option<i32>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: String,
    pub enable: i32,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "deptId", alias = "dept_id")]
    pub dept_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChangePasswordRequest {
    pub password: String,
    #[serde(rename = "newPassword")]
    pub new_password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSelfInfoRequest {
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    #[serde(rename = "headerImg")]
    pub header_img: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSelfSettingRequest {
    #[serde(flatten)]
    pub origin_setting: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteUserRequest {
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResetPasswordRequest {
    #[serde(rename = "ID")]
    pub id: i64,
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetUserRolesRequest {
    #[serde(rename = "roleIds", alias = "role_ids")]
    pub role_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetUserListRequest {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub username: Option<String>,
    #[serde(rename = "nickName")]
    pub nick_name: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "orderKey")]
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserInfoView {
    #[serde(rename = "ID")]
    pub id: i64,
    pub uuid: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: String,
    pub authority: authority::AuthorityView,
    pub authorities: Vec<authority::AuthorityView>,
    pub enable: i32,
    pub phone: String,
    pub email: String,
    #[serde(rename = "originSetting")]
    pub origin_setting: Option<serde_json::Value>,
    #[serde(rename = "deptId")]
    pub dept_id: Option<i64>,
    #[serde(rename = "deptName")]
    pub dept_name: String,
    pub roles: Vec<RoleSummary>,
    #[serde(rename = "roleIds")]
    pub role_ids: Vec<i64>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub authority_id: i64,
    pub user: UserInfoView,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginResult {
    pub user: UserInfoView,
    pub token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    #[error("invalid username or password")]
    InvalidCredentials,
    #[error("user is disabled")]
    Disabled,
    #[error("user not found")]
    UserNotFound,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("invalid password")]
    InvalidPassword,
    #[error("{0}")]
    Auth(#[from] AuthError),
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

impl From<LoginError> for AppError {
    fn from(error: LoginError) -> Self {
        match error {
            LoginError::InvalidCredentials => errors::users::INVALID_CREDENTIALS.into(),
            LoginError::Disabled => errors::users::USER_DISABLED.into(),
            LoginError::UserNotFound => errors::users::USER_NOT_FOUND.into(),
            LoginError::UserAlreadyExists => errors::users::USER_ALREADY_EXISTS.into(),
            LoginError::InvalidPassword => errors::users::INVALID_PASSWORD.into(),
            LoginError::Auth(error) => errors::users::USER_OPERATION_FAILED
                .into_error()
                .with_source(error),
            LoginError::Database(error) => errors::users::USER_OPERATION_FAILED
                .into_error()
                .with_source(error),
        }
    }
}

pub async fn ensure_admin_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    username: &str,
    password: &str,
    nick_name: &str,
) -> Result<(), sqlx::Error> {
    ensure_user_with_role(
        pool,
        password_service,
        username,
        password,
        nick_name,
        1,
        true,
    )
    .await
}

pub async fn ensure_builtin_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    username: &str,
    password: &str,
    nick_name: &str,
    role_id: i64,
) -> Result<(), sqlx::Error> {
    ensure_user_with_role(
        pool,
        password_service,
        username,
        password,
        nick_name,
        role_id,
        false,
    )
    .await
}

async fn ensure_user_with_role(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    username: &str,
    password: &str,
    nick_name: &str,
    role_id: i64,
    is_system: bool,
) -> Result<(), sqlx::Error> {
    let role = crate::roles::find(pool, role_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    let user_id = if let Some(existing) = find_by_username(pool, username).await? {
        sqlx::query(
            r#"
            update sys_users
            set nick_name = $1,
                authority_id = $2,
                authority_name = $3,
                default_router = $4,
                enable = true,
                dept_id = 1,
                is_system = $5,
                updated_at = now()
            where id = $6
            "#,
        )
        .bind(nick_name)
        .bind(role.id)
        .bind(&role.name)
        .bind("dashboard")
        .bind(is_system)
        .bind(existing.id)
        .execute(pool)
        .await?;
        existing.id
    } else {
        let password_hash = password_service
            .hash_password(password)
            .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

        sqlx::query_scalar(
            r#"
        insert into sys_users (
            uuid,
            username,
            password_hash,
            nick_name,
            header_img,
            authority_id,
            authority_name,
            default_router,
            enable,
            phone,
            email,
            origin_setting,
            dept_id,
            is_system
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, true, null, null, null, 1, $9)
        returning id
        "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(username)
        .bind(password_hash)
        .bind(nick_name)
        .bind("https://qmplusimg.henrongyi.top/gva_header.jpg")
        .bind(role.id)
        .bind(&role.name)
        .bind("dashboard")
        .bind(is_system)
        .fetch_one(pool)
        .await?
    };

    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        insert into sys_user_roles (user_id, role_id)
        values ($1, $2)
        on conflict do nothing
        "#,
    )
    .bind(user_id)
    .bind(role.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn register_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: RegisterRequest,
) -> Result<(), LoginError> {
    if find_by_username(pool, &payload.user_name).await?.is_some() {
        return Err(LoginError::UserAlreadyExists);
    }
    let role_ids = normalize_role_ids(payload.role_ids.as_ref(), payload.authority_id);
    let authority = resolve_role_authority(pool, role_ids[0]).await?;
    let password_hash = password_service.hash_password(&payload.password)?;

    let user_id: i64 = sqlx::query_scalar(
        r#"
        insert into sys_users (
            uuid,
            username,
            password_hash,
            nick_name,
            header_img,
            authority_id,
            authority_name,
            default_router,
            enable,
            phone,
            email,
            origin_setting,
            dept_id
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, null, $12)
        returning id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&payload.user_name)
    .bind(password_hash)
    .bind(&payload.nick_name)
    .bind(
        payload
            .header_img
            .unwrap_or_else(|| "https://qmplusimg.henrongyi.top/gva_header.jpg".to_string()),
    )
    .bind(authority.authority_id)
    .bind(&authority.authority_name)
    .bind(&authority.default_router)
    .bind(payload.enable.unwrap_or(1) == 1)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.dept_id.or(Some(1)))
    .fetch_one(pool)
    .await?;

    replace_user_roles(pool, user_id, role_ids).await?;

    Ok(())
}

pub async fn login(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    jwt_service: &JwtService,
    payload: LoginRequest,
) -> Result<LoginResult, LoginError> {
    let record = find_by_username(pool, &payload.username)
        .await?
        .ok_or(LoginError::InvalidCredentials)?;

    if !record.enable {
        return Err(LoginError::Disabled);
    }

    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(LoginError::InvalidCredentials);
    }

    let token = jwt_service.issue_token(record.id, &record.username, record.authority_id)?;

    let roles = get_roles_by_user_id(pool, record.id).await?;
    let permissions = get_permission_codes_by_user_id(pool, record.id).await?;

    Ok(LoginResult {
        user: build_user_info(&record, roles, permissions),
        token,
    })
}

pub async fn load_authenticated_user(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<AuthenticatedUser, LoginError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(LoginError::UserNotFound)?;

    let roles = get_roles_by_user_id(pool, record.id).await?;
    let permissions = get_permission_codes_by_user_id(pool, record.id).await?;

    Ok(AuthenticatedUser {
        id: record.id,
        authority_id: record.authority_id,
        user: build_user_info(&record, roles, permissions),
    })
}

pub async fn get_user_list(
    pool: &sqlx::PgPool,
    query: GetUserListRequest,
    actor_user_id: Option<i64>,
) -> Result<(Vec<UserInfoView>, i64), LoginError> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let order_key = match query.order_key.as_deref() {
        Some("username") => "u.username",
        Some("nick_name") => "u.nick_name",
        Some("phone") => "u.phone",
        Some("email") => "u.email",
        _ => "u.id",
    };
    let order_dir = if query.desc.unwrap_or(true) {
        "desc"
    } else {
        "asc"
    };
    let order_clause = format!("{order_key} {order_dir}");
    let scope_filter = match actor_user_id {
        Some(user_id) => crate::data_scope::resolve_user_data_scope(pool, user_id, "users").await?,
        None => DataScopeFilter::All,
    };
    if matches!(&scope_filter, DataScopeFilter::DeptIds(dept_ids) if dept_ids.is_empty()) {
        return Ok((Vec::new(), 0));
    }
    let scope_clause = scope_sql_clause(&scope_filter);

    let total_sql = format!(
        r#"
        select count(*) from sys_users u
        where ($1::text is null or u.username ilike '%' || $1 || '%')
          and ($2::text is null or u.nick_name ilike '%' || $2 || '%')
          and ($3::text is null or coalesce(u.phone, '') ilike '%' || $3 || '%')
          and ($4::text is null or coalesce(u.email, '') ilike '%' || $4 || '%')
          {scope_clause}
        "#
    );
    let mut total_query = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(total_sql))
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref());
    total_query = match &scope_filter {
        DataScopeFilter::All => total_query,
        DataScopeFilter::DeptIds(dept_ids) => total_query.bind(dept_ids),
        DataScopeFilter::Owner(owner_id) => total_query.bind(owner_id),
    };
    let total = total_query.fetch_one(pool).await?;

    let sql = format!(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.authority_id,
            u.authority_name,
            u.default_router,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where ($1::text is null or u.username ilike '%' || $1 || '%')
          and ($2::text is null or u.nick_name ilike '%' || $2 || '%')
          and ($3::text is null or coalesce(u.phone, '') ilike '%' || $3 || '%')
          and ($4::text is null or coalesce(u.email, '') ilike '%' || $4 || '%')
          {scope_clause}
        order by {order_clause}
        limit ${limit_placeholder} offset ${offset_placeholder}
        "#,
        limit_placeholder = if matches!(&scope_filter, DataScopeFilter::All) {
            5
        } else {
            6
        },
        offset_placeholder = if matches!(&scope_filter, DataScopeFilter::All) {
            6
        } else {
            7
        }
    );

    let mut rows_query = sqlx::query_as::<_, UserRecord>(sqlx::AssertSqlSafe(sql))
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref());
    rows_query = match &scope_filter {
        DataScopeFilter::All => rows_query,
        DataScopeFilter::DeptIds(dept_ids) => rows_query.bind(dept_ids),
        DataScopeFilter::Owner(owner_id) => rows_query.bind(owner_id),
    };
    let rows = rows_query
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let mut list = Vec::with_capacity(rows.len());
    for record in rows {
        let roles = get_roles_by_user_id(pool, record.id).await?;
        let permissions = get_permission_codes_by_user_id(pool, record.id).await?;
        list.push(build_user_info(&record, roles, permissions));
    }

    Ok((list, total))
}

fn scope_sql_clause(filter: &DataScopeFilter) -> &'static str {
    match filter {
        DataScopeFilter::All => "",
        DataScopeFilter::DeptIds(_) => "and u.dept_id = any($5)",
        DataScopeFilter::Owner(_) => "and u.id = $5",
    }
}

pub async fn ensure_user_in_scope(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    target_user_id: i64,
) -> Result<(), LoginError> {
    let filter = crate::data_scope::resolve_user_data_scope(pool, actor_user_id, "users").await?;
    let visible = match filter {
        DataScopeFilter::All => true,
        DataScopeFilter::Owner(owner_id) => owner_id == target_user_id,
        DataScopeFilter::DeptIds(dept_ids) => {
            if dept_ids.is_empty() {
                false
            } else {
                sqlx::query_scalar::<_, bool>(
                    r#"
                    select exists(
                        select 1 from sys_users
                        where id = $1 and dept_id = any($2)
                    )
                    "#,
                )
                .bind(target_user_id)
                .bind(&dept_ids)
                .fetch_one(pool)
                .await?
            }
        }
    };

    if visible {
        Ok(())
    } else {
        Err(LoginError::UserNotFound)
    }
}

pub async fn update_user(
    pool: &sqlx::PgPool,
    payload: UpdateUserRequest,
) -> Result<(), LoginError> {
    sqlx::query(
        r#"
        update sys_users
        set nick_name = $1,
            header_img = $2,
            enable = $3,
            phone = $4,
            email = $5,
            dept_id = coalesce($6, dept_id),
            updated_at = now()
        where id = $7
        "#,
    )
    .bind(payload.nick_name)
    .bind(payload.header_img)
    .bind(payload.enable == 1)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.dept_id)
    .bind(payload.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete_user(
    pool: &sqlx::PgPool,
    payload: DeleteUserRequest,
) -> Result<(), LoginError> {
    sqlx::query("delete from sys_users where id = $1")
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn reset_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: ResetPasswordRequest,
) -> Result<(), LoginError> {
    let password_hash = password_service.hash_password(&payload.password)?;
    sqlx::query(
        r#"
        update sys_users
        set password_hash = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(password_hash)
    .bind(payload.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_user_roles(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetUserRolesRequest,
) -> Result<(), LoginError> {
    let role_ids = normalize_role_ids(Some(&payload.role_ids), None);
    let authority = resolve_role_authority(pool, role_ids[0]).await?;
    replace_user_roles(pool, user_id, role_ids).await?;
    set_user_primary_authority(pool, user_id, authority).await?;
    Ok(())
}

pub async fn change_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    user_id: i64,
    payload: ChangePasswordRequest,
) -> Result<(), LoginError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(LoginError::UserNotFound)?;
    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(LoginError::InvalidPassword);
    }

    let password_hash = password_service.hash_password(&payload.new_password)?;
    sqlx::query(
        r#"
        update sys_users
        set password_hash = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_self_info(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetSelfInfoRequest,
) -> Result<(), LoginError> {
    sqlx::query(
        r#"
        update sys_users
        set nick_name = coalesce($1, nick_name),
            header_img = coalesce($2, header_img),
            phone = coalesce($3, phone),
            email = coalesce($4, email),
            updated_at = now()
        where id = $5
        "#,
    )
    .bind(payload.nick_name)
    .bind(payload.header_img)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_self_setting(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetSelfSettingRequest,
) -> Result<(), LoginError> {
    sqlx::query(
        r#"
        update sys_users
        set origin_setting = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(payload.origin_setting)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn find_by_username(
    pool: &sqlx::PgPool,
    username: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.authority_id,
            u.authority_name,
            u.default_router,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where u.username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

async fn find_by_id(pool: &sqlx::PgPool, user_id: i64) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.authority_id,
            u.authority_name,
            u.default_router,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

fn build_user_info(
    record: &UserRecord,
    roles: Vec<RoleSummary>,
    permissions: Vec<String>,
) -> UserInfoView {
    let authority = authority::AuthorityView {
        authority_id: record.authority_id,
        authority_name: record.authority_name.clone(),
        parent_id: 0,
        default_router: record.default_router.clone(),
        children: Vec::new(),
        data_authority_id: Vec::new(),
    };

    UserInfoView {
        id: record.id,
        uuid: record.uuid.clone(),
        user_name: record.username.clone(),
        nick_name: record.nick_name.clone(),
        header_img: record.header_img.clone(),
        authority: authority.clone(),
        authorities: vec![authority],
        enable: if record.enable { 1 } else { 2 },
        phone: record.phone.clone().unwrap_or_default(),
        email: record.email.clone().unwrap_or_default(),
        origin_setting: record.origin_setting.clone(),
        dept_id: record.dept_id,
        dept_name: record.dept_name.clone().unwrap_or_default(),
        role_ids: roles.iter().map(|role| role.id).collect(),
        roles,
        permissions,
    }
}

async fn get_roles_by_user_id(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<Vec<RoleSummary>, sqlx::Error> {
    sqlx::query_as::<_, RoleSummary>(
        r#"
        select r.id, r.code, r.name, r.status, r.sort, r.data_scope, r.is_system
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        where ur.user_id = $1
        order by r.sort, r.id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn get_permission_codes_by_user_id(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        select distinct p.code
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        join sys_role_permissions rp on rp.role_id = r.id
        join sys_permissions p on p.id = rp.permission_id
        where ur.user_id = $1
          and r.status = 'enabled'
          and p.status = 'enabled'
        order by p.code
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn replace_user_roles(
    pool: &sqlx::PgPool,
    user_id: i64,
    role_ids: Vec<i64>,
) -> Result<(), LoginError> {
    let normalized = if role_ids.is_empty() {
        vec![1]
    } else {
        role_ids.into_iter().collect()
    };

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    for role_id in normalized {
        sqlx::query(
            r#"
            insert into sys_user_roles (user_id, role_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

fn normalize_role_ids(role_ids: Option<&Vec<i64>>, authority_id: Option<i64>) -> Vec<i64> {
    role_ids
        .filter(|ids| !ids.is_empty())
        .cloned()
        .or_else(|| authority_id.map(|id| vec![id]))
        .unwrap_or_else(|| vec![1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_sql_clause_matches_filter_shape() {
        assert_eq!(scope_sql_clause(&DataScopeFilter::All), "");
        assert_eq!(
            scope_sql_clause(&DataScopeFilter::DeptIds(vec![1, 2])),
            "and u.dept_id = any($5)"
        );
        assert_eq!(
            scope_sql_clause(&DataScopeFilter::Owner(7)),
            "and u.id = $5"
        );
    }

    #[test]
    fn normalize_role_ids_uses_current_role_ids() {
        assert_eq!(normalize_role_ids(None, Some(1)), vec![1]);
        assert_eq!(normalize_role_ids(Some(&vec![1, 2]), None), vec![1, 2]);
    }
}

async fn resolve_role_authority(
    pool: &sqlx::PgPool,
    role_id: i64,
) -> Result<authority::AuthorityView, LoginError> {
    let role = crate::roles::find(pool, role_id)
        .await?
        .ok_or(LoginError::UserNotFound)?;

    Ok(authority::AuthorityView {
        authority_id: role.id,
        authority_name: role.name,
        parent_id: 0,
        default_router: "dashboard".to_string(),
        children: Vec::new(),
        data_authority_id: Vec::new(),
    })
}

async fn set_user_primary_authority(
    pool: &sqlx::PgPool,
    user_id: i64,
    authority: authority::AuthorityView,
) -> Result<(), LoginError> {
    sqlx::query(
        r#"
        update sys_users
        set authority_id = $1,
            authority_name = $2,
            default_router = $3,
            updated_at = now()
        where id = $4
        "#,
    )
    .bind(authority.authority_id)
    .bind(authority.authority_name)
    .bind(authority.default_router)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
