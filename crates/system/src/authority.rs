use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use admin_httpz::AppError;

use crate::errors;

pub const SUPER_ADMIN_AUTHORITY_ID: i64 = 888;

#[derive(Debug, Clone, FromRow)]
pub struct AuthorityRecord {
    pub authority_id: i64,
    pub authority_name: String,
    pub parent_id: i64,
    pub default_router: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorityView {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "authorityName")]
    pub authority_name: String,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    #[serde(rename = "defaultRouter")]
    pub default_router: String,
    pub children: Vec<AuthorityView>,
    #[serde(rename = "dataAuthorityId")]
    pub data_authority_id: Vec<AuthorityDataView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthorityDataView {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "authorityName")]
    pub authority_name: String,
}

pub fn default_authorities() -> Vec<AuthorityView> {
    vec![AuthorityView {
        authority_id: SUPER_ADMIN_AUTHORITY_ID,
        authority_name: "Super Admin".to_string(),
        parent_id: 0,
        default_router: "dashboard".to_string(),
        children: Vec::new(),
        data_authority_id: Vec::new(),
    }]
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateAuthorityRequest {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "authorityName")]
    pub authority_name: String,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAuthorityRequest {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "authorityName", alias = "AuthorityName")]
    pub authority_name: String,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    #[serde(rename = "defaultRouter")]
    pub default_router: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteAuthorityRequest {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CopyAuthorityRequest {
    pub authority: CreateAuthorityRequest,
    #[serde(rename = "oldAuthorityId")]
    pub old_authority_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetRoleUsersRequest {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "userIds")]
    pub user_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthorityError {
    #[error("role already exists")]
    AlreadyExists,
    #[error("default role cannot be deleted")]
    CannotDeleteRoot,
    #[error("role not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

impl From<AuthorityError> for AppError {
    fn from(error: AuthorityError) -> Self {
        match error {
            AuthorityError::AlreadyExists => errors::authority::AUTHORITY_EXISTS.into(),
            AuthorityError::CannotDeleteRoot => errors::authority::ROOT_AUTHORITY_IMMUTABLE.into(),
            AuthorityError::NotFound => errors::authority::AUTHORITY_NOT_FOUND.into(),
            AuthorityError::Database(error) => errors::authority::AUTHORITY_DB_FAILED
                .into_error()
                .with_source(error),
        }
    }
}

pub async fn ensure_default_authority(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_authorities (authority_id, authority_name, parent_id, default_router)
        values (888, 'Super Admin', 0, 'dashboard')
        on conflict (authority_id) do update
        set authority_name = excluded.authority_name,
            parent_id = excluded.parent_id,
            default_router = excluded.default_router
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        update sys_users
        set authority_name = 'Super Admin',
            default_router = 'dashboard'
        where authority_id = 888
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_authority_info_list(
    pool: &sqlx::PgPool,
) -> Result<Vec<AuthorityView>, AuthorityError> {
    let rows = sqlx::query_as::<_, AuthorityRecord>(
        r#"
        select authority_id, authority_name, parent_id, default_router
        from sys_authorities
        order by authority_id asc
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(build_tree(&rows, 0))
}

pub async fn create_authority(
    pool: &sqlx::PgPool,
    payload: CreateAuthorityRequest,
) -> Result<AuthorityView, AuthorityError> {
    let exists: Option<i64> =
        sqlx::query_scalar("select authority_id from sys_authorities where authority_id = $1")
            .bind(payload.authority_id)
            .fetch_optional(pool)
            .await?;
    if exists.is_some() {
        return Err(AuthorityError::AlreadyExists);
    }

    sqlx::query(
        r#"
        insert into sys_authorities (authority_id, authority_name, parent_id, default_router)
        values ($1, $2, $3, 'dashboard')
        "#,
    )
    .bind(payload.authority_id)
    .bind(&payload.authority_name)
    .bind(payload.parent_id)
    .execute(pool)
    .await?;

    Ok(AuthorityView {
        authority_id: payload.authority_id,
        authority_name: payload.authority_name,
        parent_id: payload.parent_id,
        default_router: "dashboard".to_string(),
        children: Vec::new(),
        data_authority_id: Vec::new(),
    })
}

pub async fn update_authority(
    pool: &sqlx::PgPool,
    payload: UpdateAuthorityRequest,
) -> Result<AuthorityView, AuthorityError> {
    let current = get_authority_record(pool, payload.authority_id)
        .await?
        .ok_or(AuthorityError::NotFound)?;
    let default_router = payload
        .default_router
        .unwrap_or_else(|| current.default_router.clone());

    sqlx::query(
        r#"
        update sys_authorities
        set authority_name = $1,
            parent_id = $2,
            default_router = $3
        where authority_id = $4
        "#,
    )
    .bind(&payload.authority_name)
    .bind(payload.parent_id)
    .bind(&default_router)
    .bind(payload.authority_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        update sys_users
        set authority_name = $1,
            default_router = $2
        where authority_id = $3
        "#,
    )
    .bind(&payload.authority_name)
    .bind(&default_router)
    .bind(payload.authority_id)
    .execute(pool)
    .await?;

    Ok(AuthorityView {
        authority_id: payload.authority_id,
        authority_name: payload.authority_name,
        parent_id: payload.parent_id,
        default_router,
        children: Vec::new(),
        data_authority_id: Vec::new(),
    })
}

pub async fn delete_authority(
    pool: &sqlx::PgPool,
    payload: DeleteAuthorityRequest,
) -> Result<(), AuthorityError> {
    if payload.authority_id == 888 {
        return Err(AuthorityError::CannotDeleteRoot);
    }

    sqlx::query("delete from sys_role_menus where authority_id = $1")
        .bind(payload.authority_id)
        .execute(pool)
        .await?;
    sqlx::query("delete from sys_authorities where authority_id = $1")
        .bind(payload.authority_id)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn copy_authority(
    pool: &sqlx::PgPool,
    payload: CopyAuthorityRequest,
) -> Result<AuthorityView, AuthorityError> {
    let new_authority = create_authority(pool, payload.authority.clone()).await?;
    let menu_ids: Vec<i64> = sqlx::query_scalar(
        "select menu_id from sys_role_menus where authority_id = $1 order by menu_id",
    )
    .bind(payload.old_authority_id)
    .fetch_all(pool)
    .await?;

    for menu_id in menu_ids {
        sqlx::query(
            r#"
            insert into sys_role_menus (authority_id, menu_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(new_authority.authority_id)
        .bind(menu_id)
        .execute(pool)
        .await?;
    }

    Ok(new_authority)
}

pub async fn get_user_ids_by_authority_id(
    pool: &sqlx::PgPool,
    authority_id: i64,
) -> Result<Vec<i64>, AuthorityError> {
    let user_ids =
        sqlx::query_scalar("select id from sys_users where authority_id = $1 order by id asc")
            .bind(authority_id)
            .fetch_all(pool)
            .await?;

    Ok(user_ids)
}

pub async fn set_role_users(
    pool: &sqlx::PgPool,
    payload: SetRoleUsersRequest,
) -> Result<(), AuthorityError> {
    let authority = get_authority_record(pool, payload.authority_id)
        .await?
        .ok_or(AuthorityError::NotFound)?;

    if payload.authority_id != 888 {
        sqlx::query(
            r#"
            update sys_users
            set authority_id = 888,
                authority_name = 'Super Admin',
                default_router = 'dashboard'
            where authority_id = $1
              and id <> all($2)
            "#,
        )
        .bind(payload.authority_id)
        .bind(&payload.user_ids)
        .execute(pool)
        .await?;
    }

    if !payload.user_ids.is_empty() {
        sqlx::query(
            r#"
            update sys_users
            set authority_id = $1,
                authority_name = $2,
                default_router = $3
            where id = any($4)
            "#,
        )
        .bind(authority.authority_id)
        .bind(authority.authority_name)
        .bind(authority.default_router)
        .bind(&payload.user_ids)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_authority_record(
    pool: &sqlx::PgPool,
    authority_id: i64,
) -> Result<Option<AuthorityRecord>, sqlx::Error> {
    sqlx::query_as::<_, AuthorityRecord>(
        r#"
        select authority_id, authority_name, parent_id, default_router
        from sys_authorities
        where authority_id = $1
        "#,
    )
    .bind(authority_id)
    .fetch_optional(pool)
    .await
}

fn build_tree(rows: &[AuthorityRecord], parent_id: i64) -> Vec<AuthorityView> {
    let mut nodes: Vec<AuthorityView> = rows
        .iter()
        .filter(|row| row.parent_id == parent_id)
        .map(|row| AuthorityView {
            authority_id: row.authority_id,
            authority_name: row.authority_name.clone(),
            parent_id: row.parent_id,
            default_router: row.default_router.clone(),
            children: build_tree(rows, row.authority_id),
            data_authority_id: Vec::new(),
        })
        .collect();

    nodes.sort_by_key(|item| item.authority_id);
    nodes
}
