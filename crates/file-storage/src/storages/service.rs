use std::str::FromStr;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Transaction};

use super::{
    S3Credentials, StorageBackendConfig, StorageBackendInput, StorageDriver, StorageError,
    StorageInput, StorageQuery, StorageView, model::StorageRecord,
};
use crate::files::storage::FileObjectStorage;

const STORAGE_LIFECYCLE_LOCK: i64 = 0x4156_415f_5354_4f52;

const STORAGE_SELECT: &str = r#"
    select
        id,
        name,
        code,
        driver,
        root,
        bucket,
        region,
        endpoint,
        public_base_url,
        access_key,
        secret_key,
        virtual_host_style,
        enabled,
        is_default,
        sort,
        description,
        created_at,
        updated_at
    from sys_storages
"#;

#[derive(Clone)]
pub(crate) struct StorageEntry {
    pub(crate) id: i64,
    pub(crate) storage: FileObjectStorage,
}

#[derive(Clone)]
pub struct StorageService {
    pool: PgPool,
}

impl StorageService {
    pub async fn load(pool: PgPool) -> Result<Self, StorageError> {
        let records = fetch_all(&pool).await?;

        let mut has_default = false;
        for record in &records {
            storage_from_record(record)?;
            if record.is_default {
                if !record.enabled {
                    return Err(StorageError::DisabledDefault);
                }
                has_default = true;
            }
        }
        if !has_default {
            return Err(StorageError::DisabledDefault);
        }
        Ok(Self { pool })
    }

    pub(crate) async fn default_entry_locked(
        &self,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<StorageEntry, StorageError> {
        lock_storage_lifecycle(transaction).await?;
        let sql = format!("{STORAGE_SELECT} where is_default = true");
        let record = sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
            .fetch_optional(&mut **transaction)
            .await?
            .ok_or(StorageError::DisabledDefault)?;
        if !record.enabled {
            return Err(StorageError::DisabledDefault);
        }
        self.entry_from_record(&record)
    }

    pub(crate) async fn entry_for_id(&self, id: i64) -> Result<StorageEntry, StorageError> {
        let record = fetch_one(&self.pool, id).await?;
        self.entry_from_record(&record)
    }

    fn entry_from_record(&self, record: &StorageRecord) -> Result<StorageEntry, StorageError> {
        Ok(StorageEntry {
            id: record.id,
            storage: storage_from_record(record)?,
        })
    }

    pub async fn list(&self, query: StorageQuery) -> Result<Vec<StorageView>, StorageError> {
        let sql = format!(
            r#"{STORAGE_SELECT}
            where ($1::text is null or name ilike '%' || $1 || '%' or code ilike '%' || $1 || '%')
              and ($2::text is null or driver = $2)
            order by sort asc, id asc"#
        );
        let records = sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
            .bind(query.keyword.as_deref())
            .bind(query.driver.as_deref())
            .fetch_all(&self.pool)
            .await?;
        Ok(records.into_iter().map(StorageView::from).collect())
    }

    pub async fn find(&self, id: i64) -> Result<StorageView, StorageError> {
        Ok(fetch_one(&self.pool, id).await?.into())
    }

    pub async fn create(&self, payload: StorageInput) -> Result<StorageView, StorageError> {
        validate_input(&payload)?;
        let config = config_from_input(&payload, None)?;
        FileObjectStorage::from_config(&config)?;
        let credentials = config.credentials();
        let result = sqlx::query_scalar::<_, i64>(
            r#"
            insert into sys_storages (
                name, code, driver, root, bucket, region, endpoint,
                public_base_url, access_key, secret_key, virtual_host_style, enabled,
                is_default, sort, description
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, false, $13, $14)
            returning id
            "#,
        )
        .bind(payload.name.trim())
        .bind(payload.code.trim())
        .bind(config.driver().as_str())
        .bind(config.root())
        .bind(config.bucket())
        .bind(config.region())
        .bind(config.endpoint())
        .bind(config.public_base_url())
        .bind(credentials.map(|value| value.access_key.as_str()))
        .bind(credentials.map(|value| value.secret_key.as_str()))
        .bind(config.virtual_host_style())
        .bind(payload.enabled)
        .bind(payload.sort)
        .bind(payload.description.trim())
        .fetch_one(&self.pool)
        .await
        .map_err(map_database_error)?;
        self.find(result).await
    }

    pub async fn update(
        &self,
        id: i64,
        payload: StorageInput,
    ) -> Result<StorageView, StorageError> {
        validate_input(&payload)?;
        let mut transaction = self.pool.begin().await?;
        lock_storage_lifecycle(&mut transaction).await?;
        let current = fetch_one_locked(&mut transaction, id).await?;
        if current.code != payload.code.trim()
            || current.driver != payload.backend.driver().as_str()
        {
            return Err(StorageError::ImmutableIdentity);
        }
        if current.is_default && !payload.enabled {
            return Err(StorageError::DefaultProtected);
        }
        let current_config = config_from_record(&current)?;
        let config = config_from_input(&payload, Some(&current))?;
        if !current_config.same_location(&config) {
            return Err(StorageError::ImmutableIdentity);
        }
        FileObjectStorage::from_config(&config)?;
        let credentials = config.credentials();
        sqlx::query(
            r#"
            update sys_storages
            set name = $1,
                root = $2,
                bucket = $3,
                region = $4,
                endpoint = $5,
                public_base_url = $6,
                access_key = $7,
                secret_key = $8,
                virtual_host_style = $9,
                enabled = $10,
                sort = $11,
                description = $12,
                updated_at = now()
            where id = $13
            "#,
        )
        .bind(payload.name.trim())
        .bind(config.root())
        .bind(config.bucket())
        .bind(config.region())
        .bind(config.endpoint())
        .bind(config.public_base_url())
        .bind(credentials.map(|value| value.access_key.as_str()))
        .bind(credentials.map(|value| value.secret_key.as_str()))
        .bind(config.virtual_host_style())
        .bind(payload.enabled)
        .bind(payload.sort)
        .bind(payload.description.trim())
        .bind(id)
        .execute(&mut *transaction)
        .await
        .map_err(map_database_error)?;
        transaction.commit().await?;
        self.find(id).await
    }

    pub async fn set_enabled(&self, id: i64, enabled: bool) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        lock_storage_lifecycle(&mut transaction).await?;
        let current = fetch_one_locked(&mut transaction, id).await?;
        if current.is_default && !enabled {
            return Err(StorageError::DefaultProtected);
        }
        if current.enabled == enabled {
            return Ok(());
        }
        if enabled {
            storage_from_record(&current)?;
        }
        sqlx::query("update sys_storages set enabled = $1, updated_at = now() where id = $2")
            .bind(enabled)
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn set_default(&self, id: i64) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        lock_storage_lifecycle(&mut transaction).await?;
        let current = fetch_one_locked(&mut transaction, id).await?;
        if !current.enabled {
            return Err(StorageError::DisabledDefault);
        }
        if current.is_default {
            return Ok(());
        }
        sqlx::query("update sys_storages set is_default = false where is_default")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("update sys_storages set is_default = true, updated_at = now() where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), StorageError> {
        let mut transaction = self.pool.begin().await?;
        lock_storage_lifecycle(&mut transaction).await?;
        let current = fetch_one_locked(&mut transaction, id).await?;
        if current.is_default {
            return Err(StorageError::DefaultProtected);
        }
        let references: i64 = sqlx::query_scalar(
            r#"
            select
                (select count(*) from uploaded_files where storage_id = $1)
                + (select count(*) from uploaded_file_sessions where storage_id = $1)
            "#,
        )
        .bind(id)
        .fetch_one(&mut *transaction)
        .await?;
        if references > 0 {
            return Err(StorageError::InUse);
        }
        sqlx::query("delete from sys_storages where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}

fn validate_input(payload: &StorageInput) -> Result<(), StorageError> {
    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(StorageError::InvalidInput(
            "name must contain 1 to 100 characters",
        ));
    }
    let code = payload.code.trim();
    let mut chars = code.chars();
    if !(2..=30).contains(&code.len())
        || !chars
            .next()
            .is_some_and(|value| value.is_ascii_alphabetic())
        || !chars.all(|value| value.is_ascii_alphanumeric() || value == '_')
    {
        return Err(StorageError::InvalidInput(
            "code must start with a letter and contain 2 to 30 letters, digits, or underscores",
        ));
    }
    if payload.description.chars().count() > 200 {
        return Err(StorageError::InvalidInput(
            "description must not exceed 200 characters",
        ));
    }
    Ok(())
}

async fn fetch_all(pool: &PgPool) -> Result<Vec<StorageRecord>, sqlx::Error> {
    let sql = format!("{STORAGE_SELECT} order by sort asc, id asc");
    sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
        .fetch_all(pool)
        .await
}

async fn fetch_one(pool: &PgPool, id: i64) -> Result<StorageRecord, StorageError> {
    let sql = format!("{STORAGE_SELECT} where id = $1");
    sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(StorageError::NotFound)
}

async fn fetch_one_locked(
    transaction: &mut Transaction<'_, Postgres>,
    id: i64,
) -> Result<StorageRecord, StorageError> {
    let sql = format!("{STORAGE_SELECT} where id = $1 for update");
    sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await?
        .ok_or(StorageError::NotFound)
}

async fn lock_storage_lifecycle(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    sqlx::query("select pg_advisory_xact_lock($1)")
        .bind(STORAGE_LIFECYCLE_LOCK)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

fn config_from_input(
    payload: &StorageInput,
    current: Option<&StorageRecord>,
) -> Result<StorageBackendConfig, StorageError> {
    match &payload.backend {
        StorageBackendInput::Local { root } => Ok(StorageBackendConfig::Local {
            root: root.trim().to_string(),
        }),
        StorageBackendInput::S3 {
            root,
            bucket,
            region,
            endpoint,
            public_base_url,
            access_key,
            secret_key,
            virtual_host_style,
        } => {
            let access_key = merge_secret(
                access_key.as_deref(),
                current.and_then(|value| value.access_key.as_deref()),
            );
            let secret_key = merge_secret(
                secret_key.as_deref(),
                current.and_then(|value| value.secret_key.as_deref()),
            );
            Ok(StorageBackendConfig::S3 {
                root: optional_owned(root.as_deref()),
                bucket: bucket.trim().to_string(),
                region: region.trim().to_string(),
                endpoint: optional_owned(endpoint.as_deref()),
                public_base_url: public_base_url.trim().trim_end_matches('/').to_string(),
                credentials: credentials(access_key, secret_key)?,
                virtual_host_style: *virtual_host_style,
            })
        }
    }
}

fn storage_from_record(record: &StorageRecord) -> Result<FileObjectStorage, StorageError> {
    let config = config_from_record(record)?;
    Ok(FileObjectStorage::from_config(&config)?)
}

fn config_from_record(record: &StorageRecord) -> Result<StorageBackendConfig, StorageError> {
    match StorageDriver::from_str(&record.driver)? {
        StorageDriver::Local => Ok(StorageBackendConfig::Local {
            root: record
                .root
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
        }),
        StorageDriver::S3 => Ok(StorageBackendConfig::S3 {
            root: optional_owned(record.root.as_deref()),
            bucket: record
                .bucket
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
            region: record
                .region
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string(),
            endpoint: optional_owned(record.endpoint.as_deref()),
            public_base_url: record
                .public_base_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .trim_end_matches('/')
                .to_string(),
            credentials: credentials(record.access_key.clone(), record.secret_key.clone())?,
            virtual_host_style: record.virtual_host_style,
        }),
    }
}

fn merge_secret(replacement: Option<&str>, current: Option<&str>) -> Option<String> {
    match replacement.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Some(value.to_string()),
        None => current.map(str::to_string),
    }
}

fn credentials(
    access_key: Option<String>,
    secret_key: Option<String>,
) -> Result<S3Credentials, StorageError> {
    let access_key = optional_owned(access_key.as_deref());
    let secret_key = optional_owned(secret_key.as_deref());
    match (access_key, secret_key) {
        (Some(access_key), Some(secret_key)) => Ok(S3Credentials {
            access_key,
            secret_key,
        }),
        (None, None) => Err(crate::files::ObjectStorageError::MissingCredentials.into()),
        _ => Err(crate::files::ObjectStorageError::IncompleteCredentials.into()),
    }
}

fn optional_owned(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn map_database_error(error: sqlx::Error) -> StorageError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        StorageError::CodeConflict
    } else {
        StorageError::Database(error)
    }
}
