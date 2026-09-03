mod menus;
#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use menus::MenuIndex;
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

#[derive(Debug, Clone)]
pub(crate) struct AccessCatalog {
    menus: MenuIndex,
}

impl AccessCatalog {
    pub(crate) async fn load(pool: &PgPool) -> Result<Self, IamInitError> {
        let nodes = sqlx::query_as::<_, AccessNode>(
            "select id, parent_id, menu_type, status, permission from sys_menus order by id",
        )
        .fetch_all(pool)
        .await?;
        Ok(Self::from_nodes(nodes)?)
    }

    fn from_nodes(nodes: Vec<AccessNode>) -> Result<Self, CatalogError> {
        let menus = MenuIndex::from_nodes(nodes)?;
        Ok(Self { menus })
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
