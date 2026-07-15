use super::{CreateLoginLog, LoginLogError, LoginLogSearch, LoginLogView};

#[derive(Clone)]
pub struct LoginLogService {
    pool: sqlx::PgPool,
}

impl LoginLogService {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }
    pub async fn record(&self, log: CreateLoginLog) -> Result<(), LoginLogError> {
        Ok(create_login_log(&self.pool, log).await?)
    }
    pub async fn list(
        &self,
        query: LoginLogSearch,
    ) -> Result<(Vec<LoginLogView>, i64), LoginLogError> {
        Ok(get_login_log_list(&self.pool, query).await?)
    }
    pub async fn find(&self, id: i64) -> Result<Option<LoginLogView>, LoginLogError> {
        Ok(find_login_log(&self.pool, id).await?)
    }
}

pub(crate) async fn find_login_log(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<Option<LoginLogView>, sqlx::Error> {
    sqlx::query_as::<_, LoginLogView>(
        r#"
        select
            id,
            username,
            ip,
            status,
            error_message,
            agent,
            to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at
        from sys_login_logs
        where id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn create_login_log(
    pool: &sqlx::PgPool,
    log: CreateLoginLog,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_login_logs (username, ip, status, error_message, agent, user_id)
        values ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(log.username)
    .bind(log.ip)
    .bind(log.status)
    .bind(log.error_message)
    .bind(log.agent)
    .bind(log.user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn get_login_log_list(
    pool: &sqlx::PgPool,
    query: LoginLogSearch,
) -> Result<(Vec<LoginLogView>, i64), sqlx::Error> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let total: i64 = sqlx::query_scalar(
        r#"
        select count(*) from sys_login_logs
        where ($1::text is null or username ilike '%' || $1 || '%')
          and ($2::bool is null or status = $2)
        "#,
    )
    .bind(query.username.as_deref())
    .bind(query.status)
    .fetch_one(pool)
    .await?;

    let list = sqlx::query_as::<_, LoginLogView>(
        r#"
        select
            id,
            username,
            ip,
            status,
            error_message,
            agent,
            to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at
        from sys_login_logs
        where ($1::text is null or username ilike '%' || $1 || '%')
          and ($2::bool is null or status = $2)
        order by id desc
        limit $3 offset $4
        "#,
    )
    .bind(query.username.as_deref())
    .bind(query.status)
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((list, total))
}
