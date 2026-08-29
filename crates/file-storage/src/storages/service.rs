use std::{collections::HashMap, str::FromStr, sync::Arc};

use sqlx::{AssertSqlSafe, PgPool};
use tokio::sync::RwLock;

use super::{
    StorageDriver, StorageError, StorageInput, StorageQuery, StorageView, model::StorageRecord,
};
use crate::files::{FileStorage, storage::FileObjectStorage};

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
pub(crate) struct StorageRegistryEntry {
    pub(crate) id: Option<i64>,
    pub(crate) storage: FileObjectStorage,
}

#[derive(Clone)]
pub(crate) struct StorageRegistry {
    inner: Arc<RwLock<StorageRegistryState>>,
}

struct StorageRegistryState {
    default_id: Option<i64>,
    fallback: StorageRegistryEntry,
    by_id: HashMap<i64, StorageRegistryEntry>,
}

impl StorageRegistry {
    pub(crate) fn fallback(storage: FileObjectStorage) -> Self {
        let fallback = StorageRegistryEntry { id: None, storage };
        Self {
            inner: Arc::new(RwLock::new(StorageRegistryState {
                default_id: None,
                fallback,
                by_id: HashMap::new(),
            })),
        }
    }

    async fn managed(
        fallback: FileObjectStorage,
        default_id: i64,
        by_id: HashMap<i64, StorageRegistryEntry>,
    ) -> Self {
        Self {
            inner: Arc::new(RwLock::new(StorageRegistryState {
                default_id: Some(default_id),
                fallback: StorageRegistryEntry {
                    id: None,
                    storage: fallback,
                },
                by_id,
            })),
        }
    }

    pub(crate) async fn default(&self) -> StorageRegistryEntry {
        let state = self.inner.read().await;
        state
            .default_id
            .and_then(|id| state.by_id.get(&id))
            .cloned()
            .unwrap_or_else(|| state.fallback.clone())
    }

    pub(crate) async fn by_id_or_default(&self, id: Option<i64>) -> StorageRegistryEntry {
        let state = self.inner.read().await;
        id.and_then(|id| state.by_id.get(&id))
            .cloned()
            .or_else(|| {
                state
                    .default_id
                    .and_then(|default_id| state.by_id.get(&default_id))
                    .cloned()
            })
            .unwrap_or_else(|| state.fallback.clone())
    }

    async fn upsert(&self, entry: StorageRegistryEntry) {
        if let Some(id) = entry.id {
            self.inner.write().await.by_id.insert(id, entry);
        }
    }

    async fn remove(&self, id: i64) {
        self.inner.write().await.by_id.remove(&id);
    }

    async fn set_default(&self, id: i64) {
        self.inner.write().await.default_id = Some(id);
    }
}

#[derive(Clone)]
pub struct StorageService {
    pool: PgPool,
    registry: StorageRegistry,
}

struct PreparedConfig {
    driver: StorageDriver,
    root: Option<String>,
    bucket: Option<String>,
    region: Option<String>,
    endpoint: Option<String>,
    public_base_url: Option<String>,
    access_key: Option<String>,
    secret_key: Option<String>,
    virtual_host_style: bool,
    storage: FileObjectStorage,
}

impl StorageService {
    pub async fn load(pool: PgPool) -> Result<Self, StorageError> {
        let records = fetch_all(&pool).await?;

        let mut by_id = HashMap::new();
        let mut default_id = None;
        for record in &records {
            let storage = storage_from_record(record)?;
            if record.is_default {
                if !record.enabled {
                    return Err(StorageError::DisabledDefault);
                }
                default_id = Some(record.id);
            }
            by_id.insert(
                record.id,
                StorageRegistryEntry {
                    id: Some(record.id),
                    storage,
                },
            );
        }
        let default_id = default_id.ok_or(StorageError::DisabledDefault)?;
        let fallback = by_id
            .get(&default_id)
            .map(|entry| entry.storage.clone())
            .ok_or(StorageError::DisabledDefault)?;
        let registry = StorageRegistry::managed(fallback, default_id, by_id).await;
        Ok(Self { pool, registry })
    }

    pub(crate) fn registry(&self) -> StorageRegistry {
        self.registry.clone()
    }

    pub(crate) async fn default_entry(&self) -> Result<StorageRegistryEntry, StorageError> {
        let sql = format!("{STORAGE_SELECT} where is_default = true");
        let record = sqlx::query_as::<_, StorageRecord>(AssertSqlSafe(sql))
            .fetch_optional(&self.pool)
            .await?
            .ok_or(StorageError::DisabledDefault)?;
        if !record.enabled {
            return Err(StorageError::DisabledDefault);
        }
        let entry = self.entry_from_record(&record)?;
        self.registry.upsert(entry.clone()).await;
        self.registry.set_default(record.id).await;
        Ok(entry)
    }

    pub(crate) async fn entry_for_id(
        &self,
        id: Option<i64>,
    ) -> Result<StorageRegistryEntry, StorageError> {
        let Some(id) = id else {
            return self.default_entry().await;
        };
        let record = fetch_one(&self.pool, id).await?;
        let entry = self.entry_from_record(&record)?;
        self.registry.upsert(entry.clone()).await;
        Ok(entry)
    }

    fn entry_from_record(
        &self,
        record: &StorageRecord,
    ) -> Result<StorageRegistryEntry, StorageError> {
        Ok(StorageRegistryEntry {
            id: Some(record.id),
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
        let prepared = prepare_input(&payload, None)?;
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
        .bind(prepared.driver.as_str())
        .bind(&prepared.root)
        .bind(&prepared.bucket)
        .bind(&prepared.region)
        .bind(&prepared.endpoint)
        .bind(&prepared.public_base_url)
        .bind(&prepared.access_key)
        .bind(&prepared.secret_key)
        .bind(prepared.virtual_host_style)
        .bind(payload.enabled)
        .bind(payload.sort)
        .bind(payload.description.trim())
        .fetch_one(&self.pool)
        .await
        .map_err(map_database_error)?;
        self.registry
            .upsert(StorageRegistryEntry {
                id: Some(result),
                storage: prepared.storage,
            })
            .await;
        self.find(result).await
    }

    pub async fn update(
        &self,
        id: i64,
        payload: StorageInput,
    ) -> Result<StorageView, StorageError> {
        validate_input(&payload)?;
        let current = fetch_one(&self.pool, id).await?;
        if current.code != payload.code.trim() || current.driver != payload.driver.trim() {
            return Err(StorageError::ImmutableIdentity);
        }
        if current.is_default && !payload.enabled {
            return Err(StorageError::DefaultProtected);
        }
        let prepared = prepare_input(&payload, Some(&current))?;
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
        .bind(&prepared.root)
        .bind(&prepared.bucket)
        .bind(&prepared.region)
        .bind(&prepared.endpoint)
        .bind(&prepared.public_base_url)
        .bind(&prepared.access_key)
        .bind(&prepared.secret_key)
        .bind(prepared.virtual_host_style)
        .bind(payload.enabled)
        .bind(payload.sort)
        .bind(payload.description.trim())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(map_database_error)?;
        self.registry
            .upsert(StorageRegistryEntry {
                id: Some(id),
                storage: prepared.storage,
            })
            .await;
        self.find(id).await
    }

    pub async fn set_enabled(&self, id: i64, enabled: bool) -> Result<(), StorageError> {
        let current = fetch_one(&self.pool, id).await?;
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
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_default(&self, id: i64) -> Result<(), StorageError> {
        let current = fetch_one(&self.pool, id).await?;
        if !current.enabled {
            return Err(StorageError::DisabledDefault);
        }
        if current.is_default {
            return Ok(());
        }
        let mut transaction = self.pool.begin().await?;
        sqlx::query("update sys_storages set is_default = false where is_default")
            .execute(&mut *transaction)
            .await?;
        sqlx::query("update sys_storages set is_default = true, updated_at = now() where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.registry.set_default(id).await;
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<(), StorageError> {
        let current = fetch_one(&self.pool, id).await?;
        if current.is_default {
            return Err(StorageError::DefaultProtected);
        }
        let references: i64 =
            sqlx::query_scalar("select count(*) from uploaded_files where storage_id = $1")
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        if references > 0 {
            return Err(StorageError::InUse);
        }
        sqlx::query("delete from sys_storages where id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        self.registry.remove(id).await;
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

fn prepare_input(
    payload: &StorageInput,
    current: Option<&StorageRecord>,
) -> Result<PreparedConfig, StorageError> {
    let driver = StorageDriver::from_str(&payload.driver)?;
    let access_key = merge_secret(
        payload.access_key.as_deref(),
        current.and_then(|value| value.access_key.as_deref()),
    );
    let secret_key = merge_secret(
        payload.secret_key.as_deref(),
        current.and_then(|value| value.secret_key.as_deref()),
    );
    let runtime = FileStorage {
        driver: driver.as_str().to_string(),
        local_root: payload.root.clone().unwrap_or_default(),
        s3_bucket: payload.bucket.clone().unwrap_or_default(),
        s3_region: payload.region.clone().unwrap_or_default(),
        s3_endpoint: payload.endpoint.clone().unwrap_or_default(),
        s3_prefix: payload.root.clone().unwrap_or_default(),
        s3_public_base_url: payload.public_base_url.clone().unwrap_or_default(),
        s3_access_key_id: access_key.clone().unwrap_or_default(),
        s3_secret_access_key: secret_key.clone().unwrap_or_default(),
        s3_virtual_host_style: payload.virtual_host_style,
    };
    let storage = FileObjectStorage::from_config(&runtime)?;
    Ok(PreparedConfig {
        driver,
        root: match driver {
            StorageDriver::Local => Some(
                payload
                    .root
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            ),
            StorageDriver::S3 => optional_owned(payload.root.as_deref().unwrap_or_default()),
        },
        bucket: (driver == StorageDriver::S3).then(|| {
            payload
                .bucket
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()
        }),
        region: (driver == StorageDriver::S3).then(|| {
            payload
                .region
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()
        }),
        endpoint: (driver == StorageDriver::S3)
            .then(|| optional_owned(payload.endpoint.as_deref().unwrap_or_default()))
            .flatten(),
        public_base_url: (driver == StorageDriver::S3).then(|| {
            payload
                .public_base_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string()
        }),
        access_key: (driver == StorageDriver::S3)
            .then_some(access_key)
            .flatten(),
        secret_key: (driver == StorageDriver::S3)
            .then_some(secret_key)
            .flatten(),
        virtual_host_style: driver == StorageDriver::S3 && payload.virtual_host_style,
        storage,
    })
}

fn storage_from_record(record: &StorageRecord) -> Result<FileObjectStorage, StorageError> {
    let config = FileStorage {
        driver: record.driver.clone(),
        local_root: record.root.clone().unwrap_or_default(),
        s3_bucket: record.bucket.clone().unwrap_or_default(),
        s3_region: record.region.clone().unwrap_or_default(),
        s3_endpoint: record.endpoint.clone().unwrap_or_default(),
        s3_prefix: record.root.clone().unwrap_or_default(),
        s3_public_base_url: record.public_base_url.clone().unwrap_or_default(),
        s3_access_key_id: record.access_key.clone().unwrap_or_default(),
        s3_secret_access_key: record.secret_key.clone().unwrap_or_default(),
        s3_virtual_host_style: record.virtual_host_style,
    };
    Ok(FileObjectStorage::from_config(&config)?)
}

fn merge_secret(replacement: Option<&str>, current: Option<&str>) -> Option<String> {
    match replacement.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Some(value.to_string()),
        None => current.map(str::to_string),
    }
}

fn optional_owned(value: &str) -> Option<String> {
    let value = value.trim();
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
