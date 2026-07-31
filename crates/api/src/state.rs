use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, accounts::Accounts, departments::DepartmentService, menus::MenuService,
    roles::RoleService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};

#[derive(Clone)]
pub struct AppState {
    pub public_base_url: String,
    pub tokens: TokenService,
    pub captcha: CaptchaService,
    pub passwords: PasswordService,
    pub accounts: Accounts,
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
        let iam = iam::Iam::load(pool.clone())
            .await
            .expect("IAM test state should load");
        let audits = AuditService::new(pool.clone());
        let departments = DepartmentService::new(pool.clone());
        let dictionaries = DictionaryService::new(pool.clone());
        let parameters = ParameterService::new(pool.clone());
        let files = FileService::new(pool, "./uploads");
        AppState {
            public_base_url: "http://127.0.0.1:3000".to_string(),
            tokens: TokenService::without_session_store("test-secret"),
            captcha: CaptchaService::without_store(),
            passwords,
            accounts: iam.accounts,
            roles: iam.roles,
            departments,
            access: iam.access,
            dictionaries,
            parameters,
            menus: iam.menus,
            audits,
            audit_analyzer: AuditAnalyzer::new("http://127.0.0.1:9/v1", "test"),
            files,
        }
    }
}
