mod catalog;
mod error;
mod scope;
mod service;

#[cfg(test)]
pub(crate) use catalog::AccessNode;
pub(crate) use catalog::{AccessCatalog, CatalogError};
pub use error::AccessEvaluationError;
pub use scope::ResolvedDataScope;
pub(crate) use scope::resolve_user_data_scope;
pub use service::{AccessContext, AccessService};
