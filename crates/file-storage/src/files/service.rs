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

pub struct FileUpload {
    pool: PgPool,
    storage_id: i64,
    storage: FileObjectStorage,
    original_name: String,
    ext: String,
    tag: String,
    category: String,
    stored_name: String,
    writer: Option<Writer>,
    size: usize,
}

impl Drop for FileUpload {
    fn drop(&mut self) {
        let Some(mut writer) = self.writer.take() else {
            return;
        };
        let operator = self.storage.operator.clone();
        let object = self.stored_name.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Err(error) = writer.abort().await {
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

impl FileUpload {
    // abort the upload and clean up the partial object
    pub async fn abort(mut self) -> Result<(), FileError> {
        self.cleanup().await
    }

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
    pub async fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FileError> {
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

    // finish the upload and store the file in the database
    pub async fn finish(mut self) -> Result<StoredFile, FileError> {
        if let Some(mut writer) = self.writer.take()
            && let Err(error) = writer.close().await
        {
            self.cleanup_after_failure("object finalization failed")
                .await;
            return Err(error.into());
        }

        let url = self.storage.public_url(&self.stored_name);
        let result = sqlx::query_as::<_, StoredFile>(
            r#"
            insert into uploaded_files (storage_id, name, url, ext, tag, category)
            values ($1, $2, $3, $4, $5, $6)
            returning
                id,
                storage_id,
                name,
                url,
                ext,
                tag,
                category,
                updated_at
            "#,
        )
        .bind(self.storage_id)
        .bind(&self.original_name)
        .bind(&url)
        .bind(&self.ext)
        .bind(&self.tag)
        .bind(&self.category)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(stored) => Ok(stored),
            Err(error) => {
                self.cleanup_after_failure("metadata persistence failed")
                    .await;
                Err(error.into())
            }
        }
    }
}

impl FileService {
    pub async fn managed(pool: PgPool) -> Result<(Self, StorageService), StorageError> {
        let storages = StorageService::load(pool.clone()).await?;
        Ok((
            Self {
                pool,
                storages: storages.clone(),
            },
            storages,
        ))
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
        sqlx::query_as::<_, UploadSession>(
            r#"
            select id, storage_id, name, object_name, ext, tag, category, total_size, uploaded_size
            from uploaded_file_sessions
            where id = $1
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
        let session = sqlx::query_as::<_, UploadSession>(
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
        .ok_or(FileError::UploadNotFound)?;
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
        let mut transaction = self.pool.begin().await?;
        let session = sqlx::query_as::<_, UploadSession>(
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
        .ok_or(FileError::UploadNotFound)?;
        if session.uploaded_size != session.total_size {
            return Err(FileError::UploadIncomplete);
        }
        let storage = self.storage_for(session.storage_id).await?;
        let writer = storage
            .storage
            .operator
            .writer(&session.object_name)
            .await?;
        let mut upload = FileUpload {
            pool: self.pool.clone(),
            storage_id: session.storage_id,
            storage: storage.storage.clone(),
            original_name: session.name,
            ext: session.ext,
            tag: session.tag,
            category: session.category,
            stored_name: session.object_name,
            writer: Some(writer),
            size: 0,
        };
        let mut offset = 0_i64;
        while offset < session.total_size {
            let bytes = storage
                .storage
                .operator
                .read(&upload_part(id, offset))
                .await?
                .to_vec();
            upload.write_chunk(&bytes).await?;
            offset += bytes.len() as i64;
        }
        let stored = upload.finish().await?;
        offset = 0;
        while offset < session.total_size {
            let part = upload_part(id, offset);
            let size = match storage.storage.operator.stat(&part).await {
                Ok(metadata) => metadata.content_length() as i64,
                Err(error) => {
                    tracing::warn!(%error, %part, "failed to inspect completed upload part");
                    break;
                }
            };
            if let Err(error) = storage.storage.operator.delete(&part).await {
                tracing::warn!(%error, %part, "failed to clean up completed upload part");
            }
            offset += size;
        }
        sqlx::query("delete from uploaded_file_sessions where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(stored)
    }
    pub async fn begin_upload(
        &self,
        name: &str,
        tag: &str,
        category: &str,
    ) -> Result<FileUpload, FileError> {
        let ext = safe_extension(name);
        let id = Uuid::new_v4();
        let stored_name = if ext.is_empty() {
            id.to_string()
        } else {
            format!("{id}.{ext}")
        };
        let active = self.default_storage().await?;
        let writer = active.storage.operator.writer(&stored_name).await?;

        Ok(FileUpload {
            pool: self.pool.clone(),
            storage_id: active.id,
            storage: active.storage,
            original_name: name.to_string(),
            ext,
            tag: tag.to_string(),
            category: category.to_string(),
            stored_name,
            writer: Some(writer),
            size: 0,
        })
    }
    pub async fn delete(&self, id: i64) -> Result<(), FileError> {
        let mut transaction = self.pool.begin().await?;
        let Some(file) = sqlx::query_as::<_, StoredFile>(
            r#"
            delete from uploaded_files
            where id = $1
            returning
                id,
                storage_id,
                name,
                url,
                ext,
                tag,
                category,
                updated_at
            "#,
        )
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        else {
            return Ok(());
        };
        let Some(storage_id) = file.storage_id else {
            transaction.commit().await?;
            return Ok(());
        };
        let storage = self.storage_for(storage_id).await?.storage;
        let Some(object) = storage.managed_object(&file.url) else {
            transaction.commit().await?;
            return Ok(());
        };
        let backup = match storage.operator.read(&object).await {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        storage.operator.delete(&object).await?;
        if let Err(error) = transaction.commit().await {
            if let Some(bytes) = backup {
                if let Err(restore_error) = storage.operator.write(&object, bytes).await {
                    tracing::error!(
                        %object,
                        %restore_error,
                        database_error = %error,
                        "failed to restore object after metadata commit failure"
                    );
                }
            }
            return Err(error.into());
        }
        Ok(())
    }

    pub async fn read_local_object(&self, object: &str) -> Result<Option<Vec<u8>>, FileError> {
        if object.is_empty() || object.contains('/') || object.contains("..") {
            return Ok(None);
        }
        let url = format!("/uploads/{object}");
        let storage_id = sqlx::query_scalar::<_, Option<i64>>(
            r#"
            select storage_id
            from uploaded_files
            where url = $1 and storage_id is not null
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
        where ($1::text is null or name ilike '%' || $1 || '%' or url ilike '%' || $1 || '%')
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
        where ($1::text is null or name ilike '%' || $1 || '%' or url ilike '%' || $1 || '%')
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
    sqlx::query("update uploaded_files set name = $1, updated_at = now() where id = $2")
        .bind(payload.name)
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
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
