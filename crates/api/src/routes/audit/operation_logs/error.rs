use admin_httpz::{AppError, ErrorSpec};

const OPERATION_FAILED: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");

pub fn map_error(error: audit::operation_logs::OperationLogError) -> AppError {
    match error {
        audit::operation_logs::OperationLogError::Database(source) => {
            OPERATION_FAILED.into_error().with_source(source)
        }
    }
}
