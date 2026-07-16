use audit::AuditService;
use auth::{captcha::CaptchaService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, departments::DepartmentService, menus::MenuService, roles::RoleService,
    users::UserService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};

#[derive(Clone)]
pub struct AppState {
    pub tokens: TokenService,
    pub captcha: CaptchaService,
    pub users: UserService,
    pub roles: RoleService,
    pub departments: DepartmentService,
    pub access: AccessService,
    pub dictionaries: DictionaryService,
    pub parameters: ParameterService,
    pub menus: MenuService,
    pub audits: AuditService,
    pub files: FileService,
}

#[cfg(test)]
pub(crate) fn test_state(pool: sqlx::PgPool) -> AppState {
    let passwords = auth::password::PasswordService::new();
    let access = AccessService::new(pool.clone());
    AppState {
        tokens: TokenService::without_revocation_store("test-secret"),
        captcha: CaptchaService::without_store(),
        users: UserService::new(pool.clone(), passwords),
        roles: RoleService::new(pool.clone()),
        departments: DepartmentService::new(pool.clone()),
        access: access.clone(),
        dictionaries: DictionaryService::new(pool.clone()),
        parameters: ParameterService::new(pool.clone()),
        menus: MenuService::new(pool.clone(), access),
        audits: AuditService::new(pool.clone()),
        files: FileService::new(pool, "./uploads"),
    }
}
