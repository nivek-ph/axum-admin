use admin_httpz::{AppError, ErrorSpec};
use file_storage::files::FileError;

const DB_FAILED: ErrorSpec = ErrorSpec::internal("FILE_DB_FAILED", "file operation failed");
const IO_FAILED: ErrorSpec = ErrorSpec::internal("FILE_IO_FAILED", "file operation failed");

pub fn map_error(error: FileError) -> AppError {
    match error {
        FileError::Database(source) => DB_FAILED.into_error().with_source(source),
        FileError::Io(source) => IO_FAILED.into_error().with_source(source),
    }
}
