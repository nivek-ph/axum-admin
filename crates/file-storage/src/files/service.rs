use std::path::Path;

use opendal::{ErrorKind, Writer};
use sqlx::PgPool;
use uuid::Uuid;

use super::{
    FileError, FileListQuery, ImportFileUrl, RenameFile, StartUpload, StoredFile, UploadSession,
    storage::FileObjectStorage,
};
use crate::storages::{StorageEntry, StorageError, StorageService};

pub const MAX_UPLOAD_BYTES: usize = 1024 * 1024 * 1024;
pub const UPLOAD_CHUNK_BYTES: usize = 8 * 1024 * 1024;

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
    cleanup_object: bool,
}

#[derive(sqlx::FromRow)]
struct FileDeletion {
    storage_id: Option<i64>,
    url: String,
    upload_id: Option<String>,
    upload_parts_pending: bool,
}

impl Drop for PendingObject {
    fn drop(&mut self) {
        if !self.cleanup_object {
            return;
        }
        let writer = self.writer.take();
        let operator = self.storage.operator.clone();
        let object = self.stored_name.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Some(mut writer) = writer
                    && let Err(error) = writer.abort().await
                {
                    tracing::warn!(%object, %error, "failed to abort abandoned upload writer");
                }
                if let Err(error) = operator.delete(&object).await {
                    tracing::warn!(%object, %error, "failed to clean up abandoned upload object");
                }
            });
        } else {
            tracing::error!(%object, "could not clean up abandoned upload outside a Tokio runtime");
        }
    }
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
        self.cleanup_object = false;
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
            "#,
        )
        .bind(id)
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
        let mut transaction = self.pool.begin().await?;
        // check if the upload session exists
        let Some(session) = sqlx::query_as::<_, UploadSession>(
            r#"
            select id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            from uploaded_file_sessions
            where id = $1
            for update
            "#,
        )
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            if completed_upload(&self.pool, id).await?.is_some() {
                return Err(FileError::OffsetMismatch);
            }
            return Err(FileError::UploadNotFound);
        };
        // check if the offset is correct
        if offset != session.uploaded_size {
            return Err(FileError::OffsetMismatch);
        }
        let remaining = session.total_size - session.uploaded_size;
        if bytes.is_empty() || bytes.len() > UPLOAD_CHUNK_BYTES || bytes.len() as i64 > remaining {
            return Err(FileError::OffsetMismatch);
        }
        let storage = self.storage_for(session.storage_id).await?;
        storage
            .storage
            .operator
            .write(&upload_part(&session.id, offset), bytes.to_vec())
            .await?;
        let uploaded_size = offset + bytes.len() as i64;
        let session = sqlx::query_as::<_, UploadSession>(
            r#"
            update uploaded_file_sessions
            set uploaded_size = $1
            where id = $2
            returning id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            "#,
        )
        .bind(uploaded_size)
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;
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
        let mut transaction = self.pool.begin().await?;
        let Some(session) = sqlx::query_as::<_, UploadSession>(
            r#"
            select id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            from uploaded_file_sessions
            where id = $1
            for update
            "#,
        )
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            transaction.rollback().await?;
            return completed_upload(&self.pool, id)
                .await?
                .ok_or(FileError::UploadNotFound);
        };
        if session.uploaded_size != session.total_size {
            return Err(FileError::UploadIncomplete);
        }
        let storage = self.storage_for(session.storage_id).await?;
        let writer = storage
            .storage
            .operator
            .writer(&session.object_name)
            .await?;
        let mut upload = PendingObject {
            storage: storage.storage.clone(),
            stored_name: session.object_name.clone(),
            writer: Some(writer),
            size: 0,
            cleanup_object: true,
        };
        let mut offset = 0_i64;
        while offset < session.total_size {
            let bytes = storage
                .storage
                .operator
                .read(&upload_part(id, offset))
                .await?
                .to_vec();
            let expected = (session.total_size - offset).min(UPLOAD_CHUNK_BYTES as i64) as usize;
            if bytes.len() != expected {
                return Err(FileError::UploadCorrupt);
            }
            upload.write_chunk(&bytes).await?;
            offset += bytes.len() as i64;
        }
        upload.close_object().await?;
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
                upload
                    .cleanup_after_failure("upload metadata persistence failed")
                    .await;
                return Err(error.into());
            }
        };
        sqlx::query("delete from uploaded_file_sessions where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        match transaction.commit().await {
            Ok(()) => upload.cleanup_object = false,
            Err(error) => {
                // PostgreSQL commit errors can be ambiguous. Keep the deterministic object so a
                // retry can either return the committed upload_id or safely overwrite it.
                upload.cleanup_object = false;
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

    // delete a file from the database and the object store
    pub async fn delete(&self, id: i64) -> Result<(), FileError> {
        let mut transaction = self.pool.begin().await?;
        let Some(file) = sqlx::query_as::<_, FileDeletion>(
            r#"
            select
                storage_id,
                url,
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
        let Some(object) = storage.managed_object(&file.url) else {
            self.finish_metadata_delete(id).await?;
            return Ok(());
        };
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

    async fn recover_pending_work(&self) {
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
    pub async fn read_local_object(&self, object: &str) -> Result<Option<Vec<u8>>, FileError> {
        if object.is_empty() || object.contains('/') || object.contains("..") {
            return Ok(None);
        }
        let url = format!("/uploads/{object}");
        let storage_id = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            select storage_id
            from uploaded_files
            where url = $1 and storage_id is not null and not deletion_pending
            order by id desc
            limit 1
            "#,
        )
        .bind(&url)
        .fetch_optional(&self.pool)
        .await?;
        let Some(Some(storage_id)) = storage_id else {
            return Ok(None);
        };
        let storage = self.storage_for(storage_id).await?;
        if !storage.storage.is_local() {
            return Ok(None);
        }
        match storage.storage.operator.read(object).await {
            Ok(bytes) => Ok(Some(bytes.to_vec())),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
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

fn upload_part(id: &str, offset: i64) -> String {
    format!(".uploads/{id}/{offset:020}")
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
