use axum::{
    extract::{Request, State},
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

use crate::{
    AppResult,
    mappings::{LOGIN_REQUIRED, PERMISSION_DENIED},
    state::AppState,
};
const X_FORWARDED_FOR: &str = "x-forwarded-for";
const USER_AGENT: &str = "user-agent";

pub(crate) fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))?
        .trim();
    (!token.is_empty()).then_some(token)
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> AppResult<Response> {
    let method = request.method().as_str().to_uppercase();
    let path = permission_registry_path(request.uri().path());
    let headers = request.headers();
    let ip = headers
        .get(X_FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let token = extract_bearer_token(headers).ok_or(LOGIN_REQUIRED)?;
    let claims = state.tokens.decode_active(token).await?;
    let snapshot = state.access.snapshot(claims.user_id).await?;

    if !is_self_service_endpoint(&method, &path) {
        let menu_id = state.access.required_menu(&method, &path)?;
        if !snapshot.allows_menu(menu_id) {
            return Err(PERMISSION_DENIED.into());
        }
    }

    let user_id = claims.user_id;
    request
        .extensions_mut()
        .insert(iam::users::AuthenticatedUser {
            id: user_id,
            data_scope: snapshot.data_scope,
        });
    let response = next.run(request).await;
    let _ = state
        .operation_logs
        .record(audit::operation_logs::CreateOperationLog {
            ip,
            method,
            path,
            status: response.status().as_u16() as i32,
            agent,
            error_message: String::new(),
            body: String::new(),
            resp: String::new(),
            user_id,
        })
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

    #[test]
    fn self_service_is_explicit() {
        assert!(is_self_service_endpoint("GET", "/api/users/me"));
        assert!(is_self_service_endpoint("GET", "/api/menus/current"));
        assert!(!is_self_service_endpoint("GET", "/api/users"));
    }
    #[test]
    fn restores_api_prefix() {
        assert_eq!(
            permission_registry_path("/roles/1/menus/"),
            "/api/roles/1/menus"
        );
    }
}
