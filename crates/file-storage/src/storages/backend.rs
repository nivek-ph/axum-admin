use std::path::PathBuf;

use opendal::{Operator, Writer, services};

use super::model::{S3Credentials, StorageBackendConfig};

const LOCAL_URL_PREFIX: &str = "/uploads";

#[derive(Debug, thiserror::Error)]
pub enum ObjectStorageError {
    #[error("unsupported storage driver `{0}`; expected `local` or `s3`")]
    UnsupportedDriver(String),
    #[error("{0} is required when driver={1}")]
    Missing(&'static str, &'static str),
    #[error("access_key and secret_key must be configured together")]
    IncompleteCredentials,
    #[error("access_key and secret_key are required when driver=s3")]
    MissingCredentials,
    #[error("public_base_url must be an http:// or https:// URL")]
    InvalidPublicBaseUrl,
    #[error("local file storage root could not be prepared: {0}")]
    LocalRoot(#[source] std::io::Error),
    #[error("file storage adapter could not be initialized: {0}")]
    Adapter(#[from] opendal::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct StorageBackend {
    pub(crate) operator: Operator,
    url_base: String,
    local_root: Option<PathBuf>,
}

impl StorageBackend {
    pub(super) fn validate_config(config: &StorageBackendConfig) -> Result<(), ObjectStorageError> {
        match config {
            StorageBackendConfig::Local { root } => {
                required(root, "root", "local")?;
                Ok(())
            }
            StorageBackendConfig::S3 {
                root,
                bucket,
                region,
                endpoint,
                public_base_url,
                credentials,
                virtual_host_style,
            } => Self::s3(
                root.as_deref(),
                bucket,
                region,
                endpoint.as_deref(),
                public_base_url,
                credentials,
                *virtual_host_style,
            )
            .map(drop),
        }
    }

    pub(super) fn from_config(config: &StorageBackendConfig) -> Result<Self, ObjectStorageError> {
        match config {
            StorageBackendConfig::Local { root } => Self::local(root),
            StorageBackendConfig::S3 {
                root,
                bucket,
                region,
                endpoint,
                public_base_url,
                credentials,
                virtual_host_style,
            } => Self::s3(
                root.as_deref(),
                bucket,
                region,
                endpoint.as_deref(),
                public_base_url,
                credentials,
                *virtual_host_style,
            ),
        }
    }

    pub(super) fn local(root: &str) -> Result<Self, ObjectStorageError> {
        let root = root.trim();
        if root.is_empty() {
            return Err(ObjectStorageError::Missing("root", "local"));
        }
        let root = absolute_path(root).map_err(ObjectStorageError::LocalRoot)?;
        std::fs::create_dir_all(&root).map_err(ObjectStorageError::LocalRoot)?;
        let root_string = root.to_string_lossy();
        let operator = Operator::new(services::Fs::default().root(&root_string))?;
        Ok(Self {
            operator,
            url_base: LOCAL_URL_PREFIX.to_string(),
            local_root: Some(root),
        })
    }

    fn s3(
        root: Option<&str>,
        bucket: &str,
        region: &str,
        endpoint: Option<&str>,
        public_base_url: &str,
        credentials: &S3Credentials,
        virtual_host_style: bool,
    ) -> Result<Self, ObjectStorageError> {
        let bucket = required(bucket, "bucket", "s3")?;
        let region = required(region, "region", "s3")?;
        let public_base_url = required(public_base_url, "public_base_url", "s3")?;
        if !public_base_url.starts_with("http://") && !public_base_url.starts_with("https://") {
            return Err(ObjectStorageError::InvalidPublicBaseUrl);
        }
        let mut builder = services::S3::default()
            .bucket(bucket)
            .region(region)
            .access_key_id(&credentials.access_key)
            .secret_access_key(&credentials.secret_key)
            .disable_config_load()
            .disable_ec2_metadata();
        if let Some(value) = endpoint.and_then(optional) {
            builder = builder.endpoint(value);
        }
        if let Some(value) = root.and_then(optional) {
            builder = builder.root(&format!("/{}", value.trim_matches('/')));
        }
        if virtual_host_style {
            builder = builder.enable_virtual_host_style();
        }

        Ok(Self {
            operator: Operator::new(builder)?,
            url_base: public_base_url.trim_end_matches('/').to_string(),
            local_root: None,
        })
    }

    pub(crate) fn public_url(&self, object: &str) -> String {
        format!("{}/{object}", self.url_base)
    }

    pub(crate) async fn writer(&self, object: &str) -> Result<Writer, opendal::Error> {
        let content_type = mime_guess::from_path(object).first_or_octet_stream();
        self.operator
            .writer_with(object)
            .content_type(content_type.as_ref())
            .await
    }

    pub(crate) fn is_local(&self) -> bool {
        self.local_root.is_some()
    }
}

fn required<'a>(
    value: &'a str,
    name: &'static str,
    driver: &'static str,
) -> Result<&'a str, ObjectStorageError> {
    optional(value).ok_or(ObjectStorageError::Missing(name, driver))
}

fn optional(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()).then_some(value)
}

fn absolute_path(path: &str) -> Result<PathBuf, std::io::Error> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use opendal::{Operator, services};

    use super::{ObjectStorageError, StorageBackend};
    use crate::storages::model::{S3Credentials, StorageBackendConfig};

    fn credentials() -> S3Credentials {
        S3Credentials {
            access_key: "access-key".to_string(),
            secret_key: "secret-key".to_string(),
        }
    }

    #[test]
    fn unsupported_driver_is_rejected() {
        let error = "azure".parse::<crate::storages::StorageDriver>();
        assert!(error.is_err());
    }

    #[test]
    fn s3_requires_bucket_and_public_url() {
        let config = StorageBackendConfig::S3 {
            root: None,
            bucket: String::new(),
            region: String::new(),
            endpoint: None,
            public_base_url: String::new(),
            credentials: credentials(),
            virtual_host_style: false,
        };

        let error = StorageBackend::from_config(&config)
            .expect_err("missing S3 settings should fail during startup");
        assert!(matches!(error, ObjectStorageError::Missing("bucket", "s3")));
    }

    #[test]
    fn s3_requires_an_http_public_url() {
        let config = StorageBackendConfig::S3 {
            root: None,
            bucket: "files".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            public_base_url: "cdn.example.test".to_string(),
            credentials: credentials(),
            virtual_host_style: false,
        };

        let error = StorageBackend::from_config(&config)
            .expect_err("invalid S3 public URLs should fail during startup");
        assert!(matches!(error, ObjectStorageError::InvalidPublicBaseUrl));
    }

    #[test]
    fn s3_public_urls_are_derived_from_the_configured_base() {
        let config = StorageBackendConfig::S3 {
            root: None,
            bucket: "files".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            public_base_url: "https://cdn.example.test/assets/".to_string(),
            credentials: credentials(),
            virtual_host_style: false,
        };
        let storage = StorageBackend::from_config(&config)
            .expect("valid S3 configuration should construct an adapter");

        assert_eq!(
            storage.public_url("report.pdf"),
            "https://cdn.example.test/assets/report.pdf"
        );
    }

    #[tokio::test]
    async fn writer_sets_content_type_from_object_extension() {
        let storage = StorageBackend {
            operator: Operator::new(services::Memory::default())
                .expect("memory adapter should construct"),
            url_base: String::new(),
            local_root: None,
        };

        let mut writer = storage
            .writer("preview.png")
            .await
            .expect("writer should open");
        writer
            .write(b"png".to_vec())
            .await
            .expect("bytes should write");
        writer.close().await.expect("writer should close");

        let metadata = storage
            .operator
            .stat("preview.png")
            .await
            .expect("object metadata should be readable");
        assert_eq!(metadata.content_type(), Some("image/png"));
    }
}
