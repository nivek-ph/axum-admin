use file_storage::storages::{StorageBackendInput, StorageInput, StorageQuery, StorageView};
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
    #[serde(flatten)]
    pub backend: StorageBackendRequest,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub sort: i32,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(
    tag = "driver",
    rename_all = "lowercase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum StorageBackendRequest {
    Local {
        #[serde(default)]
        root: String,
    },
    S3 {
        root: Option<String>,
        #[serde(default)]
        bucket: String,
        #[serde(default)]
        region: String,
        endpoint: Option<String>,
        #[serde(default)]
        public_base_url: String,
        access_key: Option<String>,
        secret_key: Option<String>,
        #[serde(default)]
        virtual_host_style: bool,
    },
}

fn enabled_by_default() -> bool {
    true
}

impl From<StorageRequest> for StorageInput {
    fn from(value: StorageRequest) -> Self {
        let backend = match value.backend {
            StorageBackendRequest::Local { root } => StorageBackendInput::Local { root },
            StorageBackendRequest::S3 {
                root,
                bucket,
                region,
                endpoint,
                public_base_url,
                access_key,
                secret_key,
                virtual_host_style,
            } => StorageBackendInput::S3 {
                root,
                bucket,
                region,
                endpoint,
                public_base_url,
                access_key,
                secret_key,
                virtual_host_style,
            },
        };
        Self {
            name: value.name,
            code: value.code,
            backend,
            enabled: value.enabled,
            sort: value.sort,
            description: value.description,
        }
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

#[cfg(test)]
mod tests {
    use super::{StorageBackendRequest, StorageRequest};

    #[test]
    fn local_request_rejects_s3_only_fields() {
        let request = serde_json::from_value::<StorageRequest>(serde_json::json!({
            "name": "Local",
            "code": "local",
            "driver": "local",
            "root": "./uploads",
            "bucket": "unexpected"
        }));

        assert!(request.is_err());
    }

    #[test]
    fn s3_request_deserializes_into_the_s3_variant() {
        let request = serde_json::from_value::<StorageRequest>(serde_json::json!({
            "name": "Objects",
            "code": "objects",
            "driver": "s3",
            "bucket": "files",
            "region": "us-east-1",
            "publicBaseUrl": "https://cdn.example.test"
        }))
        .expect("valid S3 request should deserialize");

        assert!(matches!(request.backend, StorageBackendRequest::S3 { .. }));
    }
}
