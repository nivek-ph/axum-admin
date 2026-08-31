use std::collections::HashMap;

use matchit::Router;

use super::CatalogError;

const API_PREFIX: &str = "/api";

#[derive(Debug, Clone)]
pub(super) struct RouteIndex {
    by_method: HashMap<String, Router<String>>,
}

pub(super) struct RouteBinding {
    pub method: String,
    pub path: String,
    pub permission: String,
}

impl RouteIndex {
    pub(super) fn from_bindings(bindings: Vec<RouteBinding>) -> Result<Self, CatalogError> {
        let mut by_method = HashMap::<String, Router<String>>::new();

        for binding in bindings {
            let method = normalize_method(&binding.method)?;
            let path = normalize_path(&binding.path)?;
            by_method
                .entry(method)
                .or_default()
                .insert(path, binding.permission)?;
        }

        Ok(Self { by_method })
    }

    pub(super) fn required_permission(
        &self,
        normalized_method: &str,
        normalized_path: &str,
    ) -> Result<&str, CatalogError> {
        self.by_method
            .get(normalized_method)
            .and_then(|router| router.at(normalized_path).ok())
            .map(|matched| matched.value.as_str())
            .ok_or(CatalogError::Unbound)
    }
}

fn normalize_method(method: &str) -> Result<String, CatalogError> {
    let method = method.trim().to_ascii_uppercase();
    if method.is_empty() || !method.chars().all(|ch| ch.is_ascii_uppercase()) {
        return Err(CatalogError::InvalidBinding);
    }
    Ok(method)
}

/// Force a request path into the Catalog `/api` boundary and strip trailing slashes.
pub(crate) fn normalize_request_path(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches('/');
    let normalized = if trimmed.is_empty() {
        API_PREFIX
    } else {
        trimmed
    };
    if is_under_api_boundary(normalized) {
        normalized.to_string()
    } else if normalized.starts_with('/') {
        format!("{API_PREFIX}{normalized}")
    } else {
        format!("{API_PREFIX}/{normalized}")
    }
}

fn normalize_path(path: &str) -> Result<String, CatalogError> {
    let path = path.trim();
    if path.contains('?') || path.contains('#') || !is_under_api_boundary(path) {
        return Err(CatalogError::InvalidBinding);
    }
    Ok(normalize_request_path(path))
}

fn is_under_api_boundary(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/');
    trimmed == API_PREFIX
        || path
            .strip_prefix(API_PREFIX)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::{normalize_path, normalize_request_path};
    use crate::access::CatalogError;

    #[test]
    fn request_path_normalization_keeps_the_api_boundary() {
        assert_eq!(normalize_request_path("users"), "/api/users");
        assert_eq!(normalize_request_path("/users"), "/api/users");
        assert_eq!(normalize_request_path("/api/users/"), "/api/users");
        assert_eq!(normalize_request_path("/api"), "/api");
        assert_eq!(normalize_request_path("/api/"), "/api");
    }

    #[test]
    fn binding_paths_must_already_be_under_api() {
        assert_eq!(normalize_path("/api/users/"), Ok("/api/users".to_string()));
        assert!(matches!(
            normalize_path("/users"),
            Err(CatalogError::InvalidBinding)
        ));
        assert!(matches!(
            normalize_path("/api/users?x=1"),
            Err(CatalogError::InvalidBinding)
        ));
    }
}
