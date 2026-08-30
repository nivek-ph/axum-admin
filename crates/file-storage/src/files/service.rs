use std::path::Path;

use opendal::{ErrorKind, FuturesBytesStream, Writer};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    FileError, FileListQuery, ImportFileUrl, RenameFile, StartUpload, StoredFile, UploadSession,
    storage::FileObjectStorage,
};
use crate::storages::{StorageEntry, StorageError, StorageService};

pub const MAX_UPLOAD_BYTES: usize = 1024 * 1024 * 1024; // 1GB
pub const UPLOAD_CHUNK_BYTES: usize = 8 * 1024 * 1024; // 8MB
const UPLOAD_SESSION_TTL_SECONDS: i64 = 60 * 60; // 1h

#[derive(Clone)]
pub struct FileService {
    pool: PgPool,
    storages: StorageService,
}

struct PendingObject {
    storage: FileObjectStorage,
    stored_name: String,
    writer: Option<Writer>,
    size: usize,
}

#[derive(sqlx::FromRow)]
struct FileDeletion {
    storage_id: Option<i64>,
    object_name: Option<String>,
    upload_id: Option<String>,
    upload_parts_pending: bool,
}

#[derive(sqlx::FromRow)]
struct UploadPart {
    part_offset: i64,
    size: i64,
    object_name: String,
}

#[derive(sqlx::FromRow)]
struct StaleUpload {
    id: String,
    storage_id: i64,
}

#[derive(sqlx::FromRow)]
struct CompletionClaim {
    id: String,
    storage_id: i64,
    name: String,
    object_name: String,
    ext: String,
    tag: String,
    category: String,
    total_size: i64,
    uploaded_size: i64,
    previous_object_name: String,
}

impl CompletionClaim {
    fn into_parts(self) -> (UploadSession, String) {
        let previous_object_name = self.previous_object_name;
        (
            UploadSession {
                id: self.id,
                storage_id: self.storage_id,
                name: self.name,
                object_name: self.object_name,
                ext: self.ext,
                tag: self.tag,
                category: self.category,
                total_size: self.total_size,
                uploaded_size: self.uploaded_size,
            },
            previous_object_name,
        )
    }
}

pub struct LocalFileStream {
    pub stream: FuturesBytesStream,
    pub size: u64,
}

impl PendingObject {
    // clean up the partial object
    async fn cleanup(&mut self) -> Result<(), FileError> {
        if let Some(mut writer) = self.writer.take()
            && let Err(error) = writer.abort().await
        {
            tracing::warn!(object = %self.stored_name, %error, "failed to abort upload writer");
        }
        self.storage.operator.delete(&self.stored_name).await?;
        Ok(())
    }

    // clean up the partial object after a failure
    async fn cleanup_after_failure(&mut self, operation: &'static str) {
        if let Err(error) = self.cleanup().await {
            tracing::error!(%error, operation, "failed to clean up upload");
        }
    }

    // stream a chunk of data to the configured object store
    async fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FileError> {
        let size = self
            .size
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_UPLOAD_BYTES)
            .ok_or(FileError::TooLarge)?;
        if let Some(writer) = self.writer.as_mut() {
            writer.write(bytes.to_vec()).await?;
        }
        self.size = size;
        Ok(())
    }

    async fn close_object(&mut self) -> Result<(), FileError> {
        if let Some(mut writer) = self.writer.take() {
            writer.close().await?;
        }
        Ok(())
    }
}

impl FileService {
    pub async fn managed(pool: PgPool) -> Result<(Self, StorageService), StorageError> {
        let storages = StorageService::load(pool.clone()).await?;
        let service = Self {
            pool,
            storages: storages.clone(),
        };
        service.recover_pending_work().await;
        Ok((service, storages))
    }
    pub async fn list(
        &self,
        query: FileListQuery,
    ) -> Result<(Vec<StoredFile>, i64, i64, i64), FileError> {
        list(&self.pool, query).await
    }
    pub async fn edit_name(&self, payload: RenameFile) -> Result<(), FileError> {
        edit_name(&self.pool, payload).await
    }
    pub async fn import_url(&self, payload: ImportFileUrl) -> Result<(), FileError> {
        import_url(&self.pool, payload).await
    }
    pub async fn start_upload(&self, payload: StartUpload) -> Result<UploadSession, FileError> {
        if payload.size < 0 || payload.size > MAX_UPLOAD_BYTES as i64 {
            return Err(FileError::TooLarge);
        }
        let storage = self.default_storage().await?;
        let id = Uuid::new_v4().to_string();
        let ext = safe_extension(&payload.name);
        let object_name = if ext.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            format!("{}.{ext}", Uuid::new_v4())
        };
        Ok(sqlx::query_as::<_, UploadSession>(
            r#"
            insert into uploaded_file_sessions (
                id, storage_id, name, object_name, ext, tag, category, total_size
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            returning id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            "#,
        )
        .bind(id)
        .bind(storage.id)
        .bind(payload.name)
        .bind(object_name)
        .bind(ext)
        .bind(payload.tag)
        .bind(payload.category)
        .bind(payload.size)
        .fetch_one(&self.pool)
        .await?)
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

    // write a chunk of data to the upload session
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
        let part = upload_part(&session.id, offset, &token);
        if let Err(error) = storage.storage.operator.write(&part, bytes.to_vec()).await {
            self.release_upload_operation(id, &token).await;
            if let Err(cleanup_error) = storage.storage.operator.delete(&part).await
                && cleanup_error.kind() != ErrorKind::NotFound
            {
                tracing::warn!(%part, %cleanup_error, "failed to clean up rejected upload part");
            }
            return Err(error.into());
        }

        let result = self
            .persist_upload_part(&session, &token, &part, bytes.len() as i64)
            .await;
        if result.is_err() {
            self.release_upload_operation(id, &token).await;
        }
        result
    }

    async fn persist_upload_part(
        &self,
        session: &UploadSession,
        token: &str,
        object_name: &str,
        size: i64,
    ) -> Result<UploadSession, FileError> {
        let mut transaction = self.pool.begin().await?;
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

    pub async fn complete_upload(&self, id: &str) -> Result<StoredFile, FileError> {
        if let Some(stored) = completed_upload(&self.pool, id).await? {
            if let Some(storage_id) = stored.storage_id
                && let Err(error) = self.cleanup_upload_parts(stored.id, storage_id, id).await
            {
                tracing::warn!(file_id = stored.id, %error, "failed to clean up completed upload parts");
            }
            return Ok(stored);
        }
        let token = Uuid::new_v4().to_string();
        let Some(ext) = sqlx::query_scalar::<_, String>(
            r#"
                select ext
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
        else {
            return completed_upload(&self.pool, id)
                .await?
                .ok_or(FileError::UploadNotFound);
        };
        let object_name = new_object_name(&ext);
        let Some(claim) = sqlx::query_as::<_, CompletionClaim>(
            r#"
            with candidate as (
                select id, object_name as previous_object_name
                from uploaded_file_sessions
                where id = $1
                  and uploaded_size = total_size
                  and operation_state = 'uploading'
                  and updated_at >= now() - make_interval(secs => $4)
                for update
            ), updated as (
                update uploaded_file_sessions as session
                set
                    object_name = $2,
                    operation_state = 'completing',
                    operation_token = $3,
                    operation_started_at = now()
                from candidate
                where session.id = candidate.id
                returning
                    session.id,
                    session.storage_id,
                    session.name,
                    session.object_name,
                    session.ext,
                    session.tag,
                    session.category,
                    session.total_size,
                    session.uploaded_size,
                    candidate.previous_object_name
            )
            select * from updated
            "#,
        )
        .bind(id)
        .bind(&object_name)
        .bind(&token)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_optional(&self.pool)
        .await?
        else {
            if let Some(stored) = completed_upload(&self.pool, id).await? {
                return Ok(stored);
            }
            let Some((uploaded_size, total_size, state)) =
                upload_operation_state(&self.pool, id).await?
            else {
                return Err(FileError::UploadNotFound);
            };
            return if uploaded_size != total_size {
                Err(FileError::UploadIncomplete)
            } else if state == "completing" || state == "writing" {
                Err(FileError::UploadInProgress)
            } else {
                Err(FileError::UploadCorrupt)
            };
        };
        let (session, previous_object_name) = claim.into_parts();
        let storage = match self.storage_for(session.storage_id).await {
            Ok(storage) => storage,
            Err(error) => {
                self.release_upload_operation(id, &token).await;
                return Err(error);
            }
        };
        if previous_object_name != session.object_name
            && let Err(error) = storage.storage.operator.delete(&previous_object_name).await
            && error.kind() != ErrorKind::NotFound
        {
            tracing::warn!(object = %previous_object_name, %error, "failed to clean up superseded completion object");
        }
        let mut upload = match self
            .assemble_upload(&storage.storage, &session, &token)
            .await
        {
            Ok(upload) => upload,
            Err(error) => {
                self.release_upload_operation(id, &token).await;
                return Err(error);
            }
        };

        let mut transaction = match self.pool.begin().await {
            Ok(transaction) => transaction,
            Err(error) => {
                upload
                    .cleanup_after_failure("upload finalization transaction failed")
                    .await;
                self.release_upload_operation(id, &token).await;
                return Err(error.into());
            }
        };
        let owns_claim = match sqlx::query_scalar::<_, bool>(
            r#"
            select true
            from uploaded_file_sessions
            where id = $1
              and operation_state = 'completing'
              and operation_token = $2
              and operation_started_at >= now() - make_interval(secs => $3)
            for update
            "#,
        )
        .bind(id)
        .bind(&token)
        .bind(UPLOAD_SESSION_TTL_SECONDS as f64)
        .fetch_optional(&mut *transaction)
        .await
        {
            Ok(owns_claim) => owns_claim.is_some(),
            Err(error) => {
                let _ = transaction.rollback().await;
                upload
                    .cleanup_after_failure("upload claim verification failed")
                    .await;
                self.release_upload_operation(id, &token).await;
                return Err(error.into());
            }
        };
        if !owns_claim {
            let _ = transaction.rollback().await;
            upload
                .cleanup_after_failure("superseded upload completion")
                .await;
            return completed_upload(&self.pool, id)
                .await?
                .ok_or(FileError::UploadInProgress);
        }
        let url = upload.storage.public_url(&upload.stored_name);
        let stored = match sqlx::query_as::<_, StoredFile>(
            r#"
            insert into uploaded_files (
                upload_id, storage_id, object_name, size, upload_parts_pending,
                name, url, ext, tag, category
            )
            values ($1, $2, $3, $4, true, $5, $6, $7, $8, $9)
            returning id, storage_id, name, url, ext, tag, category, updated_at
            "#,
        )
        .bind(id)
        .bind(session.storage_id)
        .bind(&upload.stored_name)
        .bind(session.total_size)
        .bind(&session.name)
        .bind(url)
        .bind(&session.ext)
        .bind(&session.tag)
        .bind(&session.category)
        .fetch_one(&mut *transaction)
        .await
        {
            Ok(stored) => stored,
            Err(error) => {
                let _ = transaction.rollback().await;
                upload
                    .cleanup_after_failure("upload metadata persistence failed")
                    .await;
                self.release_upload_operation(id, &token).await;
                return Err(error.into());
            }
        };
        let deleted = match sqlx::query(
            "delete from uploaded_file_sessions where id = $1 and operation_token = $2",
        )
        .bind(id)
        .bind(&token)
        .execute(&mut *transaction)
        .await
        {
            Ok(deleted) => deleted,
            Err(error) => {
                let _ = transaction.rollback().await;
                upload
                    .cleanup_after_failure("upload session finalization failed")
                    .await;
                self.release_upload_operation(id, &token).await;
                return Err(error.into());
            }
        };
        if deleted.rows_affected() != 1 {
            let _ = transaction.rollback().await;
            upload
                .cleanup_after_failure("upload claim was lost before commit")
                .await;
            return Err(FileError::UploadInProgress);
        }
        match transaction.commit().await {
            Ok(()) => {}
            Err(error) => {
                // PostgreSQL commit errors can be ambiguous. Keep the deterministic object so a
                // retry can either return the committed upload_id or safely supersede it.
                return Err(error.into());
            }
        }
        if let Err(error) = self
            .cleanup_upload_parts(stored.id, session.storage_id, id)
            .await
        {
            tracing::warn!(file_id = stored.id, %error, "failed to clean up completed upload parts");
        }
        Ok(stored)
    }

    async fn assemble_upload(
        &self,
        storage: &FileObjectStorage,
        session: &UploadSession,
        token: &str,
    ) -> Result<PendingObject, FileError> {
        let writer = storage.operator.writer(&session.object_name).await?;
        let mut upload = PendingObject {
            storage: storage.clone(),
            stored_name: session.object_name.clone(),
            writer: Some(writer),
            size: 0,
        };
        let result = async {
            let parts = sqlx::query_as::<_, UploadPart>(
                r#"
                select part_offset, size, object_name
                from uploaded_file_parts
                where upload_id = $1
                order by part_offset
                "#,
            )
            .bind(&session.id)
            .fetch_all(&self.pool)
            .await?;
            let mut offset = 0_i64;
            for part in parts {
                if part.part_offset != offset
                    || part.size <= 0
                    || part.size > UPLOAD_CHUNK_BYTES as i64
                    || part.size > session.total_size - offset
                {
                    return Err(FileError::UploadCorrupt);
                }
                let bytes = storage.operator.read(&part.object_name).await?.to_vec();
                if bytes.len() as i64 != part.size {
                    return Err(FileError::UploadCorrupt);
                }
                upload.write_chunk(&bytes).await?;
                offset += part.size;
                self.refresh_upload_operation(&session.id, token).await?;
            }
            if offset != session.total_size {
                return Err(FileError::UploadCorrupt);
            }
            upload.close_object().await
        }
        .await;
        match result {
            Ok(()) => Ok(upload),
            Err(error) => {
                upload
                    .cleanup_after_failure("upload object assembly failed")
                    .await;
                Err(error)
            }
        }
    }

    async fn refresh_upload_operation(&self, id: &str, token: &str) -> Result<(), FileError> {
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
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 1 {
            Ok(())
        } else {
            Err(FileError::UploadInProgress)
        }
    }

    async fn release_upload_operation(&self, id: &str, token: &str) {
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
        .execute(&self.pool)
        .await
        {
            tracing::error!(upload_id = id, %error, "failed to release upload operation claim");
        }
    }

    // delete a file from the database and the object store
    pub async fn delete(&self, id: i64) -> Result<(), FileError> {
        let mut transaction = self.pool.begin().await?;
        let Some(file) = sqlx::query_as::<_, FileDeletion>(
            r#"
            select
                storage_id,
                object_name,
                upload_id,
                upload_parts_pending
            from uploaded_files
            where id = $1
            for update
            "#,
        )
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Ok(());
        };
        let Some(storage_id) = file.storage_id else {
            sqlx::query("delete from uploaded_files where id = $1")
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            transaction.commit().await?;
            return Ok(());
        };
        sqlx::query(
            "update uploaded_files set deletion_pending = true, updated_at = now() where id = $1",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        if file.upload_parts_pending
            && let Some(upload_id) = file.upload_id.as_deref()
        {
            self.cleanup_upload_parts(id, storage_id, upload_id).await?;
        }

        let storage = self.storage_for(storage_id).await?.storage;
        let object = file.object_name.ok_or(FileError::UploadCorrupt)?;
        match storage.operator.delete(&object).await {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                self.restore_failed_delete(id).await;
                return Err(error.into());
            }
        }
        self.finish_metadata_delete(id).await?;
        Ok(())
    }

    async fn finish_metadata_delete(&self, id: i64) -> Result<(), FileError> {
        sqlx::query("delete from uploaded_files where id = $1 and deletion_pending")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn restore_failed_delete(&self, id: i64) {
        if let Err(error) = sqlx::query(
            "update uploaded_files set deletion_pending = false, updated_at = now() where id = $1",
        )
        .bind(id)
        .execute(&self.pool)
        .await
        {
            tracing::error!(id, %error, "failed to restore file metadata after object deletion failure");
        }
    }

    async fn cleanup_upload_parts(
        &self,
        file_id: i64,
        storage_id: i64,
        id: &str,
    ) -> Result<(), FileError> {
        let storage = self.storage_for(storage_id).await?;
        let prefix = format!(".uploads/{id}/");
        storage
            .storage
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

    async fn reap_stale_uploads(&self) -> Result<(), FileError> {
        let uploads = sqlx::query_as::<_, StaleUpload>(
            r#"
            select id, storage_id
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
        .execute(&self.pool)
        .await?;
        if claimed.rows_affected() == 0 {
            return Ok(());
        }

        let storage = match self.storage_for(upload.storage_id).await {
            Ok(storage) => storage,
            Err(error) => {
                self.release_upload_operation(&upload.id, &token).await;
                return Err(error);
            }
        };
        let prefix = format!(".uploads/{}/", upload.id);
        if let Err(error) = storage
            .storage
            .operator
            .delete_with(&prefix)
            .recursive(true)
            .await
        {
            self.release_upload_operation(&upload.id, &token).await;
            return Err(error.into());
        }
        let deleted = sqlx::query(
            "delete from uploaded_file_sessions where id = $1 and operation_token = $2",
        )
        .bind(&upload.id)
        .bind(&token)
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 1 {
            Ok(())
        } else {
            Err(FileError::UploadInProgress)
        }
    }

    async fn recover_pending_work(&self) {
        if let Err(error) = self.reap_stale_uploads().await {
            tracing::error!(%error, "failed to load stale uploads");
        }
        let pending_deletions = match sqlx::query_scalar::<_, i64>(
            "select id from uploaded_files where deletion_pending order by id",
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(ids) => ids,
            Err(error) => {
                tracing::error!(%error, "failed to load pending file deletions");
                return;
            }
        };
        for id in pending_deletions {
            if let Err(error) = self.delete(id).await {
                tracing::error!(id, %error, "failed to resume pending file deletion");
            }
        }

        let pending_parts = match sqlx::query_as::<_, (i64, i64, String)>(
            r#"
            select id, storage_id, upload_id
            from uploaded_files
            where upload_parts_pending and storage_id is not null and upload_id is not null
            order by id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        {
            Ok(parts) => parts,
            Err(error) => {
                tracing::error!(%error, "failed to load pending upload part cleanup");
                return;
            }
        };
        for (file_id, storage_id, upload_id) in pending_parts {
            if let Err(error) = self
                .cleanup_upload_parts(file_id, storage_id, &upload_id)
                .await
            {
                tracing::warn!(file_id, %error, "failed to resume completed upload part cleanup");
            }
        }
    }

    // read a local object from the object store
    pub async fn read_local_object(
        &self,
        object: &str,
    ) -> Result<Option<LocalFileStream>, FileError> {
        if object.is_empty() || object.contains('/') || object.contains("..") {
            return Ok(None);
        }
        let storage_id = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            select storage_id
            from uploaded_files
            where object_name = $1 and storage_id is not null and not deletion_pending
            order by id desc
            limit 1
            "#,
        )
        .bind(object)
        .fetch_optional(&self.pool)
        .await?;
        let Some(Some(storage_id)) = storage_id else {
            return Ok(None);
        };
        let storage = self.storage_for(storage_id).await?;
        if !storage.storage.is_local() {
            return Ok(None);
        }
        let metadata = match storage.storage.operator.stat(object).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let reader = storage.storage.operator.reader(object).await?;
        let stream = reader.into_bytes_stream(..).await?;
        Ok(Some(LocalFileStream {
            stream,
            size: metadata.content_length(),
        }))
    }

    async fn default_storage(&self) -> Result<StorageEntry, FileError> {
        Ok(self.storages.default_entry().await?)
    }

    async fn storage_for(&self, id: i64) -> Result<StorageEntry, FileError> {
        Ok(self.storages.entry_for_id(id).await?)
    }
}

async fn list(
    pool: &sqlx::PgPool,
    query: FileListQuery,
) -> Result<(Vec<StoredFile>, i64, i64, i64), FileError> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let total: i64 = sqlx::query_scalar(
        r#"
        select count(*) from uploaded_files
        where not deletion_pending
          and ($1::text is null or name ilike '%' || $1 || '%' or url ilike '%' || $1 || '%')
          and ($2::text is null or category = $2)
        "#,
    )
    .bind(query.keyword.as_deref())
    .bind(query.category.as_deref())
    .fetch_one(pool)
    .await?;
    let list = sqlx::query_as::<_, StoredFile>(
        r#"
        select
            id,
            storage_id,
            name,
            url,
            ext,
            tag,
            category,
            updated_at
        from uploaded_files
        where not deletion_pending
          and ($1::text is null or name ilike '%' || $1 || '%' or url ilike '%' || $1 || '%')
          and ($2::text is null or category = $2)
        order by id desc
        limit $3 offset $4
        "#,
    )
    .bind(query.keyword.as_deref())
    .bind(query.category.as_deref())
    .bind(page_size)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    Ok((list, total, page, page_size))
}

async fn edit_name(pool: &sqlx::PgPool, payload: RenameFile) -> Result<(), FileError> {
    sqlx::query(
        "update uploaded_files set name = $1, updated_at = now() where id = $2 and not deletion_pending",
    )
        .bind(payload.name)
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn completed_upload(pool: &PgPool, id: &str) -> Result<Option<StoredFile>, sqlx::Error> {
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

async fn import_url(pool: &sqlx::PgPool, payload: ImportFileUrl) -> Result<(), FileError> {
    let ext = normalized_extension(&payload.url);
    sqlx::query(
        "insert into uploaded_files (name, url, ext, tag, category) values ($1, $2, $3, $4, $5)",
    )
    .bind(payload.name)
    .bind(payload.url)
    .bind(ext)
    .bind(payload.tag)
    .bind(payload.category)
    .execute(pool)
    .await?;
    Ok(())
}

fn normalized_extension(value: &str) -> String {
    value
        .split(['?', '#'])
        .next()
        .and_then(|path| Path::new(path).extension())
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn safe_extension(value: &str) -> String {
    let ext = normalized_extension(value);
    if ext.len() <= 16
        && ext
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    {
        ext
    } else {
        String::new()
    }
}

async fn upload_operation_state(
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

fn new_object_name(ext: &str) -> String {
    if ext.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        format!("{}.{ext}", Uuid::new_v4())
    }
}

fn upload_part(id: &str, offset: i64, token: &str) -> String {
    format!(".uploads/{id}/{offset:020}-{token}")
}

#[cfg(test)]
mod tests {
    use super::normalized_extension;

    #[test]
    fn extension_is_normalized_without_query_or_fragment() {
        assert_eq!(normalized_extension("photo.PNG"), "png");
        assert_eq!(
            normalized_extension("https://example.test/report.PDF?download=1"),
            "pdf"
        );
        assert_eq!(normalized_extension("README"), "");
    }
}
