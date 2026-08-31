mod menus;
mod routes;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use menus::MenuIndex;
use routes::RouteIndex;
pub(crate) use routes::normalize_request_path;
use sqlx::{FromRow, PgPool};

use super::CatalogError;
use crate::IamInitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
enum MenuType {
    Directory,
    Page,
    Action,
}

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
struct AccessNode {
    id: i64,
    parent_id: Option<i64>,
    menu_type: MenuType,
    status: String,
    permission: Option<String>,
}

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
struct AccessBinding {
    menu_id: i64,
    method: String,
    path: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AccessCatalog {
    menus: MenuIndex,
    routes: RouteIndex,
}

impl AccessCatalog {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, IamInitError> {
        let nodes = sqlx::query_as::<_, AccessNode>(
            "select id, parent_id, menu_type, status, permission from sys_menus order by id",
        )
        .fetch_all(pool)
        .await?;
        let bindings = sqlx::query_as::<_, AccessBinding>(
            "select menu_id, method, path_pattern as path from sys_menu_apis order by method, path_pattern",
        )
        .fetch_all(pool)
        .await?;
        Ok(Self::from_parts(nodes, bindings)?)
    }

    fn from_parts(
        nodes: Vec<AccessNode>,
        bindings: Vec<AccessBinding>,
    ) -> Result<Self, CatalogError> {
        let menus = MenuIndex::from_nodes(nodes)?;
        let routes = RouteIndex::from_bindings(menus.active_route_bindings(bindings)?)?;
        Ok(Self { menus, routes })
    }

    /// Requires an uppercase HTTP method and a path normalized by
    /// [`normalize_request_path`].
    pub(crate) fn required_permission(
        &self,
        normalized_method: &str,
        normalized_path: &str,
    ) -> Result<&str, CatalogError> {
        self.routes
            .required_permission(normalized_method, normalized_path)
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
