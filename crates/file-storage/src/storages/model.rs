use std::str::FromStr;

use sqlx::FromRow;

use super::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDriver {
    Local,
    S3,
}

impl StorageDriver {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::S3 => "s3",
        }
    }
}

impl FromStr for StorageDriver {
    type Err = StorageError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "s3" => Ok(Self::S3),
            _ => Err(StorageError::InvalidConfiguration(
                crate::files::FileStorageError::UnsupportedDriver(value.to_string()),
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageQuery {
    pub keyword: Option<String>,
    pub driver: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StorageInput {
    pub name: String,
    pub code: String,
    pub driver: String,
    pub root: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub public_base_url: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub virtual_host_style: bool,
    pub enabled: bool,
    pub sort: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct StorageView {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub driver: String,
    pub root: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub public_base_url: Option<String>,
    pub virtual_host_style: bool,
    pub has_access_key: bool,
    pub has_secret_key: bool,
    pub enabled: bool,
    pub is_default: bool,
    pub sort: i32,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct StorageRecord {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub driver: String,
    pub root: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub public_base_url: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub virtual_host_style: bool,
    pub enabled: bool,
    pub is_default: bool,
    pub sort: i32,
    pub description: String,
    pub created_at: jiff_sqlx::Timestamp,
    pub updated_at: jiff_sqlx::Timestamp,
}

impl From<StorageRecord> for StorageView {
    fn from(value: StorageRecord) -> Self {
        Self {
            id: value.id,
            name: value.name,
            code: value.code,
            driver: value.driver,
            root: value.root,
            bucket: value.bucket,
            region: value.region,
            endpoint: value.endpoint,
            public_base_url: value.public_base_url,
            virtual_host_style: value.virtual_host_style,
            has_access_key: value.access_key.is_some(),
            has_secret_key: value.secret_key.is_some(),
            enabled: value.enabled,
            is_default: value.is_default,
            sort: value.sort,
            description: value.description,
            created_at: value.created_at.to_jiff().to_string(),
            updated_at: value.updated_at.to_jiff().to_string(),
        }
    }
}
