use admin_httpz::{AppError, ErrorSpec};

const OPERATION_FAILED: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");
const DETAIL_NOT_FOUND: ErrorSpec =
    ErrorSpec::not_found("DICTIONARY_DETAIL_NOT_FOUND", "dictionary detail not found");
const DICTIONARY_NOT_FOUND: ErrorSpec =
    ErrorSpec::not_found("DICTIONARY_NOT_FOUND", "dictionary not found");

pub fn map_error(error: metadata::dictionaries::DictionaryError) -> AppError {
    match error {
        metadata::dictionaries::DictionaryError::DictionaryNotFound { .. } => {
            DICTIONARY_NOT_FOUND.into()
        }
        metadata::dictionaries::DictionaryError::DetailNotFound { .. } => DETAIL_NOT_FOUND.into(),
        metadata::dictionaries::DictionaryError::Database(source) => {
            OPERATION_FAILED.into_error().with_source(source)
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;

    #[test]
    fn missing_dictionary_detail_maps_to_not_found() {
        let error = map_error(metadata::dictionaries::DictionaryError::DetailNotFound {
            dictionary_id: 1,
            detail_id: 9,
        });

        assert_eq!(error.status(), StatusCode::NOT_FOUND);
        assert_eq!(error.code(), "DICTIONARY_DETAIL_NOT_FOUND");
    }

    #[test]
    fn missing_dictionary_maps_to_not_found() {
        let error = map_error(
            metadata::dictionaries::DictionaryError::DictionaryNotFound { dictionary_id: 7 },
        );

        assert_eq!(error.status(), StatusCode::NOT_FOUND);
        assert_eq!(error.code(), "DICTIONARY_NOT_FOUND");
    }
}
