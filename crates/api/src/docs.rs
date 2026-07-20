use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    servers((url = "/api", description = "API base path")),
    paths(
        crate::routes::health::health,
        crate::routes::auth::captcha::captcha,
        crate::routes::auth::login::login,
        crate::routes::auth::refresh::refresh,
        crate::routes::auth::logout::logout,
        crate::routes::users::get_user_info,
        crate::routes::users::get_user_list_by_query,
        crate::routes::users::admin_register,
        crate::routes::users::change_password,
        crate::routes::users::set_user_info_by_id,
        crate::routes::users::set_self_info,
        crate::routes::users::set_self_setting,
        crate::routes::users::delete_user_by_id,
        crate::routes::users::reset_password_by_id,
        crate::routes::users::set_user_roles_by_id,
        crate::routes::menus::get_menu,
        crate::routes::menus::get_base_menu_tree,
        crate::routes::roles::get_roles,
        crate::routes::roles::create_role,
        crate::routes::roles::update_role,
        crate::routes::roles::delete_role,
        crate::routes::roles::get_role_menus,
        crate::routes::roles::set_role_menus,
        crate::routes::roles::get_role_depts,
        crate::routes::roles::set_role_depts,
        crate::routes::roles::get_role_users,
        crate::routes::roles::set_role_users,
        crate::routes::departments::get_dept_tree,
        crate::routes::departments::find_dept_by_id,
        crate::routes::departments::create_dept,
        crate::routes::departments::update_dept_by_id,
        crate::routes::departments::delete_dept_by_id,
        crate::routes::dictionaries::get_sys_dictionary_list,
        crate::routes::dictionaries::create_sys_dictionary,
        crate::routes::dictionaries::import_sys_dictionary,
        crate::routes::dictionaries::get_dictionary_tree_by_type,
        crate::routes::dictionaries::find_sys_dictionary_by_id,
        crate::routes::dictionaries::update_sys_dictionary_by_id,
        crate::routes::dictionaries::delete_sys_dictionary_by_id,
        crate::routes::dictionaries::export_sys_dictionary_by_id,
        crate::routes::dictionaries::get_dictionary_tree,
        crate::routes::dictionaries::create_dictionary_tree_node,
        crate::routes::dictionaries::find_dictionary_tree_node,
        crate::routes::dictionaries::update_dictionary_tree_node,
        crate::routes::dictionaries::delete_dictionary_tree_node,
        crate::routes::dictionaries::get_dictionary_tree_node_children,
        crate::routes::dictionaries::get_dictionary_tree_node_path,
        crate::routes::files::get_file_list_by_query,
        crate::routes::files::import_url,
        crate::routes::files::upload_file,
        crate::routes::files::delete_file_by_id,
        crate::routes::files::edit_file_name_by_id,
        crate::routes::parameters::get_sys_params_list,
        crate::routes::parameters::create_sys_params,
        crate::routes::parameters::get_sys_param,
        crate::routes::parameters::find_sys_params_by_id,
        crate::routes::parameters::update_sys_params_by_id,
        crate::routes::parameters::delete_sys_params_by_id,
        crate::routes::parameters::delete_sys_params_by_ids,
        crate::routes::audit::events::get_audit_events,
        crate::routes::audit::events::find_audit_event,
        crate::routes::audit::events::analyze_audit_events,
    ),
    tags(
        (name = "auth", description = "Auth"),
        (name = "users", description = "Users"),
        (name = "menus", description = "Menus"),
        (name = "roles", description = "Roles"),
        (name = "departments", description = "Departments"),
        (name = "dictionaries", description = "Dictionaries"),
        (name = "files", description = "Files"),
        (name = "parameters", description = "Parameters"),
        (name = "audit", description = "Audit"),
    )
)]
pub struct ApiDoc;
