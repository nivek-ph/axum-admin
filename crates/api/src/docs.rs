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
        crate::routes::users::get_current_user,
        crate::routes::users::list_users,
        crate::routes::users::create_user,
        crate::routes::users::change_password,
        crate::routes::users::update_user,
        crate::routes::users::update_current_user,
        crate::routes::users::update_current_user_settings,
        crate::routes::users::delete_user,
        crate::routes::users::reset_user_password,
        crate::routes::users::replace_user_roles,
        crate::routes::users::get_user_access,
        crate::routes::menus::get_current_menus,
        crate::routes::menus::get_menu_tree,
        crate::routes::roles::list_roles,
        crate::routes::roles::create_role,
        crate::routes::roles::update_role,
        crate::routes::roles::delete_role,
        crate::routes::roles::get_role_access,
        crate::routes::roles::replace_role_access,
        crate::routes::departments::get_department_tree,
        crate::routes::departments::find_department,
        crate::routes::departments::create_department,
        crate::routes::departments::update_department,
        crate::routes::departments::delete_department,
        crate::routes::dictionaries::list_dictionaries,
        crate::routes::dictionaries::create_dictionary,
        crate::routes::dictionaries::import_dictionary,
        crate::routes::dictionaries::get_dictionary_tree_by_type,
        crate::routes::dictionaries::find_dictionary,
        crate::routes::dictionaries::update_dictionary,
        crate::routes::dictionaries::delete_dictionary,
        crate::routes::dictionaries::export_dictionary,
        crate::routes::dictionaries::get_dictionary_tree,
        crate::routes::dictionaries::create_dictionary_tree_node,
        crate::routes::dictionaries::find_dictionary_tree_node,
        crate::routes::dictionaries::update_dictionary_tree_node,
        crate::routes::dictionaries::delete_dictionary_tree_node,
        crate::routes::dictionaries::list_dictionary_tree_node_children,
        crate::routes::dictionaries::get_dictionary_tree_node_path,
        crate::routes::files::list_files,
        crate::routes::files::import_url,
        crate::routes::files::start_upload,
        crate::routes::files::upload_status,
        crate::routes::files::upload_chunk,
        crate::routes::files::complete_upload,
        crate::routes::files::delete_file,
        crate::routes::files::rename_file,
        crate::routes::storages::list,
        crate::routes::storages::find,
        crate::routes::storages::create,
        crate::routes::storages::update,
        crate::routes::storages::update_status,
        crate::routes::storages::set_default,
        crate::routes::storages::delete,
        crate::routes::parameters::list_parameters,
        crate::routes::parameters::create_parameter,
        crate::routes::parameters::get_parameter_by_key,
        crate::routes::parameters::find_parameter,
        crate::routes::parameters::update_parameter,
        crate::routes::parameters::delete_parameter,
        crate::routes::parameters::delete_parameters,
        crate::routes::audit::events::list_audit_events,
        crate::routes::audit::events::get_audit_stats,
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
        (name = "storages", description = "Storages"),
        (name = "parameters", description = "Parameters"),
        (name = "audit", description = "Audit"),
    )
)]
pub struct ApiDoc;
