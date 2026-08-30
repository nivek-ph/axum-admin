mod backend;
mod error;
mod model;
mod service;

pub use backend::ObjectStorageError;
pub(crate) use backend::StorageBackend;
pub use error::StorageError;
pub(crate) use model::{S3Credentials, StorageBackendConfig};
pub use model::{
    StorageBackendInput, StorageBackendView, StorageDriver, StorageInput, StorageQuery, StorageView,
};
pub use service::StorageService;
