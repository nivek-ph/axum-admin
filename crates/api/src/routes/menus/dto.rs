use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Serialize, ToSchema)]
pub struct MenuParameterResponse {
    pub id: i64,
    #[serde(rename = "sysBaseMenuId")]
    pub sys_base_menu_id: i64,
    #[serde(rename = "type")]
    pub parameter_type: String,
    pub key: String,
    pub value: String,
}

impl From<iam::menus::MenuParameter> for MenuParameterResponse {
    fn from(value: iam::menus::MenuParameter) -> Self {
        Self {
            id: value.id,
            sys_base_menu_id: value.sys_base_menu_id,
            parameter_type: value.parameter_type,
            key: value.key,
            value: value.value,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MenuButtonResponse {
    pub id: i64,
    pub name: String,
    pub desc: String,
}

impl From<iam::menus::MenuButton> for MenuButtonResponse {
    fn from(value: iam::menus::MenuButton) -> Self {
        Self {
            id: value.id,
            name: value.name,
            desc: value.desc,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApiBindingResponse {
    pub method: String,
    #[serde(rename = "pathPattern")]
    pub path_pattern: String,
}

impl From<iam::menus::ApiBinding> for ApiBindingResponse {
    fn from(value: iam::menus::ApiBinding) -> Self {
        Self {
            method: value.method,
            path_pattern: value.path_pattern,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
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
    pub parameters: Vec<MenuParameterResponse>,
    #[serde(rename = "menuBtn")]
    pub menu_btn: Vec<MenuButtonResponse>,
    #[serde(rename = "menuType")]
    pub menu_type: String,
    pub status: String,
    pub permission: Option<String>,
    #[serde(rename = "apiBindings")]
    pub api_bindings: Vec<ApiBindingResponse>,
    #[schema(no_recursion)]
    pub children: Vec<MenuPayload>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MenuData {
    pub menus: Vec<MenuPayload>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MenuTreeData {
    pub menus: Vec<MenuPayload>,
}

impl From<iam::menus::MenuView> for MenuPayload {
    fn from(v: iam::menus::MenuView) -> Self {
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
            parameters: v.parameters.into_iter().map(Into::into).collect(),
            menu_btn: v.menu_btn.into_iter().map(Into::into).collect(),
            menu_type: v.menu_type,
            status: v.status,
            permission: v.permission,
            api_bindings: v.api_bindings.into_iter().map(Into::into).collect(),
            children: v.children.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_payload_keeps_nested_transport_shape() {
        let payload = MenuPayload::from(iam::menus::MenuView {
            id: 1,
            parent_id: 0,
            path: "/dashboard".to_string(),
            name: "Dashboard".to_string(),
            hidden: false,
            component: "Dashboard".to_string(),
            sort: 1,
            meta: iam::menus::MenuMeta {
                active_name: String::new(),
                keep_alive: true,
                default_menu: true,
                title: "Dashboard".to_string(),
                icon: "dashboard".to_string(),
                close_tab: false,
                transition_type: String::new(),
            },
            parameters: vec![iam::menus::MenuParameter {
                id: 2,
                sys_base_menu_id: 1,
                parameter_type: "query".to_string(),
                key: "tab".to_string(),
                value: "main".to_string(),
            }],
            menu_btn: vec![iam::menus::MenuButton {
                id: 3,
                name: "refresh".to_string(),
                desc: "Refresh".to_string(),
            }],
            menu_type: "menu".to_string(),
            status: "active".to_string(),
            permission: Some("dashboard:view".to_string()),
            api_bindings: vec![iam::menus::ApiBinding {
                menu_id: 1,
                method: "GET".to_string(),
                path_pattern: "/api/dashboard".to_string(),
            }],
            children: Vec::new(),
        });

        let value = serde_json::to_value(payload).expect("menu payload should serialize");
        assert_eq!(value["parameters"][0]["sysBaseMenuId"], 1);
        assert_eq!(value["parameters"][0]["type"], "query");
        assert_eq!(value["menuBtn"][0]["name"], "refresh");
        assert_eq!(value["apiBindings"][0]["pathPattern"], "/api/dashboard");
        assert!(value["apiBindings"][0].get("menu_id").is_none());
    }
}
