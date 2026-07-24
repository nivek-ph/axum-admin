use sqlx::PgPool;

use super::{ParamListQuery, ParameterError, ParameterInput, SysParam};

#[derive(Clone)]
pub struct ParameterService {
    pool: PgPool,
}

impl ParameterService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        query: ParamListQuery,
    ) -> Result<(Vec<SysParam>, i64, i64, i64), ParameterError> {
        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(10).max(1);
        let offset = (page - 1) * page_size;
        let total: i64 = sqlx::query_scalar(
            r#"
            select count(*) from sys_params
            where ($1::text is null or name ilike '%' || $1 || '%')
              and ($2::text is null or "key" ilike '%' || $2 || '%')
            "#,
        )
        .bind(query.name.as_deref())
        .bind(query.key.as_deref())
        .fetch_one(&self.pool)
        .await?;

        let list = sqlx::query_as::<_, SysParam>(
            r#"
            select id, name, "key", value, "desc"
            from sys_params
            where ($1::text is null or name ilike '%' || $1 || '%')
              and ($2::text is null or "key" ilike '%' || $2 || '%')
            order by id desc
            limit $3 offset $4
            "#,
        )
        .bind(query.name.as_deref())
        .bind(query.key.as_deref())
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((list, total, page, page_size))
    }

    pub async fn create(&self, payload: ParameterInput) -> Result<(), ParameterError> {
        sqlx::query(
            "insert into sys_params (name, \"key\", value, \"desc\") values ($1, $2, $3, $4)",
        )
        .bind(payload.name)
        .bind(payload.key)
        .bind(payload.value)
        .bind(payload.desc)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update(&self, id: i64, payload: ParameterInput) -> Result<(), ParameterError> {
        sqlx::query(
            "update sys_params set name = $1, \"key\" = $2, value = $3, \"desc\" = $4 where id = $5",
        )
        .bind(payload.name)
        .bind(payload.key)
        .bind(payload.value)
        .bind(payload.desc)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn find(&self, id: i64) -> Result<Option<SysParam>, ParameterError> {
        Ok(sqlx::query_as::<_, SysParam>(
            "select id, name, \"key\", value, \"desc\" from sys_params where id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn delete(&self, id: i64) -> Result<(), ParameterError> {
        sqlx::query("delete from sys_params where id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_many(&self, ids: Vec<i64>) -> Result<(), ParameterError> {
        sqlx::query("delete from sys_params where id = any($1)")
            .bind(ids)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn by_key(&self, key: &str) -> Result<Option<SysParam>, ParameterError> {
        Ok(sqlx::query_as::<_, SysParam>(
            "select id, name, \"key\", value, \"desc\" from sys_params where \"key\" = $1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?)
    }
}
