use opendal::ErrorKind;
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

use super::{
    FileError, StartUpload, StoredFile, UploadSession,
    catalog::safe_extension,
    upload::{ClaimConflict, UploadObjectIoGuard, UploadOperationClaim},
};
use crate::storages::{StorageBackend, StorageService};

pub const MAX_UPLOAD_BYTES: usize = 1024 * 1024 * 1024;
pub const UPLOAD_CHUNK_BYTES: usize = 4 * 1024 * 1024;
pub(super) const UPLOAD_SESSION_TTL_SECONDS: i64 = 60 * 60;

#[derive(Clone)]
pub struct FileService {
    pub(super) pool: PgPool,
    pub(super) storages: StorageService,
}

#[derive(sqlx::FromRow)]
struct StaleUpload {
    id: String,
    storage_id: i64,
    object_name: String,
}

impl FileService {
    pub fn new(pool: PgPool, storages: StorageService) -> Self {
        Self { pool, storages }
    }

    pub async fn start_upload(&self, payload: StartUpload) -> Result<UploadSession, FileError> {
        if payload.size < 0 || payload.size > MAX_UPLOAD_BYTES as i64 {
            return Err(FileError::TooLarge);
        }
        let mut transaction = self.pool.begin().await?;
        let storage_id = self.storages.default_id_locked(&mut transaction).await?;
        let id = Uuid::new_v4().to_string();
        let ext = safe_extension(&payload.name);
        let object_name = if ext.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            format!("{}.{ext}", Uuid::new_v4())
        };
        let session = sqlx::query_as::<_, UploadSession>(
            r#"
            insert into uploaded_file_sessions (
                id, storage_id, name, object_name, ext, tag, category, total_size
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            "#,
        )
        .bind(id)
        .bind(storage_id)
        .bind(payload.name)
        .bind(object_name)
        .bind(ext)
        .bind(payload.tag)
        .bind(payload.category)
        .bind(payload.size)
        .fetch_one(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(session)
    }

    pub async fn upload_status(&self, id: &str) -> Result<UploadSession, FileError> {
        if let Some(session) = sqlx::query_as::<_, UploadSession>(
            r#"
            select id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            from uploaded_file_sessions
            where id = $1
              and (
                  (operation_state = 'uploading'
                      and updated_at >= now() - make_interval(secs => $2))
                  or (operation_state <> 'uploading'
                      and operation_started_at >= now() - make_interval(secs => $2))
              )
            "#,
        )
        .bind(id)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(session);
        }
        sqlx::query_as::<_, UploadSession>(
            r#"
            select
                upload_id as id,
                storage_id,
                name,
                object_name,
                ext,
                tag,
                category,
                size as total_size,
                size as uploaded_size
            from uploaded_files
            where upload_id = $1 and not deletion_pending
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(FileError::UploadNotFound)
    }

    pub async fn write_upload_chunk(
        &self,
        id: &str,
        offset: i64,
        bytes: &[u8],
    ) -> Result<UploadSession, FileError> {
        if bytes.is_empty() || bytes.len() > UPLOAD_CHUNK_BYTES {
            return Err(FileError::OffsetMismatch);
        }
        let token = Uuid::new_v4().to_string();
        let Some(session) = sqlx::query_as::<_, UploadSession>(
            r#"
            update uploaded_file_sessions
            set
                operation_state = 'writing',
                operation_token = $3,
                operation_started_at = now()
            where id = $1
              and uploaded_size = $2
              and operation_state = 'uploading'
              and updated_at >= now() - make_interval(secs => $5)
              and $2 + $4 <= total_size
            returning id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            "#,
        )
        .bind(id)
        .bind(offset)
        .bind(&token)
        .bind(bytes.len() as i64)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_optional(&self.pool)
        .await?
        else {
            if completed_upload(&self.pool, id).await?.is_some() {
                return Err(FileError::OffsetMismatch);
            }
            return match upload_operation_state(&self.pool, id).await? {
                Some(_) => Err(FileError::OffsetMismatch),
                None => Err(FileError::UploadNotFound),
            };
        };
        let storage = match self.storage_for(session.storage_id).await {
            Ok(storage) => storage,
            Err(error) => {
                self.release_upload_operation(id, &token).await;
                return Err(error);
            }
        };
        let mut claim =
            UploadOperationClaim::acquire(self, id, token, ClaimConflict::OffsetMismatch).await?;
        let part = upload_part(&session.id, offset, claim.token());
        if let Err(error) = storage.operator.write(&part, bytes.to_vec()).await {
            if let Err(cleanup_error) = storage.operator.delete(&part).await
                && cleanup_error.kind() != ErrorKind::NotFound
            {
                tracing::warn!(%part, %cleanup_error, "failed to clean up rejected upload part");
            }
            claim.abandon().await;
            return Err(error.into());
        }

        let token = claim.token().to_string();
        let result = self
            .persist_upload_part(
                claim.connection(),
                &session,
                &token,
                &part,
                bytes.len() as i64,
            )
            .await;
        if result.is_ok() {
            claim.release_object_io().await;
        } else {
            claim.abandon().await;
        }
        result
    }

    async fn persist_upload_part(
        &self,
        connection: &mut PgConnection,
        session: &UploadSession,
        token: &str,
        object_name: &str,
        size: i64,
    ) -> Result<UploadSession, FileError> {
        let mut transaction = connection.begin().await?;
        sqlx::query(
            r#"
            insert into uploaded_file_parts (upload_id, part_offset, size, object_name)
            values ($1, $2, $3, $4)
            "#,
        )
        .bind(&session.id)
        .bind(session.uploaded_size)
        .bind(size)
        .bind(object_name)
        .execute(&mut *transaction)
        .await?;
        let uploaded_size = session.uploaded_size + size;
        let Some(session) = sqlx::query_as::<_, UploadSession>(
            r#"
            update uploaded_file_sessions
            set
                uploaded_size = $1,
                operation_state = 'uploading',
                operation_token = null,
                operation_started_at = null,
                updated_at = now()
            where id = $2
              and operation_state = 'writing'
              and operation_token = $3
              and operation_started_at >= now() - make_interval(secs => $4)
            returning id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            "#,
        )
        .bind(uploaded_size)
        .bind(&session.id)
        .bind(token)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Err(FileError::UploadInProgress);
        };
        transaction.commit().await?;
        Ok(session)
    }

    pub(super) async fn refresh_upload_operation(
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

    pub(super) async fn release_upload_operation(&self, id: &str, token: &str) {
        let mut connection = match self.pool.acquire().await {
            Ok(connection) => connection,
            Err(error) => {
                tracing::error!(upload_id = id, %error, "failed to acquire a connection to release upload operation claim");
                return;
            }
        };
        Self::release_upload_operation_on(&mut connection, id, token).await;
    }

    async fn release_upload_operation_on(connection: &mut PgConnection, id: &str, token: &str) {
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
        .execute(connection)
        .await
        {
            tracing::error!(upload_id = id, %error, "failed to release upload operation claim");
        }
    }

    pub(super) async fn cleanup_upload_parts(
        &self,
        file_id: i64,
        storage_id: i64,
        id: &str,
    ) -> Result<(), FileError> {
        let storage = self.storage_for(storage_id).await?;
        let prefix = format!(".uploads/{id}/");
        storage
            .operator
            .delete_with(&prefix)
            .recursive(true)
            .await?;
        sqlx::query("update uploaded_files set upload_parts_pending = false where id = $1")
            .bind(file_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub(super) async fn reap_stale_uploads(&self) -> Result<(), FileError> {
        let uploads = sqlx::query_as::<_, StaleUpload>(
            r#"
            select id, storage_id, object_name
            from uploaded_file_sessions
            where updated_at < now() - make_interval(secs => $1)
              and (
                  operation_state = 'uploading'
                  or operation_started_at < now() - make_interval(secs => $1)
              )
            order by updated_at, id
            "#,
        )
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_all(&self.pool)
        .await?;
        for upload in uploads {
            if let Err(error) = self.reap_stale_upload(&upload).await {
                tracing::warn!(upload_id = %upload.id, %error, "failed to reap stale upload");
            }
        }
        Ok(())
    }

    async fn reap_stale_upload(&self, upload: &StaleUpload) -> Result<(), FileError> {
        let storage = self.storage_for(upload.storage_id).await?;
        let Some(mut object_io) = UploadObjectIoGuard::try_acquire(&self.pool, &upload.id).await?
        else {
            return Ok(());
        };
        let token = Uuid::new_v4().to_string();
        let claimed = sqlx::query(
            r#"
            update uploaded_file_sessions
            set
                operation_state = 'cleaning',
                operation_token = $2,
                operation_started_at = now()
            where id = $1
              and storage_id = $3
              and updated_at < now() - make_interval(secs => $4)
              and (
                  operation_state = 'uploading'
                  or operation_started_at < now() - make_interval(secs => $4)
              )
            "#,
        )
        .bind(&upload.id)
        .bind(&token)
        .bind(upload.storage_id)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .execute(object_io.connection())
        .await?;
        if claimed.rows_affected() == 0 {
            object_io.release().await;
            return Ok(());
        }

        match storage.operator.delete(&upload.object_name).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                object_io.release().await;
                self.release_upload_operation(&upload.id, &token).await;
                return Err(error.into());
            }
        }
        let prefix = format!(".uploads/{}/", upload.id);
        if let Err(error) = storage.operator.delete_with(&prefix).recursive(true).await {
            object_io.release().await;
            self.release_upload_operation(&upload.id, &token).await;
            return Err(error.into());
        }
        let deleted = sqlx::query(
            "delete from uploaded_file_sessions where id = $1 and operation_token = $2",
        )
        .bind(&upload.id)
        .bind(&token)
        .execute(object_io.connection())
        .await?;
        object_io.release().await;
        if deleted.rows_affected() == 1 {
            Ok(())
        } else {
            Err(FileError::UploadInProgress)
        }
    }

    pub(super) async fn storage_for(&self, id: i64) -> Result<StorageBackend, FileError> {
        Ok(self.storages.backend_for_id(id).await?)
    }
}

pub(super) async fn completed_upload(
    pool: &PgPool,
    id: &str,
) -> Result<Option<StoredFile>, sqlx::Error> {
    sqlx::query_as::<_, StoredFile>(
        r#"
        select id, storage_id, name, url, ext, tag, category, updated_at
        from uploaded_files
        where upload_id = $1 and not deletion_pending
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(super) async fn upload_operation_state(
    pool: &PgPool,
    id: &str,
) -> Result<Option<(i64, i64, String)>, FileError> {
    Ok(sqlx::query_as(
        r#"
        select uploaded_size, total_size, operation_state
        from uploaded_file_sessions
        where id = $1
          and (
              (operation_state = 'uploading'
                  and updated_at >= now() - make_interval(secs => $2))
              or (operation_state <> 'uploading'
                  and operation_started_at >= now() - make_interval(secs => $2))
          )
        "#,
    )
    .bind(id)
    .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
    .fetch_optional(pool)
    .await?)
}

fn upload_part(id: &str, offset: i64, token: &str) -> String {
    format!(".uploads/{id}/{offset:020}-{token}")
}
