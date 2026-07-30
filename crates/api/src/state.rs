use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, accounts::Accounts, authorization::Authorization,
    departments::DepartmentService, menus::MenuService, roles::RoleService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};

#[derive(Clone)]
pub struct AppState {
    pub public_base_url: String,
    pub tokens: TokenService,
    pub captcha: CaptchaService,
    pub passwords: PasswordService,
    pub accounts: Accounts,
    pub authorization: Authorization,
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
pub(crate) mod tests {
    use super::*;

    pub(crate) async fn test_state(pool: sqlx::PgPool) -> AppState {
        let passwords = auth::password::PasswordService::new();
        let authorization = iam::authorization::Authorization::new(pool.clone());
        let (access, menus) = iam::load_access_and_menus(pool.clone(), authorization.clone())
            .await
            .expect("IAM test state should load the access catalog");
        let audits = AuditService::new(pool.clone());
        let accounts = Accounts::new(pool.clone(), authorization.clone());
        let roles = RoleService::new(pool.clone(), access.clone(), authorization.clone());
        let departments = DepartmentService::new(pool.clone());
        let dictionaries = DictionaryService::new(pool.clone());
        let parameters = ParameterService::new(pool.clone());
        let files = FileService::new(pool, "./uploads");
        AppState {
            public_base_url: "http://127.0.0.1:3000".to_string(),
            tokens: TokenService::without_session_store("test-secret"),
            captcha: CaptchaService::without_store(),
            passwords,
            accounts,
            authorization,
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
}
