use crate::{
    AppError,
    error::{ErrorSpec, error_spec},
};

error_spec! {
    // general
    pub(crate) INTERNAL_SERVER_ERROR, internal, "internal server error";
    pub(crate) RATE_LIMITED, too_many_requests, "too many requests";
    pub(crate) RATE_LIMIT_UNAVAILABLE, service_unavailable, "rate limit service is unavailable";
    // authentication
    pub(crate) LOGIN_REQUIRED, unauthorized, "login required";
    TOKEN_INVALID, unauthorized, "session expired";
    ACCESS_TOKEN_EXPIRED, unauthorized, "session expired";
    REFRESH_TOKEN_INVALID, unauthorized, "session expired";
    SESSION_INVALID, unauthorized, "session expired";
    pub(crate) INVALID_CREDENTIALS, unauthorized, "invalid username or password";
    pub(crate) CAPTCHA_REQUIRED, bad_request, "captcha is required";
    pub(crate) CAPTCHA_INVALID, bad_request, "captcha is invalid or expired";
    // authorization
    PERMISSION_DENIED, forbidden, "permission denied";
    pub(crate) USER_DISABLED, forbidden, "user is disabled";
    AUTHORIZATION_CONFIG_INVALID, internal, "authorization configuration is invalid";
    AUTHORIZATION_UNAVAILABLE, service_unavailable, "authorization service is unavailable";
    // accounts
    USER_NOT_FOUND, not_found, "user not found";
    USER_ALREADY_EXISTS, conflict, "user already exists";
    pub(crate) INVALID_PASSWORD, bad_request, "invalid password";
    // roles
    INVALID_ROLES, validation, "selected roles are invalid";
    ROLE_NOT_FOUND, not_found, "role not found";
    ROLE_IMMUTABLE, failed_precondition, "protected role cannot be changed";
    // audit
    INVALID_AUDIT_TIME_RANGE, validation, "audit time range must use RFC 3339 timestamps";
    // AI
    AI_PROVIDER_UNAVAILABLE, service_unavailable, "local AI provider is unavailable";
    AI_RESPONSE_INVALID, bad_gateway, "local AI provider returned an invalid response";
    // file storage
    FILE_TOO_LARGE, payload_too_large, "uploaded file is too large";
    UPLOAD_NOT_FOUND, not_found, "upload session not found";
    UPLOAD_OFFSET_MISMATCH, conflict, "upload offset does not match";
    UPLOAD_INCOMPLETE, conflict, "upload is incomplete";
    UPLOAD_IN_PROGRESS, conflict, "upload operation is already in progress";
}

impl From<iam::access::AccessEvaluationError> for AppError {
    fn from(error: iam::access::AccessEvaluationError) -> Self {
        use iam::access::AccessEvaluationError;

        match error {
            AccessEvaluationError::Authorization(source) => source.into(),
            AccessEvaluationError::UserNotFound => SESSION_INVALID.into(),
            AccessEvaluationError::UserDisabled => USER_DISABLED.into(),
            AccessEvaluationError::PermissionDenied => PERMISSION_DENIED.into(),
        }
    }
}

impl From<iam::AuthorizationError> for AppError {
    fn from(error: iam::AuthorizationError) -> Self {
        match error {
            iam::AuthorizationError::Configuration(_) => {
                AUTHORIZATION_CONFIG_INVALID.into_error().with_source(error)
            }
            source @ (iam::AuthorizationError::Database(_)
            | iam::AuthorizationError::Policy(_)
            | iam::AuthorizationError::Watcher(_)
            | iam::AuthorizationError::WatcherInstallation) => {
                AUTHORIZATION_UNAVAILABLE.into_error().with_source(source)
            }
        }
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

impl From<iam::accounts::RefreshIdentityError> for AppError {
    fn from(error: iam::accounts::RefreshIdentityError) -> Self {
        use iam::accounts::RefreshIdentityError;

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

impl From<iam::accounts::AccountError> for AppError {
    fn from(error: iam::accounts::AccountError) -> Self {
        use iam::accounts::AccountError;

        match error {
            AccountError::NotFound => USER_NOT_FOUND.into(),
            AccountError::AlreadyExists => USER_ALREADY_EXISTS.into(),
            AccountError::InvalidRoles => INVALID_ROLES.into(),
            AccountError::AccessDenied => PERMISSION_DENIED.into(),
            AccountError::Database(source) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
            AccountError::Authorization(source) => source.into(),
        }
    }
}

impl From<::auth::password::PasswordError> for AppError {
    fn from(error: ::auth::password::PasswordError) -> Self {
        INTERNAL_SERVER_ERROR.into_error().with_source(error)
    }
}

impl From<iam::menus::MenuError> for AppError {
    fn from(error: iam::menus::MenuError) -> Self {
        use iam::menus::MenuError;

        match error {
            MenuError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
            MenuError::Authorization(source) => source.into(),
        }
    }
}

impl From<iam::roles::RoleError> for AppError {
    fn from(error: iam::roles::RoleError) -> Self {
        use iam::roles::RoleError;

        match error {
            RoleError::NotFound => ROLE_NOT_FOUND.into(),
            RoleError::Immutable => ROLE_IMMUTABLE.into(),
            RoleError::HasMembers => ErrorSpec::failed_precondition(
                "ROLE_HAS_MEMBERS",
                "role cannot be deleted while users are assigned",
            )
            .into(),
            RoleError::AccessDenied => PERMISSION_DENIED.into(),
            RoleError::InvalidAccess(source) => ErrorSpec::validation(
                "INVALID_ROLE_ACCESS",
                "selected role access must contain enabled concrete Catalog Permissions",
            )
            .into_error()
            .with_source(source),
            RoleError::Menu(source) => source.into(),
            RoleError::Authorization(source) => source.into(),
            RoleError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
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
            DeptError::HasDescendants { .. } => ErrorSpec::failed_precondition(
                "DEPT_HAS_DESCENDANTS",
                "department has descendant departments",
            )
            .into(),
            DeptError::Database(source) => INTERNAL_SERVER_ERROR.into_error().with_source(source),
        }
    }
}

impl From<file_storage::files::FileError> for AppError {
    fn from(error: file_storage::files::FileError) -> Self {
        use file_storage::files::FileError;

        match error {
            FileError::TooLarge => FILE_TOO_LARGE.into(),
            FileError::UploadNotFound => UPLOAD_NOT_FOUND.into(),
            FileError::OffsetMismatch => UPLOAD_OFFSET_MISMATCH.into(),
            FileError::UploadIncomplete => UPLOAD_INCOMPLETE.into(),
            FileError::UploadInProgress => UPLOAD_IN_PROGRESS.into(),
            source @ FileError::UploadCorrupt => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
            FileError::Storage(source) => source.into(),
            source @ (FileError::Database(_) | FileError::Io(_) | FileError::Adapter(_)) => {
                INTERNAL_SERVER_ERROR.into_error().with_source(source)
            }
        }
    }
}

impl From<file_storage::storages::StorageError> for AppError {
    fn from(error: file_storage::storages::StorageError) -> Self {
        use file_storage::storages::StorageError;

        match error {
            StorageError::NotFound => {
                ErrorSpec::not_found("STORAGE_NOT_FOUND", "storage not found").into()
            }
            StorageError::CodeConflict => {
                ErrorSpec::conflict("STORAGE_CODE_CONFLICT", "storage code already exists").into()
            }
            StorageError::ImmutableIdentity => ErrorSpec::failed_precondition(
                "STORAGE_IDENTITY_IMMUTABLE",
                "storage code, driver, and object location cannot be changed",
            )
            .into(),
            StorageError::DefaultProtected => ErrorSpec::failed_precondition(
                "DEFAULT_STORAGE_PROTECTED",
                "default storage cannot be disabled or deleted",
            )
            .into(),
            StorageError::DisabledDefault => ErrorSpec::failed_precondition(
                "STORAGE_DISABLED",
                "disabled storage cannot become the default",
            )
            .into(),
            StorageError::InUse => ErrorSpec::failed_precondition(
                "STORAGE_IN_USE",
                "storage is referenced by uploaded files or active upload sessions",
            )
            .into(),
            source @ (StorageError::InvalidInput(_) | StorageError::InvalidConfiguration(_)) => {
                ErrorSpec::validation("INVALID_STORAGE", "storage is invalid")
                    .into_error()
                    .with_source(source)
            }
            source @ StorageError::Database(_) => {
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

impl From<audit::AuditAnalysisError> for AppError {
    fn from(error: audit::AuditAnalysisError) -> Self {
        match error {
            audit::AuditAnalysisError::Provider(source) => {
                AI_PROVIDER_UNAVAILABLE.into_error().with_source(source)
            }
            audit::AuditAnalysisError::InvalidResponse(source) => AI_RESPONSE_INVALID
                .into_error()
                .with_source(anyhow::anyhow!(source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

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
    fn authorization_configuration_error_has_a_stable_internal_contract() {
        let error = AppError::from(iam::AuthorizationError::Configuration(
            "invalid test policy".to_string(),
        ));

        assert_eq!(error.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(error.code(), "AUTHORIZATION_CONFIG_INVALID");
        assert_eq!(error.message(), "authorization configuration is invalid");
    }

    #[test]
    fn role_access_management_requires_super_admin_contract() {
        let error = AppError::from(iam::roles::RoleError::AccessDenied);

        assert_eq!(error.status(), StatusCode::FORBIDDEN);
        assert_eq!(error.code(), "PERMISSION_DENIED");
        assert_eq!(error.message(), "permission denied");
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
    fn storage_errors_keep_their_domain_contract_through_file_operations() {
        let error = AppError::from(file_storage::files::FileError::Storage(
            file_storage::storages::StorageError::DefaultProtected,
        ));

        assert_eq!(error.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(error.code(), "DEFAULT_STORAGE_PROTECTED");
        assert_eq!(
            error.message(),
            "default storage cannot be disabled or deleted"
        );
    }

    #[test]
    fn access_evaluation_contract_separates_identity_and_permission_failures() {
        let cases = [
            (
                AppError::from(iam::access::AccessEvaluationError::UserDisabled),
                StatusCode::FORBIDDEN,
                "USER_DISABLED",
            ),
            (
                AppError::from(iam::access::AccessEvaluationError::PermissionDenied),
                StatusCode::FORBIDDEN,
                "PERMISSION_DENIED",
            ),
        ];

        for (error, status, code) in cases {
            assert_eq!(error.status(), status);
            assert_eq!(error.code(), code);
        }
    }

    #[test]
    fn department_with_descendants_has_a_stable_precondition_contract() {
        let error = AppError::from(iam::departments::DeptError::HasDescendants {
            descendant_count: 3,
        });

        assert_eq!(error.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(error.code(), "DEPT_HAS_DESCENDANTS");
    }
}
