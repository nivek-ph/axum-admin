use admin_httpz::{AppError, ErrorSpec};

const OPERATION_FAILED: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");

pub fn map_error(error: metadata::parameters::ParameterError) -> AppError {
    match error {
        metadata::parameters::ParameterError::Database(source) => {
            OPERATION_FAILED.into_error().with_source(source)
        }
    }
}
