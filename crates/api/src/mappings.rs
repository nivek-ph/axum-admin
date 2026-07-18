use axum::http::StatusCode;

use crate::{AppError, error::ErrorSpec};

const INTERNAL_SERVER_ERROR: ErrorSpec =
    ErrorSpec::internal("INTERNAL_SERVER_ERROR", "internal server error");
pub(crate) const LOGIN_REQUIRED: ErrorSpec =
    ErrorSpec::unauthorized("LOGIN_REQUIRED", "login required");
const TOKEN_INVALID: ErrorSpec = ErrorSpec::unauthorized("TOKEN_INVALID", "session expired");
const ACCESS_TOKEN_EXPIRED: ErrorSpec =
    ErrorSpec::unauthorized("ACCESS_TOKEN_EXPIRED", "session expired");
const REFRESH_TOKEN_INVALID: ErrorSpec =
    ErrorSpec::unauthorized("REFRESH_TOKEN_INVALID", "session expired");
const SESSION_INVALID: ErrorSpec = ErrorSpec::unauthorized("SESSION_INVALID", "session expired");
pub(crate) const PERMISSION_DENIED: ErrorSpec =
    ErrorSpec::forbidden("PERMISSION_DENIED", "permission denied");
const AUTHORIZATION_CONFIG_INVALID: ErrorSpec = ErrorSpec::internal(
    "AUTHORIZATION_CONFIG_INVALID",
    "authorization configuration is invalid",
);
const AUTHORIZATION_UNAVAILABLE: ErrorSpec = ErrorSpec::new(
    StatusCode::SERVICE_UNAVAILABLE,
    "AUTHORIZATION_UNAVAILABLE",
    "authorization service is unavailable",
);
pub(crate) const CAPTCHA_REQUIRED: ErrorSpec =
    ErrorSpec::bad_request("CAPTCHA_REQUIRED", "captcha is required");
pub(crate) const CAPTCHA_INVALID: ErrorSpec =
    ErrorSpec::bad_request("CAPTCHA_INVALID", "captcha is invalid or expired");
const INVALID_CREDENTIALS: ErrorSpec =
    ErrorSpec::unauthorized("INVALID_CREDENTIALS", "invalid username or password");
const USER_DISABLED: ErrorSpec = ErrorSpec::forbidden("USER_DISABLED", "user is disabled");
const USER_NOT_FOUND: ErrorSpec = ErrorSpec::not_found("USER_NOT_FOUND", "user not found");
const USER_ALREADY_EXISTS: ErrorSpec =
    ErrorSpec::conflict("USER_ALREADY_EXISTS", "user already exists");
const INVALID_PASSWORD: ErrorSpec = ErrorSpec::bad_request("INVALID_PASSWORD", "invalid password");
const INVALID_ROLES: ErrorSpec =
    ErrorSpec::validation("INVALID_ROLES", "at least one enabled role is required");
const INVALID_AUDIT_TIME_RANGE: ErrorSpec = ErrorSpec::validation(
    "INVALID_AUDIT_TIME_RANGE",
    "audit time range must use RFC 3339 timestamps",
);
const MULTIPART_FIELD_FAILED: ErrorSpec =
    ErrorSpec::bad_request("MULTIPART_FIELD_FAILED", "failed to read upload content");
pub(crate) const MULTIPLE_FILES_NOT_SUPPORTED: ErrorSpec = ErrorSpec::bad_request(
    "MULTIPLE_FILES_NOT_SUPPORTED",
    "only one file can be uploaded at a time",
);
const FILE_TOO_LARGE: ErrorSpec = ErrorSpec::new(
    StatusCode::PAYLOAD_TOO_LARGE,
    "FILE_TOO_LARGE",
    "uploaded file is too large",
);

impl From<axum::extract::multipart::MultipartError> for AppError {
    fn from(error: axum::extract::multipart::MultipartError) -> Self {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            FILE_TOO_LARGE.into_error().with_source(error)
        } else {
            MULTIPART_FIELD_FAILED.into_error().with_source(error)
        }
    }
}

impl From<iam::access::AccessEvaluationError> for AppError {
    fn from(error: iam::access::AccessEvaluationError) -> Self {
        use iam::access::AccessEvaluationError;

        match error {
            AccessEvaluationError::UserNotFound => SESSION_INVALID.into(),
            AccessEvaluationError::UserDisabled => USER_DISABLED.into(),
            AccessEvaluationError::Cache(source) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(source)
            }
            AccessEvaluationError::Catalog(source) => AUTHORIZATION_CONFIG_INVALID
                .into_error()
                .with_source(source),
            AccessEvaluationError::Database(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
            AccessEvaluationError::Serialization(source) => AUTHORIZATION_CONFIG_INVALID
                .into_error()
                .with_source(source),
        }
    }
}

impl From<iam::access::AccessPropagationError> for AppError {
    fn from(error: iam::access::AccessPropagationError) -> Self {
        INTERNAL_SERVER_ERROR.into_error().with_source(error)
    }
}

impl From<::auth::captcha::CaptchaError> for AppError {
    fn from(error: ::auth::captcha::CaptchaError) -> Self {
        INTERNAL_SERVER_ERROR.into_error().with_source(error)
    }
}

impl From<::auth::token::TokenIssueError> for AppError {
    fn from(error: ::auth::token::TokenIssueError) -> Self {
        use ::auth::token::TokenIssueError;

        match error {
            TokenIssueError::Signing(_) => INTERNAL_SERVER_ERROR.into_error().with_source(error),
            TokenIssueError::StoreUnavailable | TokenIssueError::Store(_) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
            }
        }
    }
}

impl From<::auth::token::TokenSessionError> for AppError {
    fn from(error: ::auth::token::TokenSessionError) -> Self {
        use ::auth::token::TokenSessionError;

        match error {
            TokenSessionError::Expired(source) => {
                ACCESS_TOKEN_EXPIRED.into_error().with_source(source)
            }
            TokenSessionError::Invalid(source) => TOKEN_INVALID.into_error().with_source(source),
            TokenSessionError::SessionInvalid => SESSION_INVALID.into(),
            TokenSessionError::StoreUnavailable | TokenSessionError::Store(_) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
            }
        }
    }
}

impl From<::auth::token::RefreshError> for AppError {
    fn from(error: ::auth::token::RefreshError) -> Self {
        use ::auth::token::RefreshError;

        match error {
            RefreshError::Invalid => REFRESH_TOKEN_INVALID.into(),
            RefreshError::SessionInvalid => SESSION_INVALID.into(),
            RefreshError::Signing(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            RefreshError::StoreUnavailable | RefreshError::Store(_) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
            }
        }
    }
}

impl From<iam::users::RefreshIdentityError> for AppError {
    fn from(error: iam::users::RefreshIdentityError) -> Self {
        use iam::users::RefreshIdentityError;

        match error {
            RefreshIdentityError::NotFound => SESSION_INVALID.into(),
            RefreshIdentityError::Disabled => USER_DISABLED.into(),
            RefreshIdentityError::Database(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
        }
    }
}

impl From<::auth::token::TokenRevokeError> for AppError {
    fn from(error: ::auth::token::TokenRevokeError) -> Self {
        use ::auth::token::TokenRevokeError;

        match error {
            TokenRevokeError::Invalid(source) => TOKEN_INVALID.into_error().with_source(source),
            TokenRevokeError::StoreUnavailable | TokenRevokeError::Store(_) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
            }
        }
    }
}

impl From<::auth::token::UserSessionRevokeError> for AppError {
    fn from(error: ::auth::token::UserSessionRevokeError) -> Self {
        AUTHORIZATION_UNAVAILABLE.into_error().with_source(error)
    }
}

impl From<iam::users::UserError> for AppError {
    fn from(error: iam::users::UserError) -> Self {
        use iam::users::UserError;

        match error {
            UserError::NotFound => USER_NOT_FOUND.into(),
            UserError::AlreadyExists => USER_ALREADY_EXISTS.into(),
            UserError::InvalidPassword => INVALID_PASSWORD.into(),
            UserError::InvalidRoles => INVALID_ROLES.into(),
            UserError::Password(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            UserError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            UserError::Audit(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            UserError::AccessPropagation(source) => source.into(),
        }
    }
}

impl From<iam::users::AuthenticateError> for AppError {
    fn from(error: iam::users::AuthenticateError) -> Self {
        use iam::users::AuthenticateError;

        match error {
            AuthenticateError::InvalidCredentials => INVALID_CREDENTIALS.into(),
            AuthenticateError::Disabled => USER_DISABLED.into(),
            AuthenticateError::Credential(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
            AuthenticateError::Database(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
        }
    }
}

impl From<iam::menus::MenuError> for AppError {
    fn from(error: iam::menus::MenuError) -> Self {
        use iam::menus::MenuError;

        match error {
            MenuError::NotFound => ErrorSpec::not_found("MENU_NOT_FOUND", "menu not found").into(),
            MenuError::InvalidPayload => {
                ErrorSpec::validation("MENU_INVALID_PAYLOAD", "invalid menu payload").into()
            }
            MenuError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
        }
    }
}

impl From<iam::roles::RoleError> for AppError {
    fn from(error: iam::roles::RoleError) -> Self {
        use iam::roles::RoleError;

        match error {
            RoleError::NotFound => ErrorSpec::not_found("ROLE_NOT_FOUND", "role not found").into(),
            RoleError::Immutable => {
                ErrorSpec::failed_precondition("ROLE_IMMUTABLE", "system role cannot be deleted")
                    .into()
            }
            RoleError::InUse => {
                ErrorSpec::conflict("ROLE_IN_USE", "role is assigned to users").into()
            }
            RoleError::InvalidMenuAssignment(source) => ErrorSpec::validation(
                "INVALID_MENU_ASSIGNMENT",
                "selected menu nodes must include every ancestor",
            )
            .into_error()
            .with_source(source),
            RoleError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            RoleError::AccessPropagation(source) => source.into(),
        }
    }
}

impl From<iam::departments::DeptError> for AppError {
    fn from(error: iam::departments::DeptError) -> Self {
        use iam::departments::DeptError;

        match error {
            DeptError::InvalidParent => {
                ErrorSpec::validation("DEPT_INVALID_PARENT", "invalid department parent").into()
            }
            DeptError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            DeptError::AccessPropagation(source) => source.into(),
        }
    }
}

impl From<file_storage::files::FileError> for AppError {
    fn from(error: file_storage::files::FileError) -> Self {
        use file_storage::files::FileError;

        match error {
            FileError::TooLarge => FILE_TOO_LARGE.into(),
            source @ (FileError::Database(_) | FileError::Io(_)) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
        }
    }
}

impl From<metadata::dictionaries::DictionaryError> for AppError {
    fn from(error: metadata::dictionaries::DictionaryError) -> Self {
        use metadata::dictionaries::DictionaryError;

        match error {
            DictionaryError::DictionaryNotFound { .. } => {
                ErrorSpec::not_found("DICTIONARY_NOT_FOUND", "dictionary not found").into()
            }
            DictionaryError::DetailNotFound { .. } => {
                ErrorSpec::not_found("DICTIONARY_DETAIL_NOT_FOUND", "dictionary detail not found")
                    .into()
            }
            DictionaryError::InvalidParent { .. } => ErrorSpec::validation(
                "INVALID_DICTIONARY_PARENT",
                "dictionary parent cannot be the node or its descendant",
            )
            .into(),
            DictionaryError::Database(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
        }
    }
}

impl From<metadata::parameters::ParameterError> for AppError {
    fn from(error: metadata::parameters::ParameterError) -> Self {
        INTERNAL_SERVER_ERROR.into_error().with_source(error)
    }
}

impl From<audit::AuditError> for AppError {
    fn from(error: audit::AuditError) -> Self {
        match error {
            audit::AuditError::InvalidTimeRange(source) => {
                INVALID_AUDIT_TIME_RANGE.into_error().with_source(source)
            }
            audit::AuditError::Database(_) | audit::AuditError::Serialization(_) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capability_storage_failure_keeps_the_internal_error_contract() {
        let Err(capability_error) = ::auth::captcha::CaptchaService::without_store()
            .create()
            .await
        else {
            panic!("captcha creation should require its store");
        };
        let error = AppError::from(capability_error);

        assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.code(), "INTERNAL_SERVER_ERROR");
        assert_eq!(error.message(), "internal server error");
    }

    #[test]
    fn authorization_store_unavailable_remains_service_unavailable() {
        let error = AppError::from(::auth::token::TokenSessionError::StoreUnavailable);

        assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code(), "AUTHORIZATION_UNAVAILABLE");
        assert_eq!(error.message(), "authorization service is unavailable");
    }

    #[test]
    fn token_issue_store_unavailable_is_service_unavailable() {
        let error = AppError::from(::auth::token::TokenIssueError::StoreUnavailable);

        assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code(), "AUTHORIZATION_UNAVAILABLE");
        assert_eq!(error.message(), "authorization service is unavailable");
    }

    #[test]
    fn user_session_revoke_store_unavailable_is_service_unavailable() {
        let error = AppError::from(::auth::token::UserSessionRevokeError::StoreUnavailable);

        assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code(), "AUTHORIZATION_UNAVAILABLE");
        assert_eq!(error.message(), "authorization service is unavailable");
    }

    #[test]
    fn missing_login_session_has_a_stable_unauthorized_contract() {
        let error = AppError::from(::auth::token::TokenSessionError::SessionInvalid);

        assert_eq!(error.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(error.code(), "SESSION_INVALID");
        assert_eq!(error.message(), "session expired");
    }

    #[test]
    fn oversized_upload_has_a_stable_payload_too_large_contract() {
        let error = AppError::from(file_storage::files::FileError::TooLarge);

        assert_eq!(error.status(), StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(error.code(), "FILE_TOO_LARGE");
        assert_eq!(error.message(), "uploaded file is too large");
    }

    #[test]
    fn invalid_authorization_cache_payload_keeps_its_stable_code() {
        let source =
            serde_json::from_str::<serde_json::Value>("{").expect_err("invalid json should fail");
        let error = AppError::from(iam::access::AccessEvaluationError::Serialization(source));

        assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.code(), "AUTHORIZATION_CONFIG_INVALID");
    }

    #[test]
    fn access_evaluation_cache_failure_remains_service_unavailable() {
        let source = redis::RedisError::from((redis::ErrorKind::Io, "cache unavailable"));
        let error = AppError::from(iam::access::AccessEvaluationError::Cache(source));

        assert_eq!(error.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(error.code(), "AUTHORIZATION_UNAVAILABLE");
    }

    #[tokio::test]
    async fn access_evaluation_contract_covers_user_catalog_and_database_failures() {
        let pool = sqlx::PgPool::connect_lazy("postgres://postgres:postgres@127.0.0.1/ava")
            .expect("lazy pool should construct");
        let catalog_error = iam::access::AccessService::new(pool)
            .required_menu("GET", "/api/unbound")
            .expect_err("an empty catalog should reject an unbound route");

        let cases = [
            (
                AppError::from(iam::access::AccessEvaluationError::UserDisabled),
                StatusCode::FORBIDDEN,
                "USER_DISABLED",
            ),
            (
                AppError::from(catalog_error),
                StatusCode::INTERNAL_SERVER_ERROR,
                "AUTHORIZATION_CONFIG_INVALID",
            ),
            (
                AppError::from(iam::access::AccessEvaluationError::Database(
                    sqlx::Error::PoolTimedOut,
                )),
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_SERVER_ERROR",
            ),
        ];

        for (error, status, code) in cases {
            assert_eq!(error.status(), status);
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn access_propagation_failure_is_internal_for_every_mutation_capability() {
        fn propagation_error() -> iam::access::AccessPropagationError {
            iam::access::AccessPropagationError::Cache(redis::RedisError::from((
                redis::ErrorKind::Io,
                "cache unavailable",
            )))
        }

        let user_error =
            AppError::from(iam::users::UserError::AccessPropagation(propagation_error()));
        let role_error =
            AppError::from(iam::roles::RoleError::AccessPropagation(propagation_error()));
        let department_error = AppError::from(iam::departments::DeptError::AccessPropagation(
            propagation_error(),
        ));

        for error in [user_error, role_error, department_error] {
            assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(error.code(), "INTERNAL_SERVER_ERROR");
        }
    }
}
