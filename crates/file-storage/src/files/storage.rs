use std::path::PathBuf;

use opendal::{Operator, services};

const LOCAL_URL_PREFIX: &str = "/uploads";

#[derive(Debug, Clone)]
pub struct FileStorage {
    pub driver: String,
    pub local_root: String,
    pub s3_bucket: String,
    pub s3_region: String,
    pub s3_endpoint: String,
    pub s3_prefix: String,
    pub s3_public_base_url: String,
    pub s3_access_key_id: String,
    pub s3_secret_access_key: String,
    pub s3_virtual_host_style: bool,
}

impl Default for FileStorage {
    fn default() -> Self {
        Self {
            driver: "local".to_string(),
            local_root: "./uploads".to_string(),
            s3_bucket: String::new(),
            s3_region: String::new(),
            s3_endpoint: String::new(),
            s3_prefix: String::new(),
            s3_public_base_url: String::new(),
            s3_access_key_id: String::new(),
            s3_secret_access_key: String::new(),
            s3_virtual_host_style: false,
        }
    }
}

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
    pub(crate) fn from_config(config: &FileStorage) -> Result<Self, FileStorageError> {
        match config.driver.trim().to_ascii_lowercase().as_str() {
            "local" => Self::local(&config.local_root),
            "s3" => Self::s3(config),
            _ => Err(FileStorageError::UnsupportedDriver(config.driver.clone())),
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

    fn s3(config: &FileStorage) -> Result<Self, FileStorageError> {
        let bucket = required(&config.s3_bucket, "bucket", "s3")?;
        let region = required(&config.s3_region, "region", "s3")?;
        let public_base_url = required(&config.s3_public_base_url, "public_base_url", "s3")?;
        if !public_base_url.starts_with("http://") && !public_base_url.starts_with("https://") {
            return Err(FileStorageError::InvalidPublicBaseUrl);
        }
        let has_access_key = !config.s3_access_key_id.trim().is_empty();
        let has_secret_key = !config.s3_secret_access_key.trim().is_empty();
        if has_access_key != has_secret_key {
            return Err(FileStorageError::IncompleteCredentials);
        }

        let mut builder = services::S3::default().bucket(bucket).region(region);
        if let Some(value) = optional(&config.s3_endpoint) {
            builder = builder.endpoint(value);
        }
        if let Some(value) = optional(&config.s3_prefix) {
            builder = builder.root(&format!("/{}", value.trim_matches('/')));
        }
        if has_access_key {
            builder = builder
                .access_key_id(config.s3_access_key_id.trim())
                .secret_access_key(config.s3_secret_access_key.trim());
        }
        if config.s3_virtual_host_style {
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
    use super::{FileObjectStorage, FileStorage, FileStorageError};

    #[test]
    fn unsupported_driver_is_rejected() {
        let config = FileStorage {
            driver: "azure".to_string(),
            ..FileStorage::default()
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("unsupported drivers should fail during startup");
        assert!(matches!(error, FileStorageError::UnsupportedDriver(_)));
    }

    #[test]
    fn s3_requires_bucket_and_public_url() {
        let config = FileStorage {
            driver: "s3".to_string(),
            ..FileStorage::default()
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("missing S3 settings should fail during startup");
        assert!(matches!(error, FileStorageError::Missing("bucket", "s3")));
    }

    #[test]
    fn s3_credentials_must_be_a_complete_pair() {
        let config = FileStorage {
            driver: "s3".to_string(),
            s3_bucket: "files".to_string(),
            s3_region: "us-east-1".to_string(),
            s3_public_base_url: "https://files.example.test".to_string(),
            s3_access_key_id: "access-key".to_string(),
            ..FileStorage::default()
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("partial S3 credentials should fail during startup");
        assert!(matches!(error, FileStorageError::IncompleteCredentials));
    }

    #[test]
    fn s3_requires_an_http_public_url() {
        let config = FileStorage {
            driver: "s3".to_string(),
            s3_bucket: "files".to_string(),
            s3_region: "us-east-1".to_string(),
            s3_public_base_url: "cdn.example.test".to_string(),
            ..FileStorage::default()
        };

        let error = FileObjectStorage::from_config(&config)
            .expect_err("invalid S3 public URLs should fail during startup");
        assert!(matches!(error, FileStorageError::InvalidPublicBaseUrl));
    }

    #[test]
    fn s3_public_urls_are_derived_from_the_configured_base() {
        let config = FileStorage {
            driver: "s3".to_string(),
            s3_bucket: "files".to_string(),
            s3_region: "us-east-1".to_string(),
            s3_public_base_url: "https://cdn.example.test/assets/".to_string(),
            ..FileStorage::default()
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
