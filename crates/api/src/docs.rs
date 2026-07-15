use serde::{Deserialize, Serialize};
use utoipa::{
    Modify, OpenApi, ToSchema,
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthData {
    pub alive: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub code: String,
    pub data: HealthData,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaptchaData {
    #[schema(value_type = i32)]
    #[serde(rename = "captchaLength")]
    pub captcha_length: i32,
    #[serde(rename = "picPath")]
    pub pic_path: String,
    #[serde(rename = "captchaId")]
    pub captcha_id: String,
    #[serde(rename = "openCaptcha")]
    pub open_captcha: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaptchaResponse {
    pub code: String,
    pub data: CaptchaData,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RoleDoc {
    pub id: i64,
    pub code: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserDoc {
    #[serde(rename = "id")]
    pub id: i64,
    pub uuid: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "headerImg")]
    pub header_img: String,
    #[serde(rename = "homeRoute")]
    pub home_route: String,
    pub roles: Vec<RoleDoc>,
    #[serde(rename = "roleIds")]
    pub role_ids: Vec<i64>,
    pub enable: i32,
    pub phone: String,
    pub email: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginData {
    pub token: String,
    pub user: UserDoc,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub code: String,
    pub data: LoginData,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserInfoData {
    #[serde(rename = "userInfo")]
    pub user_info: UserDoc,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserInfoResponse {
    pub code: String,
    pub data: UserInfoData,
    pub message: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MenuMetaDoc {
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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MenuDoc {
    #[serde(rename = "id")]
    pub id: i64,
    #[serde(rename = "parentId")]
    pub parent_id: i64,
    pub path: String,
    pub name: String,
    pub hidden: bool,
    pub component: String,
    pub sort: i32,
    pub meta: MenuMetaDoc,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MenuData {
    pub menus: Vec<MenuDoc>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct MenuResponse {
    pub code: String,
    pub data: MenuData,
    pub message: String,
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        crate::routes::health::health,
        crate::routes::auth::captcha::captcha,
        crate::routes::auth::login::login,
        crate::routes::auth::logout::logout,
        crate::routes::users::get_user_info,
        crate::routes::menus::get_menu,
    ),
    components(
        schemas(
            HealthData,
            HealthResponse,
            CaptchaData,
            CaptchaResponse,
            crate::routes::auth::login::LoginRequest,
            RoleDoc,
            UserDoc,
            LoginData,
            LoginResponse,
            UserInfoData,
            UserInfoResponse,
            MenuMetaDoc,
            MenuDoc,
            MenuData,
            MenuResponse
        )
    ),
    tags(
        (name = "auth", description = "Auth endpoints"),
        (name = "user", description = "User endpoints"),
        (name = "menu", description = "Menu endpoints"),
        (name = "system", description = "System endpoints")
    )
)]
pub struct ApiDoc;
