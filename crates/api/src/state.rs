use audit::{login_logs::LoginLogService, operation_logs::OperationLogService};
use file_storage::files::FileService;
use iam::{
    access::AccessService, departments::DepartmentService, menus::MenuService, roles::RoleService,
    users::UserService,
};
use metadata::{dictionaries::DictionaryService, parameters::ParameterService};

use auth::{captcha::CaptchaService, token::TokenService};

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
    pub login_logs: LoginLogService,
    pub operation_logs: OperationLogService,
    pub files: FileService,
}
