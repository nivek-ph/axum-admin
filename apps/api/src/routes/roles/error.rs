use admin_httpz::{AppError, ErrorSpec};
use iam::roles::RoleError;

const ROLE_NOT_FOUND: ErrorSpec = ErrorSpec::not_found("ROLE_NOT_FOUND", "role not found");
const ROLE_IMMUTABLE: ErrorSpec =
    ErrorSpec::failed_precondition("ROLE_IMMUTABLE", "system role cannot be deleted");
const ROLE_DB_FAILED: ErrorSpec = ErrorSpec::internal("ROLE_DB_FAILED", "role operation failed");
const ROLE_IN_USE: ErrorSpec = ErrorSpec::conflict("ROLE_IN_USE", "role is assigned to users");
const INVALID_MENU_ASSIGNMENT: ErrorSpec = ErrorSpec::validation(
    "INVALID_MENU_ASSIGNMENT",
    "selected menu nodes must include every ancestor",
);

pub fn map_error(error: RoleError) -> AppError {
    match error {
        RoleError::Database(source) => ROLE_DB_FAILED.into_error().with_source(source),
        RoleError::NotFound => ROLE_NOT_FOUND.into(),
        RoleError::Immutable => ROLE_IMMUTABLE.into(),
        RoleError::InUse => ROLE_IN_USE.into(),
        RoleError::Authorization(source) => ROLE_DB_FAILED.into_error().with_source(source),
        RoleError::InvalidMenuAssignment(source) => {
            INVALID_MENU_ASSIGNMENT.into_error().with_source(source)
        }
    }
}
