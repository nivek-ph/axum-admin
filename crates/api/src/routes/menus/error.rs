use admin_httpz::{AppError, ErrorSpec};
use iam::menus::MenuError;

const NOT_FOUND: ErrorSpec = ErrorSpec::not_found("MENU_NOT_FOUND", "menu not found");
const INVALID_PAYLOAD: ErrorSpec =
    ErrorSpec::validation("MENU_INVALID_PAYLOAD", "invalid menu payload");
const DB_FAILED: ErrorSpec = ErrorSpec::internal("MENU_DB_FAILED", "menu operation failed");

pub fn map_error(error: MenuError) -> AppError {
    match error {
        MenuError::NotFound => NOT_FOUND.into(),
        MenuError::Database(source) => DB_FAILED.into_error().with_source(source),
        MenuError::Access(source) => DB_FAILED.into_error().with_source(source),
        MenuError::InvalidPayload => INVALID_PAYLOAD.into(),
    }
}
