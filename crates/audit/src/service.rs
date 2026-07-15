use sqlx::{PgConnection, PgPool};

use crate::{AuditError, AuditEvent, AuditEventView, AuditQuery};

#[derive(Clone)]
pub struct AuditService {
    pool: PgPool,
}

impl AuditService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record(&self, event: AuditEvent) -> Result<(), AuditError> {
        let mut connection = self.pool.acquire().await?;
        Self::record_in(&mut connection, event).await
    }

    pub async fn record_best_effort(&self, event: AuditEvent) {
        let action = event.action.as_str();
        if let Err(error) = self.record(event).await {
            tracing::error!(action, error = ?error, "audit event write failed");
        }
    }

    pub async fn record_in(
        connection: &mut PgConnection,
        event: AuditEvent,
    ) -> Result<(), AuditError> {
        let changes = serde_json::to_value(event.changes)?;
        sqlx::query(
            r#"
            insert into sys_audit_events (
                actor_id, actor_label, action, resource_type, resource_id, result,
                reason_code, source_ip, user_agent, changes
            ) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(event.actor.id)
        .bind(event.actor.label)
        .bind(event.action.as_str())
        .bind(event.resource.resource_type())
        .bind(event.resource.resource_id())
        .bind(event.result.as_str())
        .bind(event.reason_code.map(|reason| reason.as_str()))
        .bind(event.source.ip)
        .bind(event.source.user_agent)
        .bind(changes)
        .execute(connection)
        .await?;
        Ok(())
    }

    pub async fn list(
        &self,
        query: AuditQuery,
    ) -> Result<(Vec<AuditEventView>, i64, i64, i64), AuditError> {
        let page = query.page.max(1);
        let page_size = query.page_size.max(1);
        let offset = (page - 1) * page_size;
        let total = sqlx::query_scalar::<_, i64>(
            r#"
            select count(*) from sys_audit_events
            where ($1::text is null or actor_label ilike '%' || $1 || '%' or actor_id::text = $1)
              and ($2::text is null or action = $2)
              and ($3::text is null or resource_type = $3)
              and ($4::text is null or result = $4)
              and ($5::text is null or created_at >= $5::timestamptz)
              and ($6::text is null or created_at <= $6::timestamptz)
            "#,
        )
        .bind(query.actor.as_deref())
        .bind(query.action.as_deref())
        .bind(query.resource_type.as_deref())
        .bind(query.result.as_deref())
        .bind(query.started_at.as_deref())
        .bind(query.ended_at.as_deref())
        .fetch_one(&self.pool)
        .await?;

        let events = sqlx::query_as::<_, AuditEventView>(
            r#"
            select
                id, actor_id, actor_label, action, resource_type, resource_id, result,
                reason_code, source_ip, user_agent, changes,
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at
            from sys_audit_events
            where ($1::text is null or actor_label ilike '%' || $1 || '%' or actor_id::text = $1)
              and ($2::text is null or action = $2)
              and ($3::text is null or resource_type = $3)
              and ($4::text is null or result = $4)
              and ($5::text is null or created_at >= $5::timestamptz)
              and ($6::text is null or created_at <= $6::timestamptz)
            order by id desc
            limit $7 offset $8
            "#,
        )
        .bind(query.actor.as_deref())
        .bind(query.action.as_deref())
        .bind(query.resource_type.as_deref())
        .bind(query.result.as_deref())
        .bind(query.started_at.as_deref())
        .bind(query.ended_at.as_deref())
        .bind(page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok((events, total, page, page_size))
    }

    pub async fn find(&self, id: i64) -> Result<Option<AuditEventView>, AuditError> {
        Ok(sqlx::query_as::<_, AuditEventView>(
            r#"
            select
                id, actor_id, actor_label, action, resource_type, resource_id, result,
                reason_code, source_ip, user_agent, changes,
                to_char(created_at, 'YYYY-MM-DD"T"HH24:MI:SS') as created_at
            from sys_audit_events
            where id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }
}
