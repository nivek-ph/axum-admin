use std::collections::HashMap;

use matchit::Router;

use super::{AccessBinding, CatalogError};

#[derive(Debug, Clone)]
pub(super) struct RouteIndex {
    by_method: HashMap<String, Router<i64>>,
}

impl RouteIndex {
    pub(super) fn from_bindings(bindings: Vec<AccessBinding>) -> Result<Self, CatalogError> {
        let mut by_method = HashMap::<String, Router<i64>>::new();

        for binding in bindings {
            let method = normalize_method(&binding.method)?;
            let path: String = normalize_path(&binding.path)?;
            by_method
                .entry(method)
                .or_default()
                .insert(path, binding.menu_id)?;
        }

        Ok(Self { by_method })
    }

    pub(super) fn resolve(&self, method: &str, path: &str) -> Result<i64, CatalogError> {
        let method = normalize_method(method)?;
        let path = normalize_path(path)?;
        self.by_method
            .get(&method)
            .and_then(|router| router.at(&path).ok())
            .map(|matched| *matched.value)
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

fn normalize_path(path: &str) -> Result<String, CatalogError> {
    let path = path.trim();
    if !path.starts_with("/api") || path.contains('?') || path.contains('#') {
        return Err(CatalogError::InvalidBinding);
    }
    let normalized = path.trim_end_matches('/');
    Ok(if normalized.is_empty() {
        "/api".to_string()
    } else {
        normalized.to_string()
    })
}
