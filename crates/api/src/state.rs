use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, departments::DepartmentService, menus::MenuService, roles::RoleService,
    users::UserService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};

#[derive(Clone)]
pub struct AppState {
    pub public_base_url: String,
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
    pub audit_analyzer: AuditAnalyzer,
    pub files: FileService,
}

#[cfg(test)]
pub(crate) fn test_state(pool: sqlx::PgPool) -> AppState {
    let passwords = auth::password::PasswordService::new();
    let access = AccessService::new(pool.clone());
    let audits = AuditService::new(pool.clone());
    let users = UserService::new(pool.clone(), access.clone(), audits.clone(), passwords);
    let roles = RoleService::new(pool.clone(), access.clone());
    let departments = DepartmentService::new(pool.clone(), access.clone());
    let dictionaries = DictionaryService::new(pool.clone());
    let parameters = ParameterService::new(pool.clone());
    let menus = MenuService::new(pool.clone());
    let files = FileService::new(pool, "./uploads");
    AppState {
        public_base_url: "http://127.0.0.1:3000".to_string(),
        tokens: TokenService::without_session_store("test-secret"),
        captcha: CaptchaService::without_store(),
        users,
        roles,
        departments,
        access,
        dictionaries,
        parameters,
        menus,
        audits,
        audit_analyzer: AuditAnalyzer::new("http://127.0.0.1:9/v1", "test"),
        files,
    }
}
