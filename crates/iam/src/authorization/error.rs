#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("authorization configuration is invalid")]
    Configuration(String),
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization policy operation failed")]
    Policy(#[from] casbin::Error),
    #[error("authorization watcher failed")]
    Watcher(#[from] redis_watcher::WatcherError),
    #[error("authorization watcher could not be installed")]
    WatcherInstallation,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AccountPolicyError {
    #[error("user not found")]
    UserNotFound,
    #[error("only an active super_admin may manage employee access")]
    AccessDenied,
    #[error("selected roles are invalid")]
    InvalidRoleAssignment,
    #[error("authorization administration database operation failed")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(AuthorizationError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RolePolicyError {
    #[error("role not found")]
    RoleNotFound,
    #[error("protected role is immutable")]
    RoleImmutable,
    #[error("only an active super_admin may manage role access")]
    AccessDenied,
    #[error("authorization administration database operation failed")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Authorization(AuthorizationError),
}

impl From<AuthorizationError> for AccountPolicyError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Database(source) => Self::Database(source),
            source @ (AuthorizationError::Configuration(_)
            | AuthorizationError::Policy(_)
            | AuthorizationError::Watcher(_)
            | AuthorizationError::WatcherInstallation) => Self::Authorization(source),
        }
    }
}
