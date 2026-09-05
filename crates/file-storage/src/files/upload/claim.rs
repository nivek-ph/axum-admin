use sqlx::{PgConnection, PgPool, Postgres, pool::PoolConnection};

use crate::files::FileError;

pub(crate) const UPLOAD_SESSION_TTL_SECONDS: i64 = 60 * 60;

pub(crate) enum ClaimConflict {
    OffsetMismatch,
    InProgress,
}

impl ClaimConflict {
    fn error(&self) -> FileError {
        match self {
            Self::OffsetMismatch => FileError::OffsetMismatch,
            Self::InProgress => FileError::UploadInProgress,
        }
    }
}

pub(crate) struct UploadOperationClaim<'a> {
    pool: &'a PgPool,
    id: String,
    token: String,
    object_io: Option<UploadObjectIoGuard>,
}

impl<'a> UploadOperationClaim<'a> {
    pub(crate) async fn acquire(
        pool: &'a PgPool,
        id: &str,
        token: String,
        conflict: ClaimConflict,
    ) -> Result<Self, FileError> {
        let object_io = match UploadObjectIoGuard::try_acquire(pool, id).await {
            Ok(Some(object_io)) => object_io,
            Ok(None) => {
                release_upload_operation(pool, id, &token).await;
                return Err(conflict.error());
            }
            Err(error) => {
                release_upload_operation(pool, id, &token).await;
                return Err(error.into());
            }
        };
        let mut claim = Self {
            pool,
            id: id.to_string(),
            token,
            object_io: Some(object_io),
        };
        if let Err(error) = claim.heartbeat().await {
            claim.abandon().await;
            return Err(error);
        }
        Ok(claim)
    }

    pub(crate) fn token(&self) -> &str {
        &self.token
    }

    pub(crate) fn connection(&mut self) -> &mut PgConnection {
        self.object_io
            .as_mut()
            .expect("active upload claim should own its object I/O lock")
            .connection()
    }

    pub(crate) async fn heartbeat(&mut self) -> Result<(), FileError> {
        let id = self.id.clone();
        let token = self.token.clone();
        refresh_upload_operation(self.connection(), &id, &token).await
    }

    pub(crate) async fn release_object_io(&mut self) {
        if let Some(object_io) = self.object_io.take() {
            object_io.release().await;
        }
    }

    pub(crate) async fn abandon(mut self) {
        self.release_object_io().await;
        release_upload_operation(self.pool, &self.id, &self.token).await;
    }
}

pub(crate) async fn refresh_upload_operation(
    connection: &mut PgConnection,
    id: &str,
    token: &str,
) -> Result<(), FileError> {
    let updated = sqlx::query(
        r#"
        update uploaded_file_sessions
        set operation_started_at = now()
        where id = $1
          and operation_token = $2
          and operation_started_at >= now() - make_interval(secs => $3)
        "#,
    )
    .bind(id)
    .bind(token)
    .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
    .execute(connection)
    .await?;
    if updated.rows_affected() == 1 {
        Ok(())
    } else {
        Err(FileError::UploadInProgress)
    }
}

pub(crate) async fn release_upload_operation(pool: &PgPool, id: &str, token: &str) {
    let mut connection = match pool.acquire().await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::error!(upload_id = id, %error, "failed to acquire a connection to release upload operation claim");
            return;
        }
    };
    if let Err(error) = sqlx::query(
        r#"
        update uploaded_file_sessions
        set
            operation_state = 'uploading',
            operation_token = null,
            operation_started_at = null
        where id = $1 and operation_token = $2
        "#,
    )
    .bind(id)
    .bind(token)
    .execute(&mut *connection)
    .await
    {
        tracing::error!(upload_id = id, %error, "failed to release upload operation claim");
    }
}

pub(crate) struct UploadObjectIoGuard {
    connection: Option<PoolConnection<Postgres>>,
    upload_id: String,
}

impl UploadObjectIoGuard {
    pub(crate) async fn try_acquire(
        pool: &PgPool,
        upload_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let mut connection = pool.acquire().await?;
        let acquired = match sqlx::query_scalar::<_, bool>(
            "select pg_try_advisory_lock(hashtextextended($1, 0))",
        )
        .bind(upload_id)
        .fetch_one(&mut *connection)
        .await
        {
            Ok(acquired) => acquired,
            Err(error) => {
                connection.close_on_drop();
                return Err(error);
            }
        };
        Ok(acquired.then_some(Self {
            connection: Some(connection),
            upload_id: upload_id.to_string(),
        }))
    }

    pub(crate) fn connection(&mut self) -> &mut PgConnection {
        self.connection
            .as_deref_mut()
            .expect("upload object I/O guard should own its connection")
    }

    pub(crate) async fn release(mut self) {
        let Some(mut connection) = self.connection.take() else {
            return;
        };
        if let Err(error) = sqlx::query("select pg_advisory_unlock(hashtextextended($1, 0))")
            .bind(&self.upload_id)
            .execute(&mut *connection)
            .await
        {
            tracing::warn!(upload_id = %self.upload_id, %error, "failed to release upload object I/O lock");
            connection.close_on_drop();
        }
    }
}

impl Drop for UploadObjectIoGuard {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.as_mut() {
            connection.close_on_drop();
        }
    }
}
