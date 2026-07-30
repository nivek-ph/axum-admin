use serde::Deserialize;
use sqlx::FromRow;

#[derive(Debug, Clone)]
pub struct MenuMeta {
    pub active_name: String,
    pub keep_alive: bool,
    pub default_menu: bool,
    pub title: String,
    pub icon: String,
    pub close_tab: bool,
    pub transition_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuParameter {
    pub id: i64,
    #[serde(rename = "sysBaseMenuId")]
    pub sys_base_menu_id: i64,
    #[serde(rename = "type")]
    pub parameter_type: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuButton {
    pub id: i64,
    pub name: String,
    pub desc: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct ApiBinding {
    pub menu_id: i64,
    pub method: String,
    pub path_pattern: String,
}

#[derive(Debug, Clone)]
pub struct MenuView {
    pub id: i64,
    pub parent_id: i64,
    pub path: String,
    pub name: String,
    pub hidden: bool,
    pub component: String,
    pub sort: i32,
    pub meta: MenuMeta,
    pub parameters: Vec<MenuParameter>,
    pub menu_btn: Vec<MenuButton>,
    pub menu_type: String,
    pub status: String,
    pub permission: Option<String>,
    pub api_bindings: Vec<ApiBinding>,
    pub children: Vec<MenuView>,
}

#[derive(Debug, Clone, FromRow)]
pub(super) struct MenuRecord {
    pub(super) id: i64,
    pub(super) parent_id: i64,
    pub(super) path: String,
    pub(super) name: String,
    pub(super) hidden: bool,
    pub(super) component: String,
    pub(super) sort: i32,
    pub(super) active_name: String,
    pub(super) keep_alive: bool,
    pub(super) default_menu: bool,
    pub(super) title: String,
    pub(super) icon: String,
    pub(super) close_tab: bool,
    pub(super) transition_type: String,
    pub(super) parameters: serde_json::Value,
    pub(super) menu_btn: serde_json::Value,
    pub(super) menu_type: String,
    pub(super) status: String,
    pub(super) permission: Option<String>,
    #[sqlx(skip)]
    pub(super) api_bindings: Vec<ApiBinding>,
}
