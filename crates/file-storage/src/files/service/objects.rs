use std::ops::Range;

use opendal::{ErrorKind, FuturesBytesStream, Reader};

use super::FileService;
use crate::files::FileError;

#[derive(sqlx::FromRow)]
struct FileDeletion {
    storage_id: Option<i64>,
    object_name: Option<String>,
    upload_id: Option<String>,
    upload_parts_pending: bool,
}

pub struct LocalFileReader {
    reader: Reader,
    pub size: u64,
}

impl LocalFileReader {
    pub async fn into_stream(
        self,
        range: Option<Range<u64>>,
    ) -> Result<FuturesBytesStream, FileError> {
        match range {
            Some(range) => Ok(self.reader.into_bytes_stream(range).await?),
            None => Ok(self.reader.into_bytes_stream(..).await?),
        }
    }
}

impl FileService {
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
            Err(error) => return Err(error.into()),
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

    pub(super) async fn recover_pending_work(&self) {
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

    pub async fn read_local_object(
        &self,
        object: &str,
    ) -> Result<Option<LocalFileReader>, FileError> {
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
        Ok(Some(LocalFileReader {
            reader,
            size: metadata.content_length(),
        }))
    }
}
