use std::path::{Path, PathBuf};

use opendal::{ErrorKind, Operator, Writer, services};
use sqlx::PgPool;
use uuid::Uuid;

use super::{FileError, FileListQuery, ImportFileUrl, RenameFile, StoredFile};

pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct S3StorageConfig {
    pub bucket: String,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub root: String,
    pub public_base_url: String,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub enable_virtual_host_style: bool,
}

#[derive(Clone)]
pub struct FileService {
    pool: PgPool,
    storage: Operator,
    public_url_prefix: String,
    local_root: Option<PathBuf>,
}

pub struct FileUpload {
    pool: PgPool,
    storage: Operator,
    original_name: String,
    ext: String,
    tag: String,
    category: String,
    temp_path: String,
    final_path: String,
    public_url: String,
    local_temp_path: Option<PathBuf>,
    writer: Option<Writer>,
    cleanup_pending: bool,
    size: usize,
}

impl Drop for FileUpload {
    fn drop(&mut self) {
        if !self.cleanup_pending {
            return;
        }
        let writer = self.writer.take();
        let storage = self.storage.clone();
        let path = self.temp_path.clone();
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            runtime.spawn(async move {
                if let Some(mut writer) = writer
                    && let Err(error) = writer.abort().await
                    && error.kind() != ErrorKind::Unsupported
                {
                    tracing::warn!(%error, "failed to abort abandoned upload");
                }
                if let Err(error) = storage.delete(&path).await {
                    tracing::warn!(%path, %error, "failed to clean up abandoned upload");
                }
            });
        } else if let Some(path) = self.local_temp_path.take() {
            drop(writer);
            if let Err(error) = std::fs::remove_file(&path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(?path, %error, "failed to clean up abandoned local upload");
            }
        } else {
            tracing::warn!(%path, "could not clean up abandoned upload without a Tokio runtime");
        }
    }
}

impl FileUpload {
    // abort the upload and clean up the temporary file
    pub async fn abort(mut self) -> Result<(), FileError> {
        self.cleanup().await
    }

    // clean up the temporary file
    async fn cleanup(&mut self) -> Result<(), FileError> {
        if let Some(mut writer) = self.writer.take()
            && let Err(error) = writer.abort().await
            && error.kind() != ErrorKind::Unsupported
        {
            return Err(error.into());
        }
        self.storage.delete(&self.temp_path).await?;
        self.cleanup_pending = false;
        Ok(())
    }

    // clean up the temporary file after a failure
    async fn cleanup_after_failure(&mut self, operation: &'static str) {
        if let Err(error) = self.cleanup().await {
            tracing::error!(%error, operation, "failed to clean up upload");
        }
    }

    // write a chunk of data to the temporary file
    pub async fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FileError> {
        let size = self
            .size
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_UPLOAD_BYTES)
            .ok_or(FileError::TooLarge)?;
        let writer = self
            .writer
            .as_mut()
            .expect("an active upload should retain its writer");
        writer.write(bytes.to_vec()).await?;
        self.size = size;
        Ok(())
    }

    // finish the upload and store the file in the database
    pub async fn finish(mut self) -> Result<StoredFile, FileError> {
        let mut writer = self
            .writer
            .take()
            .expect("an active upload should retain its writer");
        if let Err(error) = writer.close().await {
            if let Err(abort_error) = writer.abort().await
                && abort_error.kind() != ErrorKind::Unsupported
            {
                tracing::error!(%abort_error, "failed to abort upload after close failure");
            }
            self.cleanup_after_failure("file close failed").await;
            return Err(error.into());
        }
        if let Err(error) = self.storage.rename(&self.temp_path, &self.final_path).await {
            self.cleanup_after_failure("file finalization failed").await;
            return Err(error.into());
        }
        self.cleanup_pending = false;

        let result = sqlx::query_as::<_, StoredFile>(
            r#"
            insert into uploaded_files (name, url, ext, tag, category)
            values ($1, $2, $3, $4, $5)
            returning
                id,
                name,
                url,
                ext,
                tag,
                category,
                to_char(updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') as updated_at
            "#,
        )
        .bind(&self.original_name)
        .bind(&self.public_url)
        .bind(&self.ext)
        .bind(&self.tag)
        .bind(&self.category)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(stored) => Ok(stored),
            Err(error) => {
                if let Err(cleanup_error) = self.storage.delete(&self.final_path).await {
                    tracing::error!(%cleanup_error, "failed to remove uploaded object after metadata failure");
                }
                Err(error.into())
            }
        }
    }
}

impl FileService {
    pub fn local(pool: PgPool, root: impl AsRef<Path>) -> Result<Self, FileError> {
        let root = root.as_ref().to_path_buf();
        let root_value = root.to_string_lossy();
        if root_value.trim().is_empty() {
            return Err(FileError::InvalidConfiguration(
                "FILE_STORAGE_LOCAL_ROOT must not be empty",
            ));
        }
        let builder = services::Fs::default().root(&root_value);
        let storage = Operator::new(builder)?;
        Ok(Self {
            pool,
            storage,
            public_url_prefix: "/uploads".to_string(),
            local_root: Some(root),
        })
    }

    pub fn s3(pool: PgPool, config: S3StorageConfig) -> Result<Self, FileError> {
        if config.bucket.trim().is_empty() {
            return Err(FileError::InvalidConfiguration(
                "S3_BUCKET must not be empty",
            ));
        }
        if config.public_base_url.trim().is_empty() {
            return Err(FileError::InvalidConfiguration(
                "S3_PUBLIC_BASE_URL must not be empty",
            ));
        }
        let access_key_id = non_empty(config.access_key_id.as_deref());
        let secret_access_key = non_empty(config.secret_access_key.as_deref());
        if access_key_id.is_some() != secret_access_key.is_some() {
            return Err(FileError::InvalidConfiguration(
                "AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY must be set together",
            ));
        }
        let session_token = non_empty(config.session_token.as_deref());
        if session_token.is_some() && access_key_id.is_none() {
            return Err(FileError::InvalidConfiguration(
                "AWS_SESSION_TOKEN requires explicit AWS access keys",
            ));
        }

        let root = config.root.trim_matches('/');
        let mut builder = services::S3::default()
            .bucket(config.bucket.trim())
            .root(root);
        if config.enable_virtual_host_style {
            builder = builder.enable_virtual_host_style();
        }
        if let Some(region) = non_empty(config.region.as_deref()) {
            builder = builder.region(region);
        }
        if let Some(endpoint) = non_empty(config.endpoint.as_deref()) {
            builder = builder.endpoint(endpoint);
        }
        if let Some(access_key_id) = access_key_id {
            builder = builder.access_key_id(access_key_id);
        }
        if let Some(secret_access_key) = secret_access_key {
            builder = builder.secret_access_key(secret_access_key);
        }
        if let Some(session_token) = session_token {
            builder = builder.session_token(session_token);
        }
        let storage = Operator::new(builder)?;
        let public_url_prefix = if root.is_empty() {
            config.public_base_url.trim_end_matches('/').to_string()
        } else {
            format!("{}/{root}", config.public_base_url.trim_end_matches('/'))
        };
        Ok(Self {
            pool,
            storage,
            public_url_prefix,
            local_root: None,
        })
    }

    pub fn local_root(&self) -> Option<&Path> {
        self.local_root.as_deref()
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
        let temp_path = format!(".{id}.uploading");
        let final_path = stored_name.clone();
        let public_url = format!("{}/{}", self.public_url_prefix, stored_name);
        let writer = self.storage.writer(&temp_path).await?;
        let local_temp_path = self.local_root.as_ref().map(|root| root.join(&temp_path));

        Ok(FileUpload {
            pool: self.pool.clone(),
            storage: self.storage.clone(),
            original_name: name.to_string(),
            ext,
            tag: tag.to_string(),
            category: category.to_string(),
            temp_path: temp_path.clone(),
            final_path,
            public_url,
            local_temp_path,
            writer: Some(writer),
            cleanup_pending: true,
            size: 0,
        })
    }
    pub async fn delete(&self, id: i64) -> Result<(), FileError> {
        let Some(file) = find_file(&self.pool, id).await? else {
            return Ok(());
        };
        let staged = self.stage_managed_object(&file.url).await?;
        if let Err(error) = delete_file(&self.pool, id).await {
            if let Some((original, staged)) = staged {
                self.storage.rename(&staged, &original).await?;
            }
            return Err(error);
        }
        if let Some((original, staged)) = staged
            && let Err(error) = self.storage.delete(&staged).await
        {
            if let Err(restore_error) = self.storage.rename(&staged, &original).await {
                tracing::error!(%restore_error, %original, "failed to restore managed object after delete failure");
            } else if let Err(restore_error) = restore_file(&self.pool, &file).await {
                tracing::error!(%restore_error, id = file.id, "failed to restore file metadata after delete failure");
            }
            return Err(error.into());
        }
        Ok(())
    }
    async fn stage_managed_object(&self, url: &str) -> Result<Option<(String, String)>, FileError> {
        let prefix = format!("{}/", self.public_url_prefix.trim_end_matches('/'));
        let Some(original) = url.strip_prefix(&prefix) else {
            return Ok(None);
        };
        if original.is_empty() || original.contains('/') {
            return Ok(None);
        }
        let staged = format!(".{original}.deleting-{}", Uuid::new_v4());
        match self.storage.rename(original, &staged).await {
            Ok(()) => Ok(Some((original.to_string(), staged))),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) async fn list(
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
            name,
            url,
            ext,
            tag,
            category,
            to_char(updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') as updated_at
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

pub(crate) async fn edit_name(pool: &sqlx::PgPool, payload: RenameFile) -> Result<(), FileError> {
    sqlx::query("update uploaded_files set name = $1, updated_at = now() where id = $2")
        .bind(payload.name)
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn find_file(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<Option<StoredFile>, FileError> {
    Ok(sqlx::query_as::<_, StoredFile>(
        r#"
        select
            id,
            name,
            url,
            ext,
            tag,
            category,
            to_char(updated_at, 'YYYY-MM-DD"T"HH24:MI:SS') as updated_at
        from uploaded_files
        where id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

pub(crate) async fn delete_file(pool: &sqlx::PgPool, id: i64) -> Result<(), FileError> {
    sqlx::query("delete from uploaded_files where id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

async fn restore_file(pool: &sqlx::PgPool, file: &StoredFile) -> Result<(), FileError> {
    sqlx::query(
        r#"
        insert into uploaded_files (id, name, url, ext, tag, category, updated_at)
        values ($1, $2, $3, $4, $5, $6, $7::timestamptz)
        "#,
    )
    .bind(file.id)
    .bind(&file.name)
    .bind(&file.url)
    .bind(&file.ext)
    .bind(&file.tag)
    .bind(&file.category)
    .bind(&file.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn import_url(
    pool: &sqlx::PgPool,
    payload: ImportFileUrl,
) -> Result<(), FileError> {
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

    #[tokio::test]
    async fn storage_configuration_rejects_missing_required_values() {
        use sqlx::postgres::PgPoolOptions;

        use super::{FileError, FileService, S3StorageConfig};

        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .expect("lazy pool should be created");
        assert!(matches!(
            FileService::local(pool.clone(), "")
                .err()
                .expect("empty local root should fail"),
            FileError::InvalidConfiguration(_)
        ));

        let s3 = S3StorageConfig {
            bucket: String::new(),
            region: None,
            endpoint: None,
            root: "uploads".to_string(),
            public_base_url: "https://cdn.example.test".to_string(),
            access_key_id: None,
            secret_access_key: None,
            session_token: None,
            enable_virtual_host_style: false,
        };
        assert!(matches!(
            FileService::s3(pool, s3)
                .err()
                .expect("empty S3 bucket should fail"),
            FileError::InvalidConfiguration(_)
        ));

        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .expect("lazy pool should be created");
        let incomplete_credentials = S3StorageConfig {
            bucket: "files".to_string(),
            region: None,
            endpoint: None,
            root: "uploads".to_string(),
            public_base_url: "https://cdn.example.test".to_string(),
            access_key_id: Some("access-key".to_string()),
            secret_access_key: None,
            session_token: None,
            enable_virtual_host_style: false,
        };
        assert!(matches!(
            FileService::s3(pool, incomplete_credentials)
                .err()
                .expect("partial S3 credentials should fail"),
            FileError::InvalidConfiguration(_)
        ));

        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .expect("lazy pool should be created");
        let configured = FileService::s3(
            pool,
            S3StorageConfig {
                bucket: "files".to_string(),
                region: Some("auto".to_string()),
                endpoint: Some("https://s3.example.test".to_string()),
                root: "/tenant/uploads/".to_string(),
                public_base_url: "https://cdn.example.test/".to_string(),
                access_key_id: Some("access-key".to_string()),
                secret_access_key: Some("secret-key".to_string()),
                session_token: None,
                enable_virtual_host_style: false,
            },
        )
        .expect("complete S3 configuration should build without network access");
        assert_eq!(
            configured.public_url_prefix,
            "https://cdn.example.test/tenant/uploads"
        );
        assert!(configured.local_root().is_none());
    }
}
