use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use admin_httpz::AppError;

use crate::{authority::SUPER_ADMIN_AUTHORITY_ID, errors};

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

#[derive(Debug, Clone, FromRow)]
struct RegisteredApiPath {
    pub id: i64,
    pub path: String,
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
    #[serde(rename = "authorityIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CasbinInfo {
    pub path: String,
    pub method: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCasbinRequest {
    #[serde(rename = "authorityId")]
    pub authority_id: i64,
    #[serde(rename = "casbinInfos")]
    pub casbin_infos: Vec<CasbinInfo>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct PolicyPath {
    pub path: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiRoleSelection {
    #[serde(rename = "authorityIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiRoleMatrixItem {
    pub path: String,
    pub method: String,
    #[serde(rename = "authorityIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiAccessDecision {
    Allowed,
    Denied,
    Unregistered,
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
    sqlx::query("delete from sys_role_apis where api_id = $1")
        .bind(api_id)
        .execute(pool)
        .await?;
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
    sqlx::query("delete from sys_role_apis where api_id = any($1)")
        .bind(&payload.ids)
        .execute(pool)
        .await?;
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
        select ra.authority_id
        from sys_role_apis ra
        inner join sys_apis a on a.id = ra.api_id
        where a.path = $1 and a.method = $2
        order by ra.authority_id
        "#,
    )
    .bind(query.path)
    .bind(query.method)
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
        select a.id, a.path, a.description, a.api_group, a.method
        from sys_role_apis ra
        inner join sys_apis a on a.id = ra.api_id
        where ra.authority_id = $1
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
        select a.path, a.method, ra.authority_id
        from sys_apis a
        left join sys_role_apis ra on a.id = ra.api_id
        order by a.api_group, a.path, a.method, ra.authority_id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut grouped = BTreeMap::<(String, String), Vec<i64>>::new();
    for row in rows {
        let authority_ids = grouped.entry((row.path, row.method)).or_default();
        if let Some(authority_id) = row.authority_id {
            authority_ids.push(authority_id);
        }
    }

    Ok(grouped
        .into_iter()
        .map(|((path, method), authority_ids)| ApiRoleMatrixItem {
            path,
            method,
            authority_ids,
        })
        .collect())
}

pub async fn set_api_roles(
    pool: &sqlx::PgPool,
    payload: SetApiRolesRequest,
) -> Result<(), ApiError> {
    let api_id: i64 = sqlx::query_scalar("select id from sys_apis where path = $1 and method = $2")
        .bind(&payload.path)
        .bind(&payload.method)
        .fetch_optional(pool)
        .await?
        .ok_or(ApiError::NotFound)?;

    sqlx::query("delete from sys_role_apis where api_id = $1")
        .bind(api_id)
        .execute(pool)
        .await?;

    for authority_id in payload.authority_ids {
        sqlx::query(
            r#"
            insert into sys_role_apis (authority_id, api_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(authority_id)
        .bind(api_id)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn update_casbin(
    pool: &sqlx::PgPool,
    payload: UpdateCasbinRequest,
) -> Result<(), ApiError> {
    sqlx::query("delete from sys_role_apis where authority_id = $1")
        .bind(payload.authority_id)
        .execute(pool)
        .await?;

    for item in payload.casbin_infos {
        if let Some(api_id) =
            sqlx::query_scalar::<_, i64>("select id from sys_apis where path = $1 and method = $2")
                .bind(item.path)
                .bind(item.method)
                .fetch_optional(pool)
                .await?
        {
            sqlx::query(
                r#"
                insert into sys_role_apis (authority_id, api_id)
                values ($1, $2)
                on conflict do nothing
                "#,
            )
            .bind(payload.authority_id)
            .bind(api_id)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

pub async fn get_policy_path_by_authority_id(
    pool: &sqlx::PgPool,
    authority_id: i64,
) -> Result<Vec<PolicyPath>, ApiError> {
    Ok(sqlx::query_as::<_, PolicyPath>(
        r#"
        select a.path, a.method
        from sys_role_apis ra
        inner join sys_apis a on a.id = ra.api_id
        where ra.authority_id = $1
        order by a.api_group, a.path, a.method
        "#,
    )
    .bind(authority_id)
    .fetch_all(pool)
    .await?)
}

pub fn route_pattern_matches(pattern: &str, path: &str) -> bool {
    let pattern_segments = pattern.trim_matches('/').split('/').collect::<Vec<_>>();
    let path_segments = path.trim_matches('/').split('/').collect::<Vec<_>>();

    if pattern_segments.len() != path_segments.len() {
        return false;
    }

    pattern_segments
        .iter()
        .zip(path_segments.iter())
        .all(|(pattern_segment, path_segment)| {
            is_dynamic_segment(pattern_segment) || pattern_segment == path_segment
        })
}

fn is_dynamic_segment(segment: &str) -> bool {
    (segment.starts_with('{') && segment.ends_with('}')) || segment.starts_with(':')
}

pub async fn check_api_access(
    pool: &sqlx::PgPool,
    authority_id: i64,
    path: &str,
    method: &str,
) -> Result<ApiAccessDecision, ApiError> {
    if authority_id == SUPER_ADMIN_AUTHORITY_ID {
        return Ok(ApiAccessDecision::Allowed);
    }

    let method = method.to_ascii_uppercase();
    let candidates = sqlx::query_as::<_, RegisteredApiPath>(
        r#"
        select id, path
        from sys_apis
        where method = $1
        order by path
        "#,
    )
    .bind(method)
    .fetch_all(pool)
    .await?;

    let exact_ids = candidates
        .iter()
        .filter(|api| api.path == path)
        .map(|api| api.id)
        .collect::<Vec<_>>();
    let matched_api_ids = if exact_ids.is_empty() {
        candidates
            .iter()
            .filter(|api| route_pattern_matches(&api.path, path))
            .map(|api| api.id)
            .collect::<Vec<_>>()
    } else {
        exact_ids
    };

    if matched_api_ids.is_empty() {
        return Ok(ApiAccessDecision::Unregistered);
    }

    let allowed: Option<i64> = sqlx::query_scalar(
        r#"
        select api_id
        from sys_role_apis
        where authority_id = $1 and api_id = any($2)
        limit 1
        "#,
    )
    .bind(authority_id)
    .bind(&matched_api_ids)
    .fetch_optional(pool)
    .await?;

    Ok(if allowed.is_some() {
        ApiAccessDecision::Allowed
    } else {
        ApiAccessDecision::Denied
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_pattern_matches_dynamic_segments() {
        assert!(route_pattern_matches(
            "/api/menus/{id}/roles",
            "/api/menus/1/roles",
        ));
        assert!(route_pattern_matches("/api/routes/{id}", "/api/routes/42",));
        assert!(!route_pattern_matches(
            "/api/menus/{id}/roles",
            "/api/menus/1",
        ));
        assert!(!route_pattern_matches(
            "/api/menus/{id}/roles",
            "/api/routes/1/roles",
        ));
    }
}
