use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use admin_httpz::AppError;

use crate::errors;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuParameter {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "sysBaseMenuID")]
    pub sys_base_menu_id: i64,
    #[serde(rename = "type")]
    pub parameter_type: String,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuButton {
    #[serde(rename = "ID")]
    pub id: i64,
    pub name: String,
    pub desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuView {
    #[serde(rename = "ID")]
    pub id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    pub path: String,
    pub name: String,
    pub hidden: bool,
    pub component: String,
    pub sort: i32,
    pub meta: MenuMeta,
    pub parameters: Vec<MenuParameter>,
    #[serde(rename = "menuBtn")]
    pub menu_btn: Vec<MenuButton>,
    #[serde(rename = "menuType", default = "default_menu_type")]
    pub menu_type: String,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(rename = "permissionId", default)]
    pub permission_id: Option<i64>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(rename = "apiPath", default)]
    pub api_path: Option<String>,
    pub children: Vec<MenuView>,
}

fn default_menu_type() -> String {
    "page".to_string()
}

#[derive(Debug, Clone, Copy)]
struct DefaultAction {
    name: &'static str,
    title: &'static str,
    permission: &'static str,
    method: &'static str,
    api_path: &'static str,
    sort: i32,
}

pub fn default_menus() -> Vec<MenuView> {
    [
        (
            1,
            "dashboard",
            "Dashboard",
            "view/dashboard/index.vue",
            "odometer",
            Vec::new(),
        ),
        (
            2,
            "users",
            "Users",
            "view/users/index.vue",
            "user",
            vec![
                DefaultAction {
                    name: "users:list",
                    title: "List",
                    permission: "system:user:list",
                    method: "GET",
                    api_path: "/api/users",
                    sort: 10,
                },
                DefaultAction {
                    name: "users:create",
                    title: "Create",
                    permission: "system:user:create",
                    method: "POST",
                    api_path: "/api/users",
                    sort: 20,
                },
                DefaultAction {
                    name: "users:update",
                    title: "Update",
                    permission: "system:user:update",
                    method: "PUT",
                    api_path: "/api/users/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "users:delete",
                    title: "Delete",
                    permission: "system:user:delete",
                    method: "DELETE",
                    api_path: "/api/users/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "users:reset-password",
                    title: "Reset password",
                    permission: "system:user:reset-password",
                    method: "POST",
                    api_path: "/api/users/{id}/password/reset",
                    sort: 50,
                },
                DefaultAction {
                    name: "users:assign-roles",
                    title: "Assign roles",
                    permission: "system:user:assign-roles",
                    method: "PUT",
                    api_path: "/api/users/{id}/roles",
                    sort: 60,
                },
            ],
        ),
        (
            3,
            "roles",
            "Roles",
            "view/roles/index.vue",
            "shield",
            vec![
                DefaultAction {
                    name: "roles:list",
                    title: "List",
                    permission: "system:role:list",
                    method: "GET",
                    api_path: "/api/roles",
                    sort: 10,
                },
                DefaultAction {
                    name: "roles:create",
                    title: "Create",
                    permission: "system:role:create",
                    method: "POST",
                    api_path: "/api/roles",
                    sort: 20,
                },
                DefaultAction {
                    name: "roles:update",
                    title: "Update",
                    permission: "system:role:update",
                    method: "PUT",
                    api_path: "/api/roles/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "roles:delete",
                    title: "Delete",
                    permission: "system:role:delete",
                    method: "DELETE",
                    api_path: "/api/roles/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "roles:list-users",
                    title: "View members",
                    permission: "system:role:list-users",
                    method: "GET",
                    api_path: "/api/roles/{id}/users",
                    sort: 50,
                },
                DefaultAction {
                    name: "roles:assign-users",
                    title: "Assign users",
                    permission: "system:role:assign-users",
                    method: "PUT",
                    api_path: "/api/roles/{id}/users",
                    sort: 60,
                },
                DefaultAction {
                    name: "roles:update-permission",
                    title: "Save permissions",
                    permission: "system:role:update-permission",
                    method: "PUT",
                    api_path: "/api/roles/{id}/permissions",
                    sort: 70,
                },
            ],
        ),
        (
            4,
            "menus",
            "Menus",
            "view/menus/index.vue",
            "menu",
            vec![
                DefaultAction {
                    name: "menus:list",
                    title: "List",
                    permission: "system:menu:list",
                    method: "GET",
                    api_path: "/api/menus",
                    sort: 10,
                },
                DefaultAction {
                    name: "menus:create",
                    title: "Create",
                    permission: "system:menu:create",
                    method: "POST",
                    api_path: "/api/menus",
                    sort: 20,
                },
                DefaultAction {
                    name: "menus:update",
                    title: "Update",
                    permission: "system:menu:update",
                    method: "PUT",
                    api_path: "/api/menus/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "menus:delete",
                    title: "Delete",
                    permission: "system:menu:delete",
                    method: "DELETE",
                    api_path: "/api/menus/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "menus:tree",
                    title: "Tree",
                    permission: "system:menu:tree",
                    method: "GET",
                    api_path: "/api/menus/tree",
                    sort: 50,
                },
                DefaultAction {
                    name: "menus:list-roles",
                    title: "View menu roles",
                    permission: "system:menu:list-roles",
                    method: "GET",
                    api_path: "/api/menus/{id}/roles",
                    sort: 60,
                },
                DefaultAction {
                    name: "menus:assign-roles",
                    title: "Assign menu roles",
                    permission: "system:menu:assign-roles",
                    method: "PUT",
                    api_path: "/api/menus/{id}/roles",
                    sort: 70,
                },
            ],
        ),
        (
            5,
            "apis",
            "API directory",
            "view/apis/index.vue",
            "route",
            vec![
                DefaultAction {
                    name: "apis:list",
                    title: "List",
                    permission: "system:api:list",
                    method: "GET",
                    api_path: "/api/routes",
                    sort: 10,
                },
                DefaultAction {
                    name: "apis:create",
                    title: "Create",
                    permission: "system:api:create",
                    method: "POST",
                    api_path: "/api/routes",
                    sort: 20,
                },
                DefaultAction {
                    name: "apis:update",
                    title: "Update",
                    permission: "system:api:update",
                    method: "PUT",
                    api_path: "/api/routes/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "apis:delete",
                    title: "Delete",
                    permission: "system:api:delete",
                    method: "DELETE",
                    api_path: "/api/routes/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "apis:list-roles",
                    title: "View roles",
                    permission: "system:api:list-roles",
                    method: "GET",
                    api_path: "/api/routes/roles",
                    sort: 50,
                },
                DefaultAction {
                    name: "apis:assign-roles",
                    title: "Assign roles",
                    permission: "system:api:assign-roles",
                    method: "PUT",
                    api_path: "/api/routes/roles",
                    sort: 60,
                },
                DefaultAction {
                    name: "apis:batch-delete",
                    title: "Batch delete",
                    permission: "system:api:batch-delete",
                    method: "DELETE",
                    api_path: "/api/routes/batch",
                    sort: 70,
                },
                DefaultAction {
                    name: "apis:list-all",
                    title: "List all",
                    permission: "system:api:list-all",
                    method: "GET",
                    api_path: "/api/routes/all",
                    sort: 80,
                },
                DefaultAction {
                    name: "apis:groups",
                    title: "Groups",
                    permission: "system:api:groups",
                    method: "GET",
                    api_path: "/api/routes/groups",
                    sort: 90,
                },
            ],
        ),
        (
            6,
            "params",
            "Params",
            "view/params/index.vue",
            "sliders",
            vec![
                DefaultAction {
                    name: "params:list",
                    title: "List",
                    permission: "system:param:list",
                    method: "GET",
                    api_path: "/api/params",
                    sort: 10,
                },
                DefaultAction {
                    name: "params:create",
                    title: "Create",
                    permission: "system:param:create",
                    method: "POST",
                    api_path: "/api/params",
                    sort: 20,
                },
                DefaultAction {
                    name: "params:get-by-key",
                    title: "Get by key",
                    permission: "system:param:get-by-key",
                    method: "GET",
                    api_path: "/api/params/by-key",
                    sort: 30,
                },
                DefaultAction {
                    name: "params:batch-delete",
                    title: "Batch delete",
                    permission: "system:param:batch-delete",
                    method: "DELETE",
                    api_path: "/api/params/batch",
                    sort: 40,
                },
                DefaultAction {
                    name: "params:get",
                    title: "Get",
                    permission: "system:param:get",
                    method: "GET",
                    api_path: "/api/params/{id}",
                    sort: 50,
                },
                DefaultAction {
                    name: "params:update",
                    title: "Update",
                    permission: "system:param:update",
                    method: "PUT",
                    api_path: "/api/params/{id}",
                    sort: 60,
                },
                DefaultAction {
                    name: "params:delete",
                    title: "Delete",
                    permission: "system:param:delete",
                    method: "DELETE",
                    api_path: "/api/params/{id}",
                    sort: 70,
                },
            ],
        ),
        (
            7,
            "dictionaries",
            "Dictionaries",
            "view/dictionaries/index.vue",
            "book",
            vec![
                DefaultAction {
                    name: "dictionaries:list",
                    title: "List",
                    permission: "system:dictionary:list",
                    method: "GET",
                    api_path: "/api/dictionaries",
                    sort: 10,
                },
                DefaultAction {
                    name: "dictionaries:create",
                    title: "Create",
                    permission: "system:dictionary:create",
                    method: "POST",
                    api_path: "/api/dictionaries",
                    sort: 20,
                },
                DefaultAction {
                    name: "dictionaries:import",
                    title: "Import",
                    permission: "system:dictionary:import",
                    method: "POST",
                    api_path: "/api/dictionaries/import",
                    sort: 30,
                },
                DefaultAction {
                    name: "dictionaries:get",
                    title: "Get",
                    permission: "system:dictionary:get",
                    method: "GET",
                    api_path: "/api/dictionaries/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "dictionaries:update",
                    title: "Update",
                    permission: "system:dictionary:update",
                    method: "PUT",
                    api_path: "/api/dictionaries/{id}",
                    sort: 50,
                },
                DefaultAction {
                    name: "dictionaries:delete",
                    title: "Delete",
                    permission: "system:dictionary:delete",
                    method: "DELETE",
                    api_path: "/api/dictionaries/{id}",
                    sort: 60,
                },
                DefaultAction {
                    name: "dictionaries:export",
                    title: "Export",
                    permission: "system:dictionary:export",
                    method: "GET",
                    api_path: "/api/dictionaries/{id}/export",
                    sort: 70,
                },
                DefaultAction {
                    name: "dictionaries:details-tree",
                    title: "Details tree",
                    permission: "system:dictionary:details-tree",
                    method: "GET",
                    api_path: "/api/dictionaries/{id}/details/tree",
                    sort: 80,
                },
                DefaultAction {
                    name: "dictionary-details:create",
                    title: "Create detail",
                    permission: "system:dictionary-detail:create",
                    method: "POST",
                    api_path: "/api/dictionary-details",
                    sort: 90,
                },
                DefaultAction {
                    name: "dictionary-details:tree-by-type",
                    title: "Type tree",
                    permission: "system:dictionary-detail:tree-by-type",
                    method: "GET",
                    api_path: "/api/dictionary-details/tree-by-type",
                    sort: 100,
                },
                DefaultAction {
                    name: "dictionary-details:by-parent",
                    title: "Children",
                    permission: "system:dictionary-detail:by-parent",
                    method: "GET",
                    api_path: "/api/dictionary-details/by-parent",
                    sort: 110,
                },
                DefaultAction {
                    name: "dictionary-details:get",
                    title: "Get detail",
                    permission: "system:dictionary-detail:get",
                    method: "GET",
                    api_path: "/api/dictionary-details/{id}",
                    sort: 120,
                },
                DefaultAction {
                    name: "dictionary-details:update",
                    title: "Update detail",
                    permission: "system:dictionary-detail:update",
                    method: "PUT",
                    api_path: "/api/dictionary-details/{id}",
                    sort: 130,
                },
                DefaultAction {
                    name: "dictionary-details:delete",
                    title: "Delete detail",
                    permission: "system:dictionary-detail:delete",
                    method: "DELETE",
                    api_path: "/api/dictionary-details/{id}",
                    sort: 140,
                },
                DefaultAction {
                    name: "dictionary-details:path",
                    title: "Path",
                    permission: "system:dictionary-detail:path",
                    method: "GET",
                    api_path: "/api/dictionary-details/{id}/path",
                    sort: 150,
                },
            ],
        ),
        (
            8,
            "files",
            "Files",
            "view/files/index.vue",
            "file",
            vec![
                DefaultAction {
                    name: "files:list",
                    title: "List",
                    permission: "system:file:list",
                    method: "GET",
                    api_path: "/api/files",
                    sort: 10,
                },
                DefaultAction {
                    name: "files:import-url",
                    title: "Import URL",
                    permission: "system:file:import-url",
                    method: "POST",
                    api_path: "/api/files/import-url",
                    sort: 20,
                },
                DefaultAction {
                    name: "files:upload",
                    title: "Upload",
                    permission: "system:file:upload",
                    method: "POST",
                    api_path: "/api/files/upload",
                    sort: 30,
                },
                DefaultAction {
                    name: "files:delete",
                    title: "Delete",
                    permission: "system:file:delete",
                    method: "DELETE",
                    api_path: "/api/files/{id}",
                    sort: 40,
                },
                DefaultAction {
                    name: "files:rename",
                    title: "Rename",
                    permission: "system:file:rename",
                    method: "PATCH",
                    api_path: "/api/files/{id}/name",
                    sort: 50,
                },
                DefaultAction {
                    name: "files:categories-list",
                    title: "Categories",
                    permission: "system:file:categories-list",
                    method: "GET",
                    api_path: "/api/attachment-categories",
                    sort: 60,
                },
                DefaultAction {
                    name: "files:categories-create",
                    title: "New category",
                    permission: "system:file:categories-create",
                    method: "POST",
                    api_path: "/api/attachment-categories",
                    sort: 70,
                },
                DefaultAction {
                    name: "files:categories-delete",
                    title: "Remove category",
                    permission: "system:file:categories-delete",
                    method: "DELETE",
                    api_path: "/api/attachment-categories/{id}",
                    sort: 80,
                },
            ],
        ),
        (
            9,
            "login-logs",
            "Login logs",
            "view/login-logs/index.vue",
            "history",
            vec![
                DefaultAction {
                    name: "login-logs:list",
                    title: "List",
                    permission: "system:login-log:list",
                    method: "GET",
                    api_path: "/api/login-logs",
                    sort: 10,
                },
                DefaultAction {
                    name: "login-logs:batch-delete",
                    title: "Batch delete",
                    permission: "system:login-log:batch-delete",
                    method: "DELETE",
                    api_path: "/api/login-logs",
                    sort: 20,
                },
                DefaultAction {
                    name: "login-logs:get",
                    title: "Get",
                    permission: "system:login-log:get",
                    method: "GET",
                    api_path: "/api/login-logs/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "login-logs:delete",
                    title: "Delete",
                    permission: "system:login-log:delete",
                    method: "DELETE",
                    api_path: "/api/login-logs/{id}",
                    sort: 40,
                },
            ],
        ),
        (
            10,
            "operation-logs",
            "Operation logs",
            "view/operation-logs/index.vue",
            "list",
            vec![
                DefaultAction {
                    name: "operation-logs:list",
                    title: "List",
                    permission: "system:operation-log:list",
                    method: "GET",
                    api_path: "/api/operation-logs",
                    sort: 10,
                },
                DefaultAction {
                    name: "operation-logs:batch-delete",
                    title: "Batch delete",
                    permission: "system:operation-log:batch-delete",
                    method: "DELETE",
                    api_path: "/api/operation-logs",
                    sort: 20,
                },
                DefaultAction {
                    name: "operation-logs:delete",
                    title: "Delete",
                    permission: "system:operation-log:delete",
                    method: "DELETE",
                    api_path: "/api/operation-logs/{id}",
                    sort: 30,
                },
            ],
        ),
        (
            11,
            "profile",
            "Profile",
            "view/profile/index.vue",
            "user",
            Vec::new(),
        ),
        (
            12,
            "system-config",
            "System config",
            "view/system-config/index.vue",
            "settings",
            vec![
                DefaultAction {
                    name: "system-config:get",
                    title: "View config",
                    permission: "system:config:get",
                    method: "GET",
                    api_path: "/api/system/config",
                    sort: 10,
                },
                DefaultAction {
                    name: "system-config:update",
                    title: "Update config",
                    permission: "system:config:update",
                    method: "PUT",
                    api_path: "/api/system/config",
                    sort: 20,
                },
            ],
        ),
        (
            13,
            "system-state",
            "System status",
            "view/system-state/index.vue",
            "activity",
            vec![DefaultAction {
                name: "system-state:get",
                title: "View status",
                permission: "system:state:get",
                method: "GET",
                api_path: "/api/system/server-info",
                sort: 10,
            }],
        ),
        (
            14,
            "departments",
            "Departments",
            "view/system/depts/index.vue",
            "building",
            vec![
                DefaultAction {
                    name: "departments:list",
                    title: "List",
                    permission: "system:dept:list",
                    method: "GET",
                    api_path: "/api/depts",
                    sort: 10,
                },
                DefaultAction {
                    name: "departments:create",
                    title: "Create",
                    permission: "system:dept:create",
                    method: "POST",
                    api_path: "/api/depts",
                    sort: 20,
                },
                DefaultAction {
                    name: "departments:update",
                    title: "Update",
                    permission: "system:dept:update",
                    method: "PUT",
                    api_path: "/api/depts/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "departments:delete",
                    title: "Delete",
                    permission: "system:dept:delete",
                    method: "DELETE",
                    api_path: "/api/depts/{id}",
                    sort: 40,
                },
            ],
        ),
        (
            15,
            "permissions",
            "Permissions",
            "view/system/permissions/index.vue",
            "key-round",
            vec![
                DefaultAction {
                    name: "permissions:list",
                    title: "List",
                    permission: "system:permission:list",
                    method: "GET",
                    api_path: "/api/permissions",
                    sort: 10,
                },
                DefaultAction {
                    name: "permissions:create",
                    title: "Create",
                    permission: "system:permission:create",
                    method: "POST",
                    api_path: "/api/permissions",
                    sort: 20,
                },
                DefaultAction {
                    name: "permissions:update",
                    title: "Update",
                    permission: "system:permission:update",
                    method: "PUT",
                    api_path: "/api/permissions/{id}",
                    sort: 30,
                },
                DefaultAction {
                    name: "permissions:delete",
                    title: "Delete",
                    permission: "system:permission:delete",
                    method: "DELETE",
                    api_path: "/api/permissions/{id}",
                    sort: 40,
                },
            ],
        ),
        (
            16,
            "api-permissions",
            "API permission bindings",
            "view/system/permissions/api-bindings.vue",
            "lock-keyhole",
            vec![
                DefaultAction {
                    name: "api-permissions:apis-read",
                    title: "View bindings",
                    permission: "system:permission:apis-read",
                    method: "GET",
                    api_path: "/api/permissions/{id}/apis",
                    sort: 10,
                },
                DefaultAction {
                    name: "api-permissions:apis-update",
                    title: "Edit bindings",
                    permission: "system:permission:apis-update",
                    method: "PUT",
                    api_path: "/api/permissions/{id}/apis",
                    sort: 20,
                },
            ],
        ),
    ]
    .into_iter()
    .map(|(id, name, title, component, icon, actions)| {
        default_menu(id, name, title, component, icon, actions)
    })
    .collect()
}

fn default_menu(
    id: i64,
    name: &str,
    title: &str,
    component: &str,
    icon: &str,
    actions: Vec<DefaultAction>,
) -> MenuView {
    let mut menu = MenuView {
        id,
        parent_id: 0,
        path: name.to_string(),
        name: name.to_string(),
        hidden: false,
        component: component.to_string(),
        sort: id as i32,
        meta: MenuMeta {
            active_name: String::new(),
            keep_alive: false,
            default_menu: false,
            title: title.to_string(),
            icon: icon.to_string(),
            close_tab: false,
            transition_type: String::new(),
        },
        parameters: Vec::new(),
        menu_btn: Vec::new(),
        menu_type: "page".to_string(),
        permission: Some(default_page_permission_code(name)),
        permission_id: None,
        method: None,
        api_path: None,
        children: Vec::new(),
    };
    menu.children = actions
        .into_iter()
        .map(|action| default_action(id, action))
        .collect();
    menu
}

fn default_page_permission_code(name: &str) -> String {
    format!("system:{}:page", name.replace('-', "_"))
}

fn default_action(parent_id: i64, action: DefaultAction) -> MenuView {
    MenuView {
        id: 0,
        parent_id,
        path: String::new(),
        name: action.name.to_string(),
        hidden: true,
        component: String::new(),
        sort: action.sort,
        meta: MenuMeta {
            active_name: String::new(),
            keep_alive: false,
            default_menu: false,
            title: action.title.to_string(),
            icon: String::new(),
            close_tab: false,
            transition_type: String::new(),
        },
        parameters: Vec::new(),
        menu_btn: Vec::new(),
        menu_type: "action".to_string(),
        permission: Some(action.permission.to_string()),
        permission_id: None,
        method: Some(action.method.to_string()),
        api_path: Some(action.api_path.to_string()),
        children: Vec::new(),
    }
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
    pub parameters: Option<serde_json::Value>,
    pub menu_btn: Option<serde_json::Value>,
    pub menu_type: String,
    pub permission: Option<String>,
    pub permission_id: Option<i64>,
    pub method: Option<String>,
    pub api_path: Option<String>,
}

#[derive(Debug, Clone, FromRow)]
struct UserPermissionRow {
    pub id: i64,
    pub code: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MenuIdRequest {
    #[serde(rename = "ID", alias = "id", alias = "menuId")]
    pub id: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetMenuRolesRequest {
    #[serde(rename = "menuId")]
    pub menu_id: i64,
    #[serde(rename = "roleIds")]
    pub authority_ids: Vec<i64>,
}

#[derive(Debug, thiserror::Error)]
pub enum MenuError {
    #[error("menu not found")]
    NotFound,
    #[error("{0}")]
    Database(#[from] sqlx::Error),
    #[error("invalid menu payload")]
    InvalidPayload,
    #[error("default role permissions cannot be changed")]
    RootAuthorityImmutable,
}

impl From<MenuError> for AppError {
    fn from(error: MenuError) -> Self {
        match error {
            MenuError::NotFound => errors::menu::MENU_NOT_FOUND.into(),
            MenuError::Database(error) => {
                errors::menu::MENU_DB_FAILED.into_error().with_source(error)
            }
            MenuError::InvalidPayload => errors::menu::MENU_INVALID_PAYLOAD.into(),
            MenuError::RootAuthorityImmutable => errors::menu::ROOT_AUTHORITY_IMMUTABLE.into(),
        }
    }
}

// ensure default menu
pub async fn ensure_default_menu(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    for mut menu in default_menus() {
        let children = std::mem::take(&mut menu.children);
        let menu_id = insert_menu_node(pool, menu, 0).await?;
        for child in children {
            insert_menu_node(pool, child, menu_id).await?;
        }
    }

    sqlx::query(
        r#"
        select setval(
            pg_get_serial_sequence('sys_menus', 'id'),
            greatest((select coalesce(max(id), 1) from sys_menus), 1),
            true
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn insert_menu_node(
    pool: &sqlx::PgPool,
    menu: MenuView,
    parent_id: i64,
) -> Result<i64, sqlx::Error> {
    let menu_id: i64 = sqlx::query_scalar(
        r#"
        insert into sys_menus (
            parent_id, path, name, hidden, component, sort,
            active_name, keep_alive, default_menu, title, icon, close_tab, transition_type,
            parameters, menu_btn, menu_type, permission, method, api_path, permission_id
        ) values (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18, $19, coalesce($20, (select id from sys_permissions where code = $17))
        )
        on conflict (name) do update
        set parent_id = excluded.parent_id,
            path = excluded.path,
            hidden = excluded.hidden,
            component = excluded.component,
            sort = excluded.sort,
            active_name = excluded.active_name,
            keep_alive = excluded.keep_alive,
            default_menu = excluded.default_menu,
            title = excluded.title,
            icon = excluded.icon,
            close_tab = excluded.close_tab,
            transition_type = excluded.transition_type,
            parameters = excluded.parameters,
            menu_btn = excluded.menu_btn,
            menu_type = excluded.menu_type,
            permission = excluded.permission,
            method = excluded.method,
            api_path = excluded.api_path,
            permission_id = excluded.permission_id
        returning id
        "#,
    )
    .bind(parent_id)
    .bind(menu.path)
    .bind(menu.name)
    .bind(menu.hidden)
    .bind(menu.component)
    .bind(menu.sort)
    .bind(menu.meta.active_name)
    .bind(menu.meta.keep_alive)
    .bind(menu.meta.default_menu)
    .bind(menu.meta.title)
    .bind(menu.meta.icon)
    .bind(menu.meta.close_tab)
    .bind(menu.meta.transition_type)
    .bind(serde_json::to_value(menu.parameters).unwrap_or_else(|_| serde_json::json!([])))
    .bind(serde_json::to_value(menu.menu_btn).unwrap_or_else(|_| serde_json::json!([])))
    .bind(menu.menu_type)
    .bind(menu.permission.clone())
    .bind(menu.method)
    .bind(menu.api_path)
    .bind(menu.permission_id)
    .fetch_one(pool)
    .await?;

    Ok(menu_id)
}

pub async fn get_menu_tree_for_user(
    pool: &sqlx::PgPool,
    user_id: i64,
    _authority_id: i64,
) -> Result<Vec<MenuView>, MenuError> {
    let rows = load_menu_records(pool).await?;
    let has_super_admin_role = crate::roles::user_has_role_code(pool, user_id, "super_admin")
        .await
        .map_err(MenuError::Database)?;

    if is_menu_super_admin_identity(has_super_admin_role) {
        let rows = filter_visible_navigation(&rows);
        return Ok(build_tree(&rows, 0));
    }

    let permissions = sqlx::query_as::<_, UserPermissionRow>(
        r#"
        select distinct p.id, p.code
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        join sys_role_permissions rp on rp.role_id = r.id
        join sys_permissions p on p.id = rp.permission_id
        where ur.user_id = $1
          and r.status = 'enabled'
          and p.status = 'enabled'
        order by p.code
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let permission_ids = permissions
        .iter()
        .map(|permission| permission.id)
        .collect::<HashSet<_>>();
    let permission_codes = permissions
        .iter()
        .map(|permission| permission.code.as_str())
        .collect::<HashSet<_>>();
    let rows = filter_rows_for_permissions(&rows, &permission_ids, &permission_codes);

    Ok(build_tree(&rows, 0))
}

pub async fn get_menu_list(pool: &sqlx::PgPool) -> Result<Vec<MenuView>, MenuError> {
    let rows = load_menu_records(pool).await?;

    Ok(build_tree(&rows, 0))
}

async fn load_menu_records(pool: &sqlx::PgPool) -> Result<Vec<MenuRecord>, MenuError> {
    let rows = sqlx::query_as::<_, MenuRecord>(
        r#"
        select
            id, parent_id, path, name, hidden, component, sort, active_name, keep_alive,
            default_menu, title, icon, close_tab, transition_type, parameters, menu_btn,
            menu_type, permission, method, api_path, permission_id
        from sys_menus
        order by sort asc, id asc
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

pub async fn get_base_menu_tree(pool: &sqlx::PgPool) -> Result<Vec<MenuView>, MenuError> {
    get_menu_list(pool).await
}

pub async fn add_base_menu(pool: &sqlx::PgPool, payload: MenuView) -> Result<(), MenuError> {
    sqlx::query(
        r#"
        insert into sys_menus (
            parent_id, path, name, hidden, component, sort,
            active_name, keep_alive, default_menu, title, icon, close_tab, transition_type,
            parameters, menu_btn, menu_type, permission, method, api_path, permission_id
        ) values (
            $1, $2, $3, $4, $5, $6,
            $7, $8, $9, $10, $11, $12, $13,
            $14, $15, $16, $17, $18, $19, coalesce($20, (select id from sys_permissions where code = $17))
        )
        "#,
    )
    .bind(payload.parent_id)
    .bind(payload.path)
    .bind(payload.name)
    .bind(payload.hidden)
    .bind(payload.component)
    .bind(payload.sort)
    .bind(payload.meta.active_name)
    .bind(payload.meta.keep_alive)
    .bind(payload.meta.default_menu)
    .bind(payload.meta.title)
    .bind(payload.meta.icon)
    .bind(payload.meta.close_tab)
    .bind(payload.meta.transition_type)
    .bind(serde_json::to_value(payload.parameters).map_err(|_| MenuError::InvalidPayload)?)
    .bind(serde_json::to_value(payload.menu_btn).map_err(|_| MenuError::InvalidPayload)?)
    .bind(payload.menu_type)
    .bind(payload.permission.clone())
    .bind(payload.method)
    .bind(payload.api_path)
    .bind(payload.permission_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_base_menu(pool: &sqlx::PgPool, payload: MenuView) -> Result<(), MenuError> {
    sqlx::query(
        r#"
        update sys_menus
        set parent_id = $1,
            path = $2,
            name = $3,
            hidden = $4,
            component = $5,
            sort = $6,
            active_name = $7,
            keep_alive = $8,
            default_menu = $9,
            title = $10,
            icon = $11,
            close_tab = $12,
            transition_type = $13,
            parameters = $14,
            menu_btn = $15,
            menu_type = $16,
            permission = $17,
            method = $18,
            api_path = $19,
            permission_id = coalesce($20, (select id from sys_permissions where code = $17))
        where id = $21
        "#,
    )
    .bind(payload.parent_id)
    .bind(payload.path)
    .bind(payload.name)
    .bind(payload.hidden)
    .bind(payload.component)
    .bind(payload.sort)
    .bind(payload.meta.active_name)
    .bind(payload.meta.keep_alive)
    .bind(payload.meta.default_menu)
    .bind(payload.meta.title)
    .bind(payload.meta.icon)
    .bind(payload.meta.close_tab)
    .bind(payload.meta.transition_type)
    .bind(serde_json::to_value(payload.parameters).map_err(|_| MenuError::InvalidPayload)?)
    .bind(serde_json::to_value(payload.menu_btn).map_err(|_| MenuError::InvalidPayload)?)
    .bind(payload.menu_type)
    .bind(payload.permission.clone())
    .bind(payload.method)
    .bind(payload.api_path)
    .bind(payload.permission_id)
    .bind(payload.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_base_menu(pool: &sqlx::PgPool, menu_id: i64) -> Result<(), MenuError> {
    sqlx::query("delete from sys_menus where id = $1 or parent_id = $1")
        .bind(menu_id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_base_menu_by_id(pool: &sqlx::PgPool, menu_id: i64) -> Result<MenuView, MenuError> {
    let row = sqlx::query_as::<_, MenuRecord>(
        r#"
        select
            id, parent_id, path, name, hidden, component, sort, active_name, keep_alive,
            default_menu, title, icon, close_tab, transition_type, parameters, menu_btn,
            menu_type, permission, method, api_path, permission_id
        from sys_menus
        where id = $1
        "#,
    )
    .bind(menu_id)
    .fetch_optional(pool)
    .await?
    .ok_or(MenuError::NotFound)?;

    build_menu_view(&row)
}

pub async fn get_menu_roles(
    pool: &sqlx::PgPool,
    menu_id: i64,
) -> Result<MenuRoleSelection, MenuError> {
    let authority_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        select rp.role_id
        from sys_menus m
        join sys_role_permissions rp on rp.permission_id = m.permission_id
        join sys_roles r on r.id = rp.role_id
        where m.id = $1
          and m.permission_id is not null
          and r.status = 'enabled'
        order by rp.role_id
        "#,
    )
    .bind(menu_id)
    .fetch_all(pool)
    .await?;
    Ok(MenuRoleSelection { authority_ids })
}

fn normalize_role_ids_for_menu_permission_sync(role_ids: Vec<i64>) -> Vec<i64> {
    role_ids
        .into_iter()
        .filter(|role_id| *role_id > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub async fn set_menu_roles(
    pool: &sqlx::PgPool,
    payload: SetMenuRolesRequest,
) -> Result<(), MenuError> {
    let authority_ids = normalize_role_ids_for_menu_permission_sync(payload.authority_ids);
    sync_menu_permission_roles(pool, payload.menu_id, &authority_ids).await?;
    Ok(())
}

async fn sync_menu_permission_roles(
    pool: &sqlx::PgPool,
    menu_id: i64,
    role_ids: &[i64],
) -> Result<(), MenuError> {
    let permission_id: Option<i64> =
        sqlx::query_scalar("select permission_id from sys_menus where id = $1")
            .bind(menu_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    let Some(permission_id) = permission_id else {
        return Ok(());
    };

    let existing_role_ids: Vec<i64> = sqlx::query_scalar(
        r#"
        select id
        from sys_roles
        where id = any($1)
        order by id
        "#,
    )
    .bind(role_ids)
    .fetch_all(pool)
    .await?;

    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_role_permissions where permission_id = $1")
        .bind(permission_id)
        .execute(&mut *tx)
        .await?;
    for role_id in existing_role_ids {
        sqlx::query(
            r#"
            insert into sys_role_permissions (role_id, permission_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(role_id)
        .bind(permission_id)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

// filter visible navigation
fn filter_visible_navigation(rows: &[MenuRecord]) -> Vec<MenuRecord> {
    rows.iter()
        .filter(|row| row.menu_type != "action" && !row.hidden)
        .cloned()
        .collect()
}

fn is_menu_super_admin_identity(has_super_admin_role: bool) -> bool {
    has_super_admin_role
}

fn filter_rows_for_permissions(
    rows: &[MenuRecord],
    permission_ids: &HashSet<i64>,
    permission_codes: &HashSet<&str>,
) -> Vec<MenuRecord> {
    let rows_by_id = rows
        .iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let action_ancestor_ids = action_ancestor_ids(rows, &rows_by_id);
    let mut included_ids = HashSet::new();

    for row in rows {
        if row.menu_type == "action" {
            if row_allowed_by_permissions(row, permission_ids, permission_codes) {
                include_with_ancestors(row.id, &rows_by_id, &mut included_ids);
            }
            continue;
        }

        if row.hidden {
            continue;
        }

        let has_explicit_permission = row.permission_id.is_some() || row.permission.is_some();
        let allowed = if has_explicit_permission {
            row_allowed_by_permissions(row, permission_ids, permission_codes)
        } else {
            !action_ancestor_ids.contains(&row.id)
        };

        if allowed {
            include_with_ancestors(row.id, &rows_by_id, &mut included_ids);
        }
    }

    rows.iter()
        .filter(|row| included_ids.contains(&row.id))
        .filter(|row| row.menu_type != "action" && !row.hidden)
        .cloned()
        .collect()
}

fn row_allowed_by_permissions(
    row: &MenuRecord,
    permission_ids: &HashSet<i64>,
    permission_codes: &HashSet<&str>,
) -> bool {
    row.permission_id
        .is_some_and(|permission_id| permission_ids.contains(&permission_id))
        || row
            .permission
            .as_deref()
            .is_some_and(|permission| permission_codes.contains(permission))
}

fn action_ancestor_ids<'a>(
    rows: &'a [MenuRecord],
    rows_by_id: &HashMap<i64, &'a MenuRecord>,
) -> HashSet<i64> {
    let mut ancestor_ids = HashSet::new();

    for row in rows.iter().filter(|row| row.menu_type == "action") {
        let mut current_id = row.parent_id;
        while current_id != 0 {
            let Some(parent) = rows_by_id.get(&current_id) else {
                break;
            };
            if !ancestor_ids.insert(parent.id) {
                break;
            }
            current_id = parent.parent_id;
        }
    }

    ancestor_ids
}

fn include_with_ancestors(
    row_id: i64,
    rows_by_id: &HashMap<i64, &MenuRecord>,
    included_ids: &mut HashSet<i64>,
) {
    let mut current_id = row_id;
    while current_id != 0 {
        let Some(row) = rows_by_id.get(&current_id) else {
            break;
        };
        if !included_ids.insert(current_id) {
            break;
        }
        current_id = row.parent_id;
    }
}

fn build_tree(rows: &[MenuRecord], parent_id: i64) -> Vec<MenuView> {
    let mut menus = rows
        .iter()
        .filter(|row| row.parent_id == parent_id)
        .filter_map(|row| {
            let mut view = build_menu_view(row).ok()?;
            view.children = build_tree(rows, row.id);
            Some(view)
        })
        .collect::<Vec<_>>();

    menus.sort_by_key(|item| (item.sort, item.id));
    menus
}

fn build_menu_view(row: &MenuRecord) -> Result<MenuView, MenuError> {
    Ok(MenuView {
        id: row.id,
        parent_id: row.parent_id,
        path: row.path.clone(),
        name: row.name.clone(),
        hidden: row.hidden,
        component: row.component.clone(),
        sort: row.sort,
        meta: MenuMeta {
            active_name: row.active_name.clone(),
            keep_alive: row.keep_alive,
            default_menu: row.default_menu,
            title: row.title.clone(),
            icon: row.icon.clone(),
            close_tab: row.close_tab,
            transition_type: row.transition_type.clone(),
        },
        parameters: serde_json::from_value(
            row.parameters
                .clone()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(|_| MenuError::InvalidPayload)?,
        menu_btn: serde_json::from_value(
            row.menu_btn
                .clone()
                .unwrap_or_else(|| serde_json::json!([])),
        )
        .map_err(|_| MenuError::InvalidPayload)?,
        menu_type: row.menu_type.clone(),
        permission: row.permission.clone(),
        permission_id: row.permission_id,
        method: row.method.clone(),
        api_path: row.api_path.clone(),
        children: Vec::new(),
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct MenuRoleSelection {
    #[serde(rename = "roleIds")]
    pub authority_ids: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_super_admin_identity_uses_role_code() {
        assert!(is_menu_super_admin_identity(true));
        assert!(!is_menu_super_admin_identity(false));
    }

    #[test]
    fn normalize_role_ids_for_menu_permission_sync_deduplicates_and_filters_invalid_ids() {
        assert_eq!(
            normalize_role_ids_for_menu_permission_sync(vec![3, -1, 3, 0, 1]),
            vec![1, 3]
        );
    }
}
