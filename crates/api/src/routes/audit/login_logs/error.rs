use admin_httpz::{AppError, ErrorSpec};

const OPERATION_FAILED: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");

pub fn map_error(error: audit::login_logs::LoginLogError) -> AppError {
    match error {
        audit::login_logs::LoginLogError::Database(source) => {
            OPERATION_FAILED.into_error().with_source(source)
        }
    }
}
