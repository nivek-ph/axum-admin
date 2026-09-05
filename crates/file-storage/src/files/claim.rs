use sqlx::{PgConnection, PgPool, Postgres, pool::PoolConnection};

use crate::files::{FileError, FileService};

pub(super) enum ClaimConflict {
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

pub(super) struct UploadOperationClaim<'a> {
    service: &'a FileService,
    id: String,
    token: String,
    object_io: Option<UploadObjectIoGuard>,
}

impl<'a> UploadOperationClaim<'a> {
    pub(super) async fn acquire(
        service: &'a FileService,
        id: &str,
        token: String,
        conflict: ClaimConflict,
    ) -> Result<Self, FileError> {
        let object_io = match UploadObjectIoGuard::try_acquire(&service.pool, id).await {
            Ok(Some(object_io)) => object_io,
            Ok(None) => {
                service.release_upload_operation(id, &token).await;
                return Err(conflict.error());
            }
            Err(error) => {
                service.release_upload_operation(id, &token).await;
                return Err(error.into());
            }
        };
        let mut claim = Self {
            service,
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

    pub(super) fn token(&self) -> &str {
        &self.token
    }

    pub(super) fn connection(&mut self) -> &mut PgConnection {
        self.object_io
            .as_mut()
            .expect("active upload claim should own its object I/O lock")
            .connection()
    }

    pub(super) async fn heartbeat(&mut self) -> Result<(), FileError> {
        let id = self.id.clone();
        let token = self.token.clone();
        FileService::refresh_upload_operation(self.connection(), &id, &token).await
    }

    pub(super) async fn release_object_io(&mut self) {
        if let Some(object_io) = self.object_io.take() {
            object_io.release().await;
        }
    }

    pub(super) async fn abandon(mut self) {
        self.release_object_io().await;
        self.service
            .release_upload_operation(&self.id, &self.token)
            .await;
    }
}

pub(super) struct UploadObjectIoGuard {
    connection: Option<PoolConnection<Postgres>>,
    upload_id: String,
}

impl UploadObjectIoGuard {
    pub(super) async fn try_acquire(
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

    pub(super) fn connection(&mut self) -> &mut PgConnection {
        self.connection
            .as_deref_mut()
            .expect("upload object I/O guard should own its connection")
    }

    pub(super) async fn release(mut self) {
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
