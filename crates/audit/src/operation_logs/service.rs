use super::model::OperationLogRow;
use super::{
    CreateOperationLog, OperationLogError, OperationLogSearch, OperationLogView, OperationUserView,
};

#[derive(Clone)]
pub struct OperationLogService {
    pool: sqlx::PgPool,
}

impl OperationLogService {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
    pub async fn record(&self, log: CreateOperationLog) -> Result<(), OperationLogError> {
        Ok(create(&self.pool, log).await?)
    }
    pub async fn list(
        &self,
        query: OperationLogSearch,
    ) -> Result<(Vec<OperationLogView>, i64), OperationLogError> {
        Ok(list(&self.pool, query).await?)
    }
    pub async fn find(&self, id: i64) -> Result<Option<OperationLogView>, OperationLogError> {
        Ok(find(&self.pool, id).await?.map(OperationLogView::from))
    }
}

impl From<OperationLogRow> for OperationLogView {
    fn from(row: OperationLogRow) -> Self {
        Self {
            id: row.id,
            ip: row.ip,
            method: row.method,
            path: row.path,
            status: row.status,
            agent: row.agent,
            error_message: row.error_message,
            body: row.body,
            resp: row.resp,
            created_at: row.created_at,
            user: OperationUserView {
                user_name: row.user_name,
                nick_name: row.nick_name,
            },
        }
    }
}

pub(crate) async fn find(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<Option<OperationLogRow>, sqlx::Error> {
    sqlx::query_as::<_, OperationLogRow>(
        r#"
        select r.id, r.ip, r.method, r.path, r.status, r.agent, r.error_message, r.body, r.resp,
               to_char(r.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at,
               coalesce(u.username, '') as user_name, coalesce(u.nick_name, '') as nick_name
        from sys_operation_records r
        left join sys_users u on u.id = r.user_id
        where r.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn create(
    pool: &sqlx::PgPool,
    log: CreateOperationLog,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_operation_records (
            ip, method, path, status, agent, error_message, body, resp, user_id
        ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(log.ip)
    .bind(log.method)
    .bind(log.path)
    .bind(log.status)
    .bind(log.agent)
    .bind(log.error_message)
    .bind(log.body)
    .bind(log.resp)
    .bind(log.user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn list(
    pool: &sqlx::PgPool,
    query: OperationLogSearch,
) -> Result<(Vec<OperationLogView>, i64), sqlx::Error> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let total: i64 = sqlx::query_scalar(
        r#"
        select count(*) from sys_operation_records
        where ($1::text is null or method ilike '%' || $1 || '%')
          and ($2::text is null or path ilike '%' || $2 || '%')
          and ($3::int is null or status = $3)
        "#,
    )
    .bind(query.method.as_deref())
    .bind(query.path.as_deref())
    .bind(query.status)
    .fetch_one(pool)
    .await?;

    let rows = sqlx::query_as::<_, OperationLogRow>(
        r#"
        select r.id, r.ip, r.method, r.path, r.status, r.agent, r.error_message, r.body, r.resp,
               to_char(r.created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at,
               coalesce(u.username, '') as user_name, coalesce(u.nick_name, '') as nick_name
        from sys_operation_records r
        left join sys_users u on u.id = r.user_id
        where ($1::text is null or r.method ilike '%' || $1 || '%')
          and ($2::text is null or r.path ilike '%' || $2 || '%')
          and ($3::int is null or r.status = $3)
        order by r.id desc
        limit $4 offset $5
        "#,
    )
    .bind(query.method.as_deref())
    .bind(query.path.as_deref())
    .bind(query.status)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((
        rows.into_iter().map(OperationLogView::from).collect(),
        total,
    ))
}
