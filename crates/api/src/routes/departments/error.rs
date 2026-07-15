use admin_httpz::{AppError, ErrorSpec};
use iam::departments::DeptError;

const INVALID_PARENT: ErrorSpec =
    ErrorSpec::validation("DEPT_INVALID_PARENT", "invalid department parent");
const DB_FAILED: ErrorSpec = ErrorSpec::internal("DEPT_DB_FAILED", "department operation failed");

pub fn map_error(error: DeptError) -> AppError {
    match error {
        DeptError::Database(source) => DB_FAILED.into_error().with_source(source),
        DeptError::InvalidParent => INVALID_PARENT.into(),
    }
}
