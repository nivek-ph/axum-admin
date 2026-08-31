mod menus;
mod routes;
mod source;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use menus::MenuIndex;
use routes::RouteIndex;
pub(crate) use routes::normalize_request_path;
use source::{AccessBinding, AccessNode};
use sqlx::PgPool;

use super::CatalogError;
use crate::IamInitError;

#[derive(Debug, Clone)]
pub(crate) struct AccessCatalog {
    menus: MenuIndex,
    routes: RouteIndex,
}

impl AccessCatalog {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, IamInitError> {
        let source = source::load(pool).await?;
        Ok(Self::from_parts(source.nodes, source.bindings)?)
    }

    fn from_parts(
        nodes: Vec<AccessNode>,
        bindings: Vec<AccessBinding>,
    ) -> Result<Self, CatalogError> {
        let menus = MenuIndex::from_nodes(nodes)?;
        let routes = RouteIndex::from_bindings(menus.active_route_bindings(bindings)?)?;
        Ok(Self { menus, routes })
    }

    pub(crate) fn required_permission(
        &self,
        method: &str,
        path: &str,
    ) -> Result<&str, CatalogError> {
        self.routes.required_permission(method, path)
    }

    pub(crate) fn enabled_permissions(&self) -> &HashSet<String> {
        self.menus.enabled_permissions()
    }

    pub(crate) fn normalize_role_access(
        &self,
        permissions: impl IntoIterator<Item = String>,
    ) -> Result<BTreeSet<String>, CatalogError> {
        self.menus.normalize_role_access(permissions)
    }

    pub(crate) fn navigation_menu_ids(&self, permissions: &BTreeSet<String>) -> BTreeSet<i64> {
        self.menus.navigation_menu_ids(permissions)
    }
}
