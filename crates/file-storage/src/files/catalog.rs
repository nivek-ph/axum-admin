use std::path::Path;

use sqlx::PgPool;

use super::service::FileService;
use crate::files::{FileError, FileListQuery, ImportFileUrl, RenameFile, StoredFile};

impl FileService {
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
}

async fn list(
    pool: &PgPool,
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

async fn edit_name(pool: &PgPool, payload: RenameFile) -> Result<(), FileError> {
    sqlx::query(
        "update uploaded_files set name = $1, updated_at = now() where id = $2 and not deletion_pending",
    )
    .bind(payload.name)
    .bind(payload.id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn import_url(pool: &PgPool, payload: ImportFileUrl) -> Result<(), FileError> {
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

pub(super) fn safe_extension(value: &str) -> String {
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
    use super::safe_extension;

    #[test]
    fn extension_is_normalized_without_query_or_fragment() {
        assert_eq!(safe_extension("report.PDF?download=1"), "pdf");
        assert_eq!(safe_extension("archive.tar.gz#latest"), "gz");
        assert_eq!(safe_extension("README"), "");
        assert_eq!(safe_extension("unsafe.bad/ext"), "");
    }
}
