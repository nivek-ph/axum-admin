use std::ops::Deref;

use axum::{extract::FromRequestParts, http::request::Parts};

use admin_httpz::{AppError, OptionAppExt};
use iam::users::AuthenticatedUser;

use crate::errors::auth as errors;

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
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .map(Self)
            .ok_or_spec(errors::LOGIN_REQUIRED)
    }
}
