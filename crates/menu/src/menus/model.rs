use serde::{Deserialize, Serialize};
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuParameter {
    pub id: i64,
    #[serde(rename = "sysBaseMenuId")]
    pub sys_base_menu_id: i64,
    #[serde(rename = "type")]
    pub parameter_type: String,
    pub key: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuButton {
    pub id: i64,
    pub name: String,
    pub desc: String,
}
#[derive(Debug, Clone, Serialize, FromRow)]
pub struct ApiBinding {
    #[serde(skip)]
    pub menu_id: i64,
    pub method: String,
    #[serde(rename = "pathPattern")]
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
pub struct MenuRecord {
    pub id: i64,
    pub parent_id: i64,
    pub path: String,
    pub name: String,
    pub hidden: bool,
    pub component: String,
    pub sort: i32,
    pub active_name: String,
    pub keep_alive: bool,
    pub default_menu: bool,
    pub title: String,
    pub icon: String,
    pub close_tab: bool,
    pub transition_type: String,
    pub parameters: serde_json::Value,
    pub menu_btn: serde_json::Value,
    pub menu_type: String,
    pub status: String,
    pub permission: Option<String>,
    #[sqlx(skip)]
    pub api_bindings: Vec<ApiBinding>,
}
