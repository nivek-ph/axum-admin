use std::str::FromStr;

use sqlx::FromRow;

use super::StorageError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageDriver {
    Local,
    S3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct S3Credentials {
    pub(super) access_key: String,
    pub(super) secret_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StorageBackendConfig {
    Local {
        root: String,
    },
    S3 {
        root: Option<String>,
        bucket: String,
        region: String,
        endpoint: Option<String>,
        public_base_url: String,
        credentials: S3Credentials,
        virtual_host_style: bool,
    },
}

impl StorageBackendConfig {
    pub(super) fn driver(&self) -> StorageDriver {
        match self {
            Self::Local { .. } => StorageDriver::Local,
            Self::S3 { .. } => StorageDriver::S3,
        }
    }

    pub(super) fn same_location(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Local { root: left }, Self::Local { root: right }) => left == right,
            (
                Self::S3 {
                    root: left_root,
                    bucket: left_bucket,
                    region: left_region,
                    endpoint: left_endpoint,
                    public_base_url: left_public_base_url,
                    virtual_host_style: left_virtual_host_style,
                    ..
                },
                Self::S3 {
                    root: right_root,
                    bucket: right_bucket,
                    region: right_region,
                    endpoint: right_endpoint,
                    public_base_url: right_public_base_url,
                    virtual_host_style: right_virtual_host_style,
                    ..
                },
            ) => {
                left_root == right_root
                    && left_bucket == right_bucket
                    && left_region == right_region
                    && left_endpoint == right_endpoint
                    && left_public_base_url == right_public_base_url
                    && left_virtual_host_style == right_virtual_host_style
            }
            _ => false,
        }
    }

    pub(super) fn root(&self) -> Option<&str> {
        match self {
            Self::Local { root } => Some(root),
            Self::S3 { root, .. } => root.as_deref(),
        }
    }

    pub(super) fn bucket(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::S3 { bucket, .. } => Some(bucket),
        }
    }

    pub(super) fn region(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::S3 { region, .. } => Some(region),
        }
    }

    pub(super) fn endpoint(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::S3 { endpoint, .. } => endpoint.as_deref(),
        }
    }

    pub(super) fn public_base_url(&self) -> Option<&str> {
        match self {
            Self::Local { .. } => None,
            Self::S3 {
                public_base_url, ..
            } => Some(public_base_url),
        }
    }

    pub(super) fn credentials(&self) -> Option<&S3Credentials> {
        match self {
            Self::Local { .. } => None,
            Self::S3 { credentials, .. } => Some(credentials),
        }
    }

    pub(super) fn virtual_host_style(&self) -> bool {
        match self {
            Self::Local { .. } => false,
            Self::S3 {
                virtual_host_style, ..
            } => *virtual_host_style,
        }
    }
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
                super::ObjectStorageError::UnsupportedDriver(value.to_string()),
            )),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct StorageQuery {
    pub keyword: Option<String>,
    pub driver: Option<StorageDriver>,
}

#[derive(Debug, Clone)]
pub struct StorageInput {
    pub name: String,
    pub code: String,
    pub backend: StorageBackendInput,
    pub enabled: bool,
    pub sort: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum StorageBackendInput {
    Local {
        root: String,
    },
    S3 {
        root: Option<String>,
        bucket: String,
        region: String,
        endpoint: Option<String>,
        public_base_url: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        virtual_host_style: bool,
    },
}

impl StorageBackendInput {
    pub fn driver(&self) -> StorageDriver {
        match self {
            Self::Local { .. } => StorageDriver::Local,
            Self::S3 { .. } => StorageDriver::S3,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StorageView {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub backend: StorageBackendView,
    pub enabled: bool,
    pub is_default: bool,
    pub sort: i32,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub enum StorageBackendView {
    Local {
        root: String,
    },
    S3 {
        root: Option<String>,
        bucket: String,
        region: String,
        endpoint: Option<String>,
        public_base_url: String,
        virtual_host_style: bool,
        has_access_key: bool,
        has_secret_key: bool,
    },
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct StorageRecord {
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

impl TryFrom<StorageRecord> for StorageView {
    type Error = StorageError;

    fn try_from(value: StorageRecord) -> Result<Self, Self::Error> {
        let backend = match StorageDriver::from_str(&value.driver)? {
            StorageDriver::Local => StorageBackendView::Local {
                root: value.root.unwrap_or_default(),
            },
            StorageDriver::S3 => StorageBackendView::S3 {
                root: value.root,
                bucket: value.bucket.unwrap_or_default(),
                region: value.region.unwrap_or_default(),
                endpoint: value.endpoint,
                public_base_url: value.public_base_url.unwrap_or_default(),
                virtual_host_style: value.virtual_host_style,
                has_access_key: value.access_key.is_some(),
                has_secret_key: value.secret_key.is_some(),
            },
        };
        Ok(Self {
            id: value.id,
            name: value.name,
            code: value.code,
            backend,
            enabled: value.enabled,
            is_default: value.is_default,
            sort: value.sort,
            description: value.description,
            created_at: value.created_at.to_jiff().to_string(),
            updated_at: value.updated_at.to_jiff().to_string(),
        })
    }
}
