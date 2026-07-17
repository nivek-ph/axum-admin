mod catalog;
mod error;
mod scope;
mod service;

pub(crate) use catalog::CatalogError;
#[cfg(test)]
pub(crate) use catalog::{AccessCatalog, AccessNode};
pub use error::{AccessEvaluationError, AccessInitError, AccessPropagationError};
pub use scope::DataScopeFilter;
pub(crate) use scope::resolve_user_data_scope;
pub use service::{AccessService, AccessSnapshot};
