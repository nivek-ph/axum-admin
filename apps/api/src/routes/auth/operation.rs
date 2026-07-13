use audit::login_logs::{CreateLoginLog, service::LoginLogService};
use auth::session::{AuthSessionError, AuthSessionService};
use iam::{users, users::service::UserService};

#[derive(Debug)]
pub struct LoginInput {
    pub username: String,
    pub password: String,
    pub captcha: String,
    pub captcha_id: String,
    pub ip: String,
    pub agent: String,
}

#[derive(Debug)]
pub struct LoginOutput {
    pub user: users::UserInfoView,
    pub token: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LoginOperationError {
    #[error("captcha is required")]
    CaptchaRequired,
    #[error("captcha is invalid or expired")]
    CaptchaInvalid,
    #[error("captcha operation failed")]
    Captcha(#[source] AuthSessionError),
    #[error("{0}")]
    Identity(#[source] users::LoginError),
    #[error("session operation failed")]
    Session(#[source] AuthSessionError),
}

#[derive(Clone)]
pub struct LoginOperation {
    users: UserService,
    sessions: AuthSessionService,
    audit: Option<LoginLogService>,
}

impl LoginOperation {
    pub fn new(users: UserService, sessions: AuthSessionService, audit: LoginLogService) -> Self {
        Self {
            users,
            sessions,
            audit: Some(audit),
        }
    }

    pub async fn execute(&self, input: LoginInput) -> Result<LoginOutput, LoginOperationError> {
        if input.captcha.trim().is_empty() || input.captcha_id.trim().is_empty() {
            self.record(&input, false, "captcha is required", None)
                .await;
            return Err(LoginOperationError::CaptchaRequired);
        }

        let captcha_valid = match self
            .sessions
            .verify_captcha(&input.captcha_id, &input.captcha)
            .await
        {
            Ok(valid) => valid,
            Err(error) => {
                self.record(&input, false, "captcha operation failed", None)
                    .await;
                return Err(LoginOperationError::Captcha(error));
            }
        };
        if !captcha_valid {
            self.record(&input, false, "captcha is invalid or expired", None)
                .await;
            return Err(LoginOperationError::CaptchaInvalid);
        }

        let identity = match self
            .users
            .authenticate(users::LoginRequest {
                username: input.username.clone(),
                password: input.password.clone(),
            })
            .await
        {
            Ok(identity) => identity,
            Err(error) => {
                self.record(&input, false, &error.to_string(), None).await;
                return Err(LoginOperationError::Identity(error));
            }
        };

        // Audit persistence is intentionally best effort: an unavailable audit store must not
        // turn valid credentials into a failed login response.
        self.record(&input, true, "login succeeded", Some(identity.user.id))
            .await;

        let token = self
            .sessions
            .issue_token(identity.id, &identity.username)
            .map_err(LoginOperationError::Session)?;

        Ok(LoginOutput {
            user: identity.user,
            token,
        })
    }

    async fn record(&self, input: &LoginInput, status: bool, message: &str, user_id: Option<i64>) {
        let Some(audit) = &self.audit else { return };
        let _ = audit
            .record(CreateLoginLog {
                username: input.username.clone(),
                ip: input.ip.clone(),
                status,
                error_message: message.to_string(),
                agent: input.agent.clone(),
                user_id,
            })
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use auth::password::PasswordService;

    #[tokio::test]
    async fn missing_captcha_is_rejected_before_identity_or_session_access() {
        let pool =
            db::DbPool::connect_lazy("postgres://postgres:postgres@localhost/axum_vue_admin")
                .expect("lazy pool should construct");
        let operation = LoginOperation {
            users: UserService::new(pool, PasswordService::new()),
            sessions: AuthSessionService::without_revocation_store("test-secret"),
            audit: None,
        };

        let error = operation
            .execute(LoginInput {
                username: "admin".to_string(),
                password: "secret".to_string(),
                captcha: String::new(),
                captcha_id: String::new(),
                ip: String::new(),
                agent: String::new(),
            })
            .await
            .expect_err("missing captcha should fail");

        assert!(matches!(error, LoginOperationError::CaptchaRequired));
    }
}
