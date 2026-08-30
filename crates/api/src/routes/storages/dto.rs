use file_storage::storages::{
    StorageBackendInput, StorageError, StorageInput, StorageQuery, StorageView,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Default, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(rename_all = "camelCase")]
pub struct StorageListRequest {
    pub keyword: Option<String>,
    pub driver: Option<String>,
}

impl From<StorageListRequest> for StorageQuery {
    fn from(value: StorageListRequest) -> Self {
        Self {
            keyword: value.keyword,
            driver: value.driver,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StorageRequest {
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
    #[serde(default)]
    pub virtual_host_style: bool,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub sort: i32,
    #[serde(default)]
    pub description: String,
}

fn enabled_by_default() -> bool {
    true
}

impl TryFrom<StorageRequest> for StorageInput {
    type Error = StorageError;

    fn try_from(value: StorageRequest) -> Result<Self, Self::Error> {
        let backend = match value.driver.trim().to_ascii_lowercase().as_str() {
            "local" => {
                if [
                    value.bucket.as_deref(),
                    value.region.as_deref(),
                    value.endpoint.as_deref(),
                    value.public_base_url.as_deref(),
                    value.access_key.as_deref(),
                    value.secret_key.as_deref(),
                ]
                .into_iter()
                .flatten()
                .any(|field| !field.trim().is_empty())
                    || value.virtual_host_style
                {
                    return Err(StorageError::InvalidInput(
                        "local storage cannot include S3 settings",
                    ));
                }
                StorageBackendInput::Local {
                    root: value.root.unwrap_or_default(),
                }
            }
            "s3" => StorageBackendInput::S3 {
                root: value.root,
                bucket: value.bucket.unwrap_or_default(),
                region: value.region.unwrap_or_default(),
                endpoint: value.endpoint,
                public_base_url: value.public_base_url.unwrap_or_default(),
                access_key: value.access_key,
                secret_key: value.secret_key,
                virtual_host_style: value.virtual_host_style,
            },
            _ => {
                return Err(StorageError::InvalidInput(
                    "driver must be either local or s3",
                ));
            }
        };
        Ok(Self {
            name: value.name,
            code: value.code,
            backend,
            enabled: value.enabled,
            sort: value.sort,
            description: value.description,
        })
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StorageStatusRequest {
    pub enabled: bool,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StorageResponse {
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

impl From<StorageView> for StorageResponse {
    fn from(value: StorageView) -> Self {
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
            has_access_key: value.has_access_key,
            has_secret_key: value.has_secret_key,
            enabled: value.enabled,
            is_default: value.is_default,
            sort: value.sort,
            description: value.description,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StorageListData {
    pub list: Vec<StorageResponse>,
}
