use audit::{AuditAnalyzer, AuditService};
use auth::{captcha::CaptchaService, password::PasswordService, token::TokenService};
use file_storage::{files::FileService, storages::StorageService};
use iam::{
    access::AccessService, accounts::Accounts, departments::DepartmentService, menus::MenuService,
    roles::RoleService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};
use redis::aio::MultiplexedConnection;

#[derive(Clone)]
pub struct AppState {
    pub redis: MultiplexedConnection,
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
    pub storages: StorageService,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) async fn test_state(pool: sqlx::PgPool) -> AppState {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        let passwords = auth::password::PasswordService::new();
        let iam = iam::Iam::load(pool.clone())
            .await
            .expect("IAM test state should load");
        let audits = AuditService::new(pool.clone());
        let departments = DepartmentService::new(pool.clone());
        let dictionaries = DictionaryService::new(pool.clone());
        let parameters = ParameterService::new(pool.clone());
        let (files, storages) = FileService::managed(pool)
            .await
            .expect("test storage should load");
        AppState {
            redis,
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
            storages,
        }
    }
}
