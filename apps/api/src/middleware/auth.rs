use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

use admin_httpz::{AppResult, OptionAppExt};
use system::users::LoginError;

use crate::auth::{errors, session::AuthSessionError};
use crate::state::AppState;

const X_FORWARDED_FOR: &str = "x-forwarded-for";
const USER_AGENT: &str = "user-agent";

pub(crate) fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?
        .trim();
    if token.is_empty() {
        return None;
    }
    Some(token)
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let headers = request.headers();
    let ip = headers
        .get(X_FORWARDED_FOR)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let agent = headers
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let token = extract_bearer_token(headers).ok_or_spec(errors::LOGIN_REQUIRED)?;
    let claims = state
        .auth_session_service
        .decode_active_token(token)
        .await
        .map_err(|error| match error {
            AuthSessionError::Auth(error) => errors::TOKEN_INVALID.into_error().with_source(error),
            AuthSessionError::Revoked => errors::TOKEN_REVOKED.into_error(),
            AuthSessionError::RevocationStoreUnavailable | AuthSessionError::Redis(_) => {
                errors::AUTH_RESOLVE_FAILED.into_error().with_source(error)
            }
        })?;
    let user = match system::users::load_authenticated_user(&state.pool, claims.user_id).await {
        Ok(user) => user,
        Err(LoginError::InvalidCredentials | LoginError::UserNotFound) => {
            return Err(errors::SESSION_INVALID.into());
        }
        Err(LoginError::Disabled) => {
            return Err(system::errors::users::USER_DISABLED.into());
        }
        Err(LoginError::UserAlreadyExists | LoginError::InvalidPassword) => {
            return Err(errors::AUTH_RESOLVE_FAILED.into_error());
        }
        Err(error @ (LoginError::Auth(_) | LoginError::Database(_))) => {
            return Err(errors::AUTH_RESOLVE_FAILED.into_error().with_source(error));
        }
    };
    let user_id = user.id;

    request.extensions_mut().insert(user);

    let response = next.run(request).await;
    let _ = system::logs::create_operation_log(
        &state.pool,
        system::logs::CreateOperationLog {
            ip,
            method,
            path,
            status: response.status().as_u16() as i32,
            agent,
            error_message: String::new(),
            body: String::new(),
            resp: String::new(),
            user_id,
        },
    )
    .await;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn extract_bearer_token_reads_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            "Bearer test.jwt.token".parse().expect("valid header value"),
        );

        assert_eq!(extract_bearer_token(&headers), Some("test.jwt.token"));
    }

    #[test]
    fn extract_bearer_token_rejects_missing_or_invalid_values() {
        let mut headers = HeaderMap::new();
        assert_eq!(extract_bearer_token(&headers), None);

        headers.insert(
            AUTHORIZATION,
            "Basic abc".parse().expect("valid header value"),
        );
        assert_eq!(extract_bearer_token(&headers), None);

        headers.insert(
            AUTHORIZATION,
            "Bearer ".parse().expect("valid header value"),
        );
        assert_eq!(extract_bearer_token(&headers), None);
    }
}
