use std::path::PathBuf;

use opendal::{Operator, services};

use crate::storages::{S3Credentials, StorageBackendConfig};

const LOCAL_URL_PREFIX: &str = "/uploads";

#[derive(Debug, thiserror::Error)]
pub enum FileStorageError {
    #[error("unsupported storage driver `{0}`; expected `local` or `s3`")]
    UnsupportedDriver(String),
    #[error("{0} is required when driver={1}")]
    Missing(&'static str, &'static str),
    #[error("access_key and secret_key must be configured together")]
    IncompleteCredentials,
    #[error("public_base_url must be an http:// or https:// URL")]
    InvalidPublicBaseUrl,
    #[error("local file storage root could not be prepared: {0}")]
    LocalRoot(#[source] std::io::Error),
    #[error("file storage adapter could not be initialized: {0}")]
    Adapter(#[from] opendal::Error),
}

#[derive(Debug, Clone)]
pub(crate) struct FileObjectStorage {
    pub(crate) operator: Operator,
    url_base: String,
    local_root: Option<PathBuf>,
}

impl FileObjectStorage {
    pub(crate) fn from_config(config: &StorageBackendConfig) -> Result<Self, FileStorageError> {
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
                credentials.as_ref(),
                *virtual_host_style,
            ),
        }
    }

    pub(crate) fn local(root: &str) -> Result<Self, FileStorageError> {
        let root = root.trim();
        if root.is_empty() {
            return Err(FileStorageError::Missing("root", "local"));
        }
        let root = absolute_path(root).map_err(FileStorageError::LocalRoot)?;
        std::fs::create_dir_all(&root).map_err(FileStorageError::LocalRoot)?;
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
        credentials: Option<&S3Credentials>,
        virtual_host_style: bool,
    ) -> Result<Self, FileStorageError> {
        let bucket = required(bucket, "bucket", "s3")?;
        let region = required(region, "region", "s3")?;
        let public_base_url = required(public_base_url, "public_base_url", "s3")?;
        if !public_base_url.starts_with("http://") && !public_base_url.starts_with("https://") {
            return Err(FileStorageError::InvalidPublicBaseUrl);
        }
        let mut builder = services::S3::default().bucket(bucket).region(region);
        if let Some(value) = endpoint.and_then(optional) {
            builder = builder.endpoint(value);
        }
        if let Some(value) = root.and_then(optional) {
            builder = builder.root(&format!("/{}", value.trim_matches('/')));
        }
        if let Some(credentials) = credentials {
            builder = builder
                .access_key_id(&credentials.access_key)
                .secret_access_key(&credentials.secret_key);
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

    pub(crate) fn managed_object(&self, url: &str) -> Option<String> {
        let object = url.strip_prefix(&format!("{}/", self.url_base))?;
        (!object.is_empty() && !object.contains('/')).then(|| object.to_string())
    }

    pub(crate) fn is_local(&self) -> bool {
        self.local_root.is_some()
    }
}

fn required<'a>(
    value: &'a str,
    name: &'static str,
    driver: &'static str,
) -> Result<&'a str, FileStorageError> {
    optional(value).ok_or(FileStorageError::Missing(name, driver))
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
    use super::{FileObjectStorage, FileStorageError};
    use crate::storages::StorageBackendConfig;

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
            credentials: None,
            virtual_host_style: false,
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("missing S3 settings should fail during startup");
        assert!(matches!(error, FileStorageError::Missing("bucket", "s3")));
    }

    #[test]
    fn s3_requires_an_http_public_url() {
        let config = StorageBackendConfig::S3 {
            root: None,
            bucket: "files".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            public_base_url: "cdn.example.test".to_string(),
            credentials: None,
            virtual_host_style: false,
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("invalid S3 public URLs should fail during startup");
        assert!(matches!(error, FileStorageError::InvalidPublicBaseUrl));
    }

    #[test]
    fn s3_public_urls_are_derived_from_the_configured_base() {
        let config = StorageBackendConfig::S3 {
            root: None,
            bucket: "files".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            public_base_url: "https://cdn.example.test/assets/".to_string(),
            credentials: None,
            virtual_host_style: false,
        };
        let storage = FileObjectStorage::from_config(&config)
            .expect("valid S3 configuration should construct an adapter");

        assert_eq!(
            storage.public_url("report.pdf"),
            "https://cdn.example.test/assets/report.pdf"
        );
        assert_eq!(
            storage.managed_object("https://cdn.example.test/assets/report.pdf"),
            Some("report.pdf".to_string())
        );
    }
}
