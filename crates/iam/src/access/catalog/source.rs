use sqlx::{FromRow, PgPool};

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::EnumString, sqlx::Type)]
#[strum(serialize_all = "lowercase")]
#[sqlx(type_name = "text", rename_all = "lowercase")]
pub(super) enum MenuType {
    Directory,
    Page,
    Action,
}

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub(super) struct AccessNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub menu_type: MenuType,
    pub status: String,
    pub permission: Option<String>,
}

#[derive(Debug, Clone, FromRow, PartialEq, Eq)]
pub(super) struct AccessBinding {
    pub menu_id: i64,
    pub method: String,
    pub path: String,
}

pub(super) struct CatalogSource {
    pub nodes: Vec<AccessNode>,
    pub bindings: Vec<AccessBinding>,
}

pub(super) async fn load(pool: &PgPool) -> Result<CatalogSource, sqlx::Error> {
    let nodes = sqlx::query_as::<_, AccessNode>(
        "select id, parent_id, title, menu_type, status, permission from sys_menus order by id",
    )
    .fetch_all(pool)
    .await?;
    let bindings = sqlx::query_as::<_, AccessBinding>(
        "select menu_id, method, path_pattern as path from sys_menu_apis order by method, path_pattern",
    )
    .fetch_all(pool)
    .await?;
    Ok(CatalogSource { nodes, bindings })
}
