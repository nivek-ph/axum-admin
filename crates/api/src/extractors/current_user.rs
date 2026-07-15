use std::ops::Deref;

use axum::{extract::FromRequestParts, http::request::Parts};
use iam::users::AuthenticatedUser;

use crate::{AppError, mappings::LOGIN_REQUIRED};

#[derive(Debug, Clone)]
pub struct CurrentUser(pub AuthenticatedUser);

impl Deref for CurrentUser {
    type Target = AuthenticatedUser;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(Self)
            .ok_or(LOGIN_REQUIRED)?)
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::FromRequestParts,
        http::{Request, StatusCode},
    };

    use super::*;

    #[tokio::test]
    async fn missing_current_user_extension_requires_login() {
        let (mut parts, _) = Request::new(()).into_parts();

        let error = CurrentUser::from_request_parts(&mut parts, &())
            .await
            .expect_err("missing current user should be rejected");

        assert_eq!(error.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(error.code(), "LOGIN_REQUIRED");
    }
}
