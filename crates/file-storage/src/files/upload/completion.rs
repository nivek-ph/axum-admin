use opendal::{ErrorKind, Writer};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{
    files::{
        FileError, FileService, StoredFile, UploadSession,
        service::{MAX_UPLOAD_BYTES, UPLOAD_CHUNK_BYTES, completed_upload, upload_operation_state},
        upload::{
            ClaimConflict, UPLOAD_SESSION_TTL_SECONDS, UploadOperationClaim,
            refresh_upload_operation, release_upload_operation,
        },
    },
    storages::StorageBackend,
};

struct PendingObject {
    storage: StorageBackend,
    stored_name: String,
    writer: Option<Writer>,
    size: usize,
}

#[derive(sqlx::FromRow)]
struct UploadPart {
    part_offset: i64,
    size: i64,
    object_name: String,
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

impl PendingObject {
    async fn cleanup(&mut self) -> Result<(), FileError> {
        if let Some(mut writer) = self.writer.take()
            && let Err(error) = writer.abort().await
        {
            tracing::warn!(object = %self.stored_name, %error, "failed to abort upload writer");
        }
        self.storage.operator.delete(&self.stored_name).await?;
        Ok(())
    }

    async fn cleanup_after_failure(&mut self, operation: &'static str) {
        if let Err(error) = self.cleanup().await {
            tracing::error!(%error, operation, "failed to clean up upload");
        }
    }

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
                release_upload_operation(&self.pool, id, &token).await;
                return Err(error);
            }
        };
        let mut claim =
            UploadOperationClaim::acquire(&self.pool, id, token.clone(), ClaimConflict::InProgress)
                .await?;
        if previous_object_name != session.object_name
            && let Err(error) = storage.operator.delete(&previous_object_name).await
            && error.kind() != ErrorKind::NotFound
        {
            tracing::warn!(object = %previous_object_name, %error, "failed to clean up superseded completion object");
        }
        let mut upload = match self
            .assemble_upload(claim.connection(), &storage, &session, &token)
            .await
        {
            Ok(upload) => upload,
            Err(error) => {
                claim.abandon().await;
                return Err(error);
            }
        };
        claim.release_object_io().await;

        let mut transaction = match self.pool.begin().await {
            Ok(transaction) => transaction,
            Err(error) => {
                upload
                    .cleanup_after_failure("upload finalization transaction failed")
                    .await;
                claim.abandon().await;
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
                claim.abandon().await;
                return Err(error.into());
            }
        };
        if !owns_claim {
            let _ = transaction.rollback().await;
            upload
                .cleanup_after_failure("superseded upload completion")
                .await;
            claim.abandon().await;
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
                claim.abandon().await;
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
                claim.abandon().await;
                return Err(error.into());
            }
        };
        if deleted.rows_affected() != 1 {
            let _ = transaction.rollback().await;
            upload
                .cleanup_after_failure("upload claim was lost before commit")
                .await;
            claim.abandon().await;
            return Err(FileError::UploadInProgress);
        }
        if let Err(error) = transaction.commit().await {
            return Err(error.into());
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
        connection: &mut PgConnection,
        storage: &StorageBackend,
        session: &UploadSession,
        token: &str,
    ) -> Result<PendingObject, FileError> {
        let writer = storage.writer(&session.object_name).await?;
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
            .fetch_all(&mut *connection)
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
                refresh_upload_operation(connection, &session.id, token).await?;
            }
            if offset != session.total_size {
                return Err(FileError::UploadCorrupt);
            }
            upload.close_object().await?;
            refresh_upload_operation(connection, &session.id, token).await
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
}

fn new_object_name(ext: &str) -> String {
    if ext.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        format!("{}.{ext}", Uuid::new_v4())
    }
}
