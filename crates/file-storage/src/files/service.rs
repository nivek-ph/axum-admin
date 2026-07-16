use std::path::{Path, PathBuf};

use sqlx::PgPool;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use super::{FileError, FileListQuery, ImportFileUrl, RenameFile, StoredFile};

pub const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;

#[derive(Clone)]
pub struct FileService {
    pool: PgPool,
    upload_dir: String,
}

pub struct FileUpload {
    pool: PgPool,
    original_name: String,
    ext: String,
    tag: String,
    category: String,
    temp_path: PathBuf,
    final_path: PathBuf,
    stored_name: String,
    file: Option<tokio::fs::File>,
    cleanup_path: Option<PathBuf>,
    size: usize,
}

impl Drop for FileUpload {
    fn drop(&mut self) {
        self.file.take();
        if let Some(path) = self.cleanup_path.take() {
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    if let Err(error) = tokio::fs::remove_file(&path).await
                        && error.kind() != std::io::ErrorKind::NotFound
                    {
                        tracing::warn!(?path, %error, "failed to clean up abandoned upload");
                    }
                });
            } else if let Err(error) = std::fs::remove_file(&path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(?path, %error, "failed to clean up abandoned upload");
            }
        }
    }
}

impl FileUpload {
    pub async fn abort(mut self) -> Result<(), FileError> {
        self.file.take();
        if let Some(path) = self.cleanup_path.as_ref() {
            match tokio::fs::remove_file(path).await {
                Ok(()) => self.cleanup_path = None,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.cleanup_path = None;
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    pub async fn write_chunk(&mut self, bytes: &[u8]) -> Result<(), FileError> {
        let size = self
            .size
            .checked_add(bytes.len())
            .filter(|size| *size <= MAX_UPLOAD_BYTES)
            .ok_or(FileError::TooLarge)?;
        if let Some(file) = self.file.as_mut() {
            file.write_all(bytes).await?;
        }
        self.size = size;
        Ok(())
    }

    pub async fn finish(mut self) -> Result<StoredFile, FileError> {
        if let Some(mut file) = self.file.take()
            && let Err(error) = file.flush().await
        {
            drop(file);
            self.abort().await?;
            return Err(error.into());
        }
        if let Err(error) = tokio::fs::rename(&self.temp_path, &self.final_path).await {
            self.abort().await?;
            return Err(error.into());
        }
        self.cleanup_path = Some(self.final_path.clone());

        let url = format!("/uploads/{}", self.stored_name);
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
        .bind(&url)
        .bind(&self.ext)
        .bind(&self.tag)
        .bind(&self.category)
        .fetch_one(&self.pool)
        .await;

        match result {
            Ok(stored) => {
                self.cleanup_path = None;
                Ok(stored)
            }
            Err(error) => {
                if let Some(path) = self.cleanup_path.as_ref() {
                    match tokio::fs::remove_file(path).await {
                        Ok(()) => self.cleanup_path = None,
                        Err(cleanup_error)
                            if cleanup_error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            self.cleanup_path = None;
                        }
                        Err(cleanup_error) => return Err(cleanup_error.into()),
                    }
                }
                Err(error.into())
            }
        }
    }
}

impl FileService {
    pub fn new(pool: PgPool, upload_dir: impl Into<String>) -> Self {
        Self {
            pool,
            upload_dir: upload_dir.into(),
        }
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
        tokio::fs::create_dir_all(&self.upload_dir).await?;
        let ext = safe_extension(name);
        let id = Uuid::new_v4();
        let stored_name = if ext.is_empty() {
            id.to_string()
        } else {
            format!("{id}.{ext}")
        };
        let upload_dir = Path::new(&self.upload_dir);
        let temp_path = upload_dir.join(format!(".{id}.uploading"));
        let final_path = upload_dir.join(&stored_name);
        let file = tokio::fs::File::create(&temp_path).await?;

        Ok(FileUpload {
            pool: self.pool.clone(),
            original_name: name.to_string(),
            ext,
            tag: tag.to_string(),
            category: category.to_string(),
            temp_path: temp_path.clone(),
            final_path,
            stored_name,
            file: Some(file),
            cleanup_path: Some(temp_path),
            size: 0,
        })
    }
    pub async fn delete(&self, id: i64) -> Result<(), FileError> {
        let Some(file) = find_file(&self.pool, id).await? else {
            return Ok(());
        };
        let staged = self.stage_local_file(&file.url).await?;
        if let Err(error) = delete_file(&self.pool, id).await {
            if let Some((original, staged)) = staged {
                tokio::fs::rename(staged, original).await?;
            }
            return Err(error);
        }
        if let Some((_, staged)) = staged {
            match tokio::fs::remove_file(staged).await {
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                    return Err(error.into());
                }
                _ => {}
            }
        }
        Ok(())
    }
    async fn stage_local_file(&self, url: &str) -> Result<Option<(PathBuf, PathBuf)>, FileError> {
        if !url.starts_with("/uploads/") {
            return Ok(None);
        }
        let Some(name) = Path::new(url).file_name() else {
            return Ok(None);
        };
        let original = Path::new(&self.upload_dir).join(name);
        let staged = original.with_extension(format!("deleting-{}", Uuid::new_v4()));
        match tokio::fs::rename(&original, &staged).await {
            Ok(()) => Ok(Some((original, staged))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
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
    async fn disk_write_failure_can_be_aborted_without_leaving_upload_state() {
        use sqlx::postgres::PgPoolOptions;
        use uuid::Uuid;

        use super::{FileError, FileUpload};

        let upload_dir =
            std::env::temp_dir().join(format!("ava-file-write-failure-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&upload_dir)
            .await
            .expect("test upload directory should be created");
        let temp_path = upload_dir.join("partial.uploading");
        tokio::fs::write(&temp_path, b"partial")
            .await
            .expect("test temporary file should be created");
        let read_only_file = tokio::fs::OpenOptions::new()
            .read(true)
            .open(&temp_path)
            .await
            .expect("temporary file should open read-only");
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .expect("lazy pool should be created");
        let mut upload = FileUpload {
            pool,
            original_name: "report.pdf".to_string(),
            ext: "pdf".to_string(),
            tag: String::new(),
            category: String::new(),
            temp_path: temp_path.clone(),
            final_path: upload_dir.join("report.pdf"),
            stored_name: "report.pdf".to_string(),
            file: Some(read_only_file),
            cleanup_path: Some(temp_path.clone()),
            size: 0,
        };

        upload
            .write_chunk(b"content")
            .await
            .expect("buffered write may complete before the disk operation");
        let error = upload
            .finish()
            .await
            .expect_err("flushing the read-only file should reject the upload");
        assert!(matches!(error, FileError::Io(_)));
        assert!(!temp_path.exists());

        tokio::fs::remove_dir_all(upload_dir)
            .await
            .expect("test upload directory should be removed");
    }
}
