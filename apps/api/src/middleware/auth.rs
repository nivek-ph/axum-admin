use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

use admin_httpz::{AppResult, OptionAppExt};
use system::users::LoginError;

use crate::errors::auth::{
    self as errors, AUTH_RESOLVE_FAILED, PERMISSION_DENIED, SESSION_INVALID,
};
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
    let permission_path = permission_registry_path(&path);
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
        .await?;
    let user = system::users::load_authenticated_user(&state.pool, claims.user_id)
        .await
        .map_err(|error| match error {
            LoginError::InvalidCredentials | LoginError::UserNotFound => SESSION_INVALID.into(),
            LoginError::Disabled => system::errors::users::USER_DISABLED.into(),
            LoginError::UserAlreadyExists | LoginError::InvalidPassword => {
                AUTH_RESOLVE_FAILED.into_error()
            }
            LoginError::Auth(_) | LoginError::Database(_) => {
                AUTH_RESOLVE_FAILED.into_error().with_source(error)
            }
        })?;
    let user_id = user.id;

    let has_super_admin_role =
        system::roles::user_has_role_code(&state.pool, user.id, "super_admin").await?;
    let is_super_admin = is_super_admin_identity(has_super_admin_role);

    if !is_super_admin && !is_self_service_endpoint(&method, &permission_path) {
        let required_permission = system::permission_apis::resolve_required_permission(
            &state.pool,
            &method,
            &permission_path,
        )
        .await?;

        let Some(permission_code) = required_permission else {
            return Err(PERMISSION_DENIED.into());
        };

        let allowed =
            system::permissions::user_has_permission(&state.pool, user.id, &permission_code)
                .await?;

        if !allowed {
            return Err(PERMISSION_DENIED.into());
        }
    }

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

fn is_self_service_endpoint(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/api/users/me")
            | ("PUT", "/api/users/me")
            | ("PUT", "/api/users/me/password")
            | ("PUT", "/api/users/me/settings")
            | ("GET", "/api/menus/current")
            | ("POST", "/api/auth/logout")
    )
}

fn is_super_admin_identity(has_super_admin_role: bool) -> bool {
    has_super_admin_role
}

fn permission_registry_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    let normalized = if trimmed.is_empty() { "/api" } else { trimmed };

    if normalized == "/api" || normalized.starts_with("/api/") {
        normalized.to_string()
    } else if normalized.starts_with('/') {
        format!("/api{normalized}")
    } else {
        format!("/api/{normalized}")
    }
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

    #[test]
    fn self_service_endpoints_remain_explicitly_whitelisted() {
        assert!(is_self_service_endpoint("GET", "/api/users/me"));
        assert!(is_self_service_endpoint("PUT", "/api/users/me"));
        assert!(is_self_service_endpoint("GET", "/api/menus/current"));
        assert!(is_self_service_endpoint("POST", "/api/auth/logout"));
        assert!(!is_self_service_endpoint("GET", "/api/users"));
        assert!(!is_self_service_endpoint("PUT", "/api/users/me/authority",));
    }

    #[test]
    fn super_admin_identity_uses_role_code() {
        assert!(is_super_admin_identity(true));
        assert!(!is_super_admin_identity(false));
    }

    #[test]
    fn permission_registry_path_restores_nested_api_prefix() {
        assert_eq!(
            permission_registry_path("/menus/1/roles"),
            "/api/menus/1/roles"
        );
        assert_eq!(
            permission_registry_path("/api/menus/current"),
            "/api/menus/current"
        );
    }

    #[test]
    fn permission_registry_path_trims_trailing_slashes() {
        assert_eq!(permission_registry_path("/api/users/me/"), "/api/users/me");
        assert!(is_self_service_endpoint(
            "GET",
            &permission_registry_path("/api/menus/current/")
        ));
    }
}
