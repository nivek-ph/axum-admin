use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use admin_httpz::AppError;

use crate::errors;

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct ApiRecord {
    #[serde(rename = "ID")]
    pub id: i64,
    pub path: String,
    pub description: String,
    #[serde(rename = "apiGroup")]
    pub api_group: String,
    pub method: String,
}

#[derive(Debug, Clone, FromRow)]
struct ApiRoleMatrixRow {
    pub path: String,
    pub method: String,
    pub authority_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchApiRequest {
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub path: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "apiGroup")]
    pub api_group: Option<String>,
    pub method: Option<String>,
    #[serde(rename = "orderKey")]
    pub order_key: Option<String>,
    pub desc: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiIdRequest {
    #[serde(rename = "ID", alias = "id")]
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteApisByIdsRequest {
    pub ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiPayload {
    #[serde(rename = "ID", default)]
    pub id: i64,
    pub path: String,
    pub description: String,
    #[serde(rename = "apiGroup")]
    pub api_group: String,
    pub method: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiRoleQuery {
    pub path: String,
    pub method: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthorityApiQuery {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetApiRolesRequest {
    pub path: String,
    pub method: String,
    #[serde(rename = "roleIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiRoleSelection {
    #[serde(rename = "roleIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ApiRoleMatrixItem {
    pub path: String,
    pub method: String,
    #[serde(rename = "roleIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("API already exists")]
    AlreadyExists,
    #[error("API not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
}

impl From<ApiError> for AppError {
    fn from(error: ApiError) -> Self {
        match error {
            ApiError::AlreadyExists => errors::api_registry::API_EXISTS.into(),
            ApiError::NotFound => errors::api_registry::API_NOT_FOUND.into(),
            ApiError::Database(error) => errors::api_registry::API_DB_FAILED
                .into_error()
                .with_source(error),
        }
    }
}

pub async fn get_api_list(
    pool: &sqlx::PgPool,
    query: SearchApiRequest,
) -> Result<(Vec<ApiRecord>, i64), ApiError> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let order_key = match query.order_key.as_deref() {
        Some("path") => "path",
        Some("api_group") => "api_group",
        Some("description") => "description",
        Some("method") => "method",
        _ => "id",
    };
    let order_dir = if query.desc.unwrap_or(true) {
        "desc"
    } else {
        "asc"
    };
    let order_clause = format!("{order_key} {order_dir}");

    let total: i64 = sqlx::query_scalar(
        r#"
        select count(*) from sys_apis
        where ($1::text is null or path ilike '%' || $1 || '%')
          and ($2::text is null or description ilike '%' || $2 || '%')
          and ($3::text is null or api_group = $3)
          and ($4::text is null or method = $4)
        "#,
    )
    .bind(query.path.as_deref())
    .bind(query.description.as_deref())
    .bind(query.api_group.as_deref())
    .bind(query.method.as_deref())
    .fetch_one(pool)
    .await?;

    let sql = format!(
        r#"
        select id, path, description, api_group, method
        from sys_apis
        where ($1::text is null or path ilike '%' || $1 || '%')
          and ($2::text is null or description ilike '%' || $2 || '%')
          and ($3::text is null or api_group = $3)
          and ($4::text is null or method = $4)
        order by {order_clause}
        limit $5 offset $6
        "#
    );

    let list = sqlx::query_as::<_, ApiRecord>(sqlx::AssertSqlSafe(sql))
        .bind(query.path.as_deref())
        .bind(query.description.as_deref())
        .bind(query.api_group.as_deref())
        .bind(query.method.as_deref())
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    Ok((list, total))
}

pub async fn create_api(pool: &sqlx::PgPool, payload: ApiPayload) -> Result<(), ApiError> {
    let exists: Option<i64> =
        sqlx::query_scalar("select id from sys_apis where path = $1 and method = $2")
            .bind(&payload.path)
            .bind(&payload.method)
            .fetch_optional(pool)
            .await?;
    if exists.is_some() {
        return Err(ApiError::AlreadyExists);
    }

    sqlx::query(
        r#"
        insert into sys_apis (path, description, api_group, method)
        values ($1, $2, $3, $4)
        "#,
    )
    .bind(payload.path)
    .bind(payload.description)
    .bind(payload.api_group)
    .bind(payload.method)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_api(pool: &sqlx::PgPool, payload: ApiPayload) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        update sys_apis
        set path = $1, description = $2, api_group = $3, method = $4
        where id = $5
        "#,
    )
    .bind(payload.path)
    .bind(payload.description)
    .bind(payload.api_group)
    .bind(payload.method)
    .bind(payload.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_api(pool: &sqlx::PgPool, api_id: i64) -> Result<(), ApiError> {
    sqlx::query("delete from sys_apis where id = $1")
        .bind(api_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_apis_by_ids(
    pool: &sqlx::PgPool,
    payload: DeleteApisByIdsRequest,
) -> Result<(), ApiError> {
    if payload.ids.is_empty() {
        return Ok(());
    }
    sqlx::query("delete from sys_apis where id = any($1)")
        .bind(&payload.ids)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_api_by_id(pool: &sqlx::PgPool, api_id: i64) -> Result<ApiRecord, ApiError> {
    sqlx::query_as::<_, ApiRecord>(
        "select id, path, description, api_group, method from sys_apis where id = $1",
    )
    .bind(api_id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

pub async fn get_all_apis(pool: &sqlx::PgPool) -> Result<Vec<ApiRecord>, ApiError> {
    Ok(sqlx::query_as::<_, ApiRecord>(
        "select id, path, description, api_group, method from sys_apis order by api_group, path, method",
    )
    .fetch_all(pool)
    .await?)
}

pub async fn get_api_groups(
    pool: &sqlx::PgPool,
) -> Result<(Vec<String>, BTreeMap<String, String>), ApiError> {
    let groups: Vec<String> = sqlx::query_scalar(
        "select distinct api_group from sys_apis where api_group <> '' order by api_group",
    )
    .fetch_all(pool)
    .await?;

    let api_group_map = groups
        .iter()
        .map(|group| (group.clone(), group.clone()))
        .collect::<BTreeMap<_, _>>();

    Ok((groups, api_group_map))
}

pub async fn get_api_roles(
    pool: &sqlx::PgPool,
    query: ApiRoleQuery,
) -> Result<ApiRoleSelection, ApiError> {
    let authority_ids = sqlx::query_scalar(
        r#"
        select rp.role_id
        from sys_permission_apis pa
        join sys_role_permissions rp on rp.permission_id = pa.permission_id
        join sys_roles r on r.id = rp.role_id
        where pa.path_pattern = $1
          and pa.method = $2
          and r.status = 'enabled'
        order by rp.role_id
        "#,
    )
    .bind(&query.path)
    .bind(&query.method)
    .fetch_all(pool)
    .await?;

    Ok(ApiRoleSelection { authority_ids })
}

pub async fn get_apis_by_authority_id(
    pool: &sqlx::PgPool,
    authority_id: i64,
) -> Result<Vec<ApiRecord>, ApiError> {
    Ok(sqlx::query_as::<_, ApiRecord>(
        r#"
        select distinct a.id, a.path, a.description, a.api_group, a.method
        from sys_apis a
        join sys_permission_apis pa on pa.path_pattern = a.path and pa.method = a.method
        join sys_role_permissions rp on rp.permission_id = pa.permission_id and rp.role_id = $1
        join sys_roles r on r.id = rp.role_id
        where r.status = 'enabled'
        order by a.api_group, a.path, a.method
        "#,
    )
    .bind(authority_id)
    .fetch_all(pool)
    .await?)
}

pub async fn get_api_role_matrix(pool: &sqlx::PgPool) -> Result<Vec<ApiRoleMatrixItem>, ApiError> {
    let rows = sqlx::query_as::<_, ApiRoleMatrixRow>(
        r#"
        select
            a.path,
            a.method,
            case when r.id is not null then rp.role_id end as authority_id
        from sys_apis a
        left join sys_permission_apis pa on pa.path_pattern = a.path and pa.method = a.method
        left join sys_role_permissions rp on rp.permission_id = pa.permission_id
        left join sys_roles r on r.id = rp.role_id and r.status = 'enabled'
        order by a.api_group, a.path, a.method, rp.role_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(merge_api_role_matrix(&rows))
}

fn merge_api_role_matrix(rows: &[ApiRoleMatrixRow]) -> Vec<ApiRoleMatrixItem> {
    let mut current_key: Option<(String, String)> = None;
    let mut current_ids = BTreeSet::new();
    let mut rows = rows.iter().collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        (left.path.as_str(), left.method.as_str(), left.authority_id).cmp(&(
            right.path.as_str(),
            right.method.as_str(),
            right.authority_id,
        ))
    });
    let mut items = Vec::new();

    for row in rows {
        let key = (row.path.clone(), row.method.clone());
        if current_key.as_ref() != Some(&key) {
            if let Some((path, method)) = current_key.take() {
                items.push(ApiRoleMatrixItem {
                    path,
                    method,
                    authority_ids: current_ids.iter().copied().collect(),
                });
                current_ids.clear();
            }
            current_key = Some(key);
        }
        if let Some(authority_id) = row.authority_id {
            current_ids.insert(authority_id);
        }
    }

    if let Some((path, method)) = current_key {
        items.push(ApiRoleMatrixItem {
            path,
            method,
            authority_ids: current_ids.iter().copied().collect(),
        });
    }

    items
}

fn normalize_api_role_ids(role_ids: Vec<i64>) -> Vec<i64> {
    role_ids
        .into_iter()
        .filter(|role_id| *role_id > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub async fn set_api_roles(
    pool: &sqlx::PgPool,
    payload: SetApiRolesRequest,
) -> Result<(), ApiError> {
    let authority_ids = normalize_api_role_ids(payload.authority_ids);
    let api_exists: bool =
        sqlx::query_scalar("select exists(select 1 from sys_apis where path = $1 and method = $2)")
            .bind(&payload.path)
            .bind(&payload.method)
            .fetch_one(pool)
            .await?;
    if !api_exists {
        return Err(ApiError::NotFound);
    }
    sync_api_permission_roles(pool, &payload.path, &payload.method, &authority_ids).await?;

    Ok(())
}

async fn sync_api_permission_roles(
    pool: &sqlx::PgPool,
    path: &str,
    method: &str,
    role_ids: &[i64],
) -> Result<(), ApiError> {
    let permission_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        select permission_id
        from sys_permission_apis
        where path_pattern = $1
          and method = $2
        order by permission_id
        "#,
    )
    .bind(path)
    .bind(method)
    .fetch_all(pool)
    .await?;
    if permission_ids.is_empty() {
        return Ok(());
    }

    let existing_role_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        select id
        from sys_roles
        where id = any($1)
        order by id
        "#,
    )
    .bind(role_ids)
    .fetch_all(pool)
    .await?;

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_role_permissions where permission_id = any($1)")
        .bind(&permission_ids)
        .execute(&mut *tx)
        .await?;
    for role_id in existing_role_ids {
        for permission_id in &permission_ids {
            sqlx::query(
                r#"
                insert into sys_role_permissions (role_id, permission_id)
                values ($1, $2)
                on conflict do nothing
                "#,
            )
            .bind(role_id)
            .bind(*permission_id)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_api_role_ids_deduplicates_and_filters_invalid_ids() {
        assert_eq!(normalize_api_role_ids(vec![4, 0, 4, -1, 2]), vec![2, 4]);
    }

    #[test]
    fn merge_api_role_matrix_groups_permission_backed_roles() {
        let rows = vec![
            ApiRoleMatrixRow {
                path: "/api/users".to_string(),
                method: "GET".to_string(),
                authority_id: Some(3),
            },
            ApiRoleMatrixRow {
                path: "/api/roles".to_string(),
                method: "GET".to_string(),
                authority_id: None,
            },
        ];

        assert_eq!(
            merge_api_role_matrix(&rows),
            vec![
                ApiRoleMatrixItem {
                    path: "/api/roles".to_string(),
                    method: "GET".to_string(),
                    authority_ids: vec![],
                },
                ApiRoleMatrixItem {
                    path: "/api/users".to_string(),
                    method: "GET".to_string(),
                    authority_ids: vec![3],
                },
            ]
        );
    }
}
