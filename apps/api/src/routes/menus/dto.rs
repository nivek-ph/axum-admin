use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct MenuMeta {
    #[serde(rename = "activeName")]
    pub active_name: String,
    #[serde(rename = "keepAlive")]
    pub keep_alive: bool,
    #[serde(rename = "defaultMenu")]
    pub default_menu: bool,
    pub title: String,
    pub icon: String,
    #[serde(rename = "closeTab")]
    pub close_tab: bool,
    #[serde(rename = "transitionType")]
    pub transition_type: String,
}

#[derive(Debug, Serialize)]
pub struct MenuPayload {
    pub id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    pub path: String,
    pub name: String,
    pub hidden: bool,
    pub component: String,
    pub sort: i32,
    pub meta: MenuMeta,
    pub parameters: Vec<menu::MenuParameter>,
    #[serde(rename = "menuBtn")]
    pub menu_btn: Vec<menu::MenuButton>,
    #[serde(rename = "menuType")]
    pub menu_type: String,
    pub status: String,
    pub permission: Option<String>,
    #[serde(rename = "apiBindings")]
    pub api_bindings: Vec<menu::ApiBinding>,
    pub children: Vec<MenuPayload>,
}

impl From<menu::MenuView> for MenuPayload {
    fn from(v: menu::MenuView) -> Self {
        Self {
            id: v.id,
            parent_id: v.parent_id,
            path: v.path,
            name: v.name,
            hidden: v.hidden,
            component: v.component,
            sort: v.sort,
            meta: MenuMeta {
                active_name: v.meta.active_name,
                keep_alive: v.meta.keep_alive,
                default_menu: v.meta.default_menu,
                title: v.meta.title,
                icon: v.meta.icon,
                close_tab: v.meta.close_tab,
                transition_type: v.meta.transition_type,
            },
            parameters: v.parameters,
            menu_btn: v.menu_btn,
            menu_type: v.menu_type,
            status: v.status,
            permission: v.permission,
            api_bindings: v.api_bindings,
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}
