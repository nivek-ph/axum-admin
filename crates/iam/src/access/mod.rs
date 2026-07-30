mod catalog;
mod scope;
mod service;

pub(crate) use catalog::{AccessCatalog, CatalogError};
pub use scope::ResolvedDataScope;
pub(crate) use scope::resolve_user_data_scope;
pub use service::{AccessContext, AccessEvaluationError, AccessService};

#[cfg(test)]
pub(crate) mod tests {
    pub(crate) use super::catalog::AccessNode;
}
