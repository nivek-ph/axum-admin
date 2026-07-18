use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, header},
};
use iam::{
    access::AccessService, departments::DepartmentService, roles::RoleService, users::UserService,
};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::json;
use tower::ServiceExt;

fn test_state() -> api::AppState {
    let database_url = "postgres://postgres:postgres@127.0.0.1/ava";
    let pool = db::DbPool::connect_lazy(database_url).expect("lazy pool should construct");
    let passwords = auth::password::PasswordService::new();
    let tokens = auth::token::TokenService::without_session_store("test-secret");
    let captcha = auth::captcha::CaptchaService::without_store();
    let access = AccessService::new(pool.clone());
    let audits = audit::AuditService::new(pool.clone());
    let users = UserService::new(pool.clone(), access.clone(), audits.clone(), passwords);
    let roles = RoleService::new(pool.clone(), access.clone());
    let departments = DepartmentService::new(pool.clone(), access.clone());
    api::AppState {
        tokens,
        captcha,
        users,
        roles,
        departments,
        access,
        dictionaries: metadata::dictionaries::DictionaryService::new(pool.clone()),
        parameters: metadata::parameters::ParameterService::new(pool.clone()),
        menus: iam::menus::MenuService::new(pool.clone()),
        audits,
        files: file_storage::files::FileService::new(pool, "./uploads"),
    }
}

#[tokio::test]
async fn health_route_returns_ok_response_body() {
    let app = api::router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 200);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("response should be JSON"),
        json!({
            "code": "OK",
            "message": "ok",
            "data": { "alive": true }
        })
    );
}

#[tokio::test]
async fn swagger_ui_route_is_available() {
    let app = api::router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/swagger-ui/")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn protected_route_without_bearer_returns_login_required_envelope() {
    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "LOGIN_REQUIRED",
            "message": "login required",
            "data": null
        })
    );
}

#[tokio::test]
async fn protected_route_with_invalid_bearer_returns_token_invalid_envelope() {
    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .header(header::AUTHORIZATION, "Bearer invalid-token")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "TOKEN_INVALID",
            "message": "session expired",
            "data": null
        })
    );
}

#[tokio::test]
async fn protected_route_with_expired_bearer_returns_access_token_expired_envelope() {
    let token = encode(
        &Header::default(),
        &auth::jwt::Claims {
            user_id: 1,
            username: "admin".to_string(),
            sid: "expired-session".to_string(),
            exp: 1,
        },
        &EncodingKey::from_secret(b"test-secret"),
    )
    .expect("expired token should encode");
    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "ACCESS_TOKEN_EXPIRED",
            "message": "session expired",
            "data": null
        })
    );
}

#[tokio::test]
async fn protected_route_with_missing_login_session_returns_session_invalid_envelope() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let redis = redis::Client::open(redis_url)
        .expect("Redis test client should construct")
        .get_multiplexed_async_connection()
        .await
        .expect("Redis test connection should open");
    let tokens = auth::token::TokenService::new("test-secret", redis);
    let pair = tokens
        .create_session(1, "admin")
        .await
        .expect("login session should be issued");
    let token = pair.access_token;
    tokens
        .revoke(&token)
        .await
        .expect("login session should be removed");
    let mut state = test_state();
    state.tokens = tokens;

    let response = api::router(state)
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "SESSION_INVALID",
            "message": "session expired",
            "data": null
        })
    );
}

#[tokio::test]
async fn protected_route_without_session_store_returns_authorization_unavailable_envelope() {
    let token = auth::jwt::JwtService::new("test-secret")
        .issue_token(1, "admin", "missing-store-session")
        .expect("valid access token should be issued");

    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .uri("/api/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 503);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "AUTHORIZATION_UNAVAILABLE",
            "message": "service unavailable",
            "data": null
        })
    );
}

#[tokio::test]
async fn malformed_refresh_token_returns_refresh_token_invalid_envelope() {
    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"refreshToken":"malformed"}"#))
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "REFRESH_TOKEN_INVALID",
            "message": "session expired",
            "data": null
        })
    );
}

#[tokio::test]
async fn valid_refresh_shape_without_session_store_returns_authorization_unavailable_envelope() {
    let refresh_token = format!("{}.{}", "a".repeat(64), "b".repeat(64));
    let response = api::router(test_state())
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refreshToken": refresh_token }).to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 503);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "AUTHORIZATION_UNAVAILABLE",
            "message": "service unavailable",
            "data": null
        })
    );
}

#[tokio::test]
async fn refresh_for_missing_session_returns_session_invalid_envelope() {
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
    let redis = redis::Client::open(redis_url)
        .expect("Redis test client should construct")
        .get_multiplexed_async_connection()
        .await
        .expect("Redis test connection should open");
    let mut state = test_state();
    state.tokens = auth::token::TokenService::new("test-secret", redis);
    let refresh_token = format!("{}.{}", "c".repeat(64), "d".repeat(64));
    let response = api::router(state)
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "refreshToken": refresh_token }).to_string(),
                ))
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 401);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("body should be json"),
        json!({
            "code": "SESSION_INVALID",
            "message": "session expired",
            "data": null
        })
    );
}

#[tokio::test]
async fn removed_non_core_routes_return_not_found() {
    let app = api::router(test_state());
    let requests = [
        ("/api/autoCode/getDB", "GET"),
        ("/api/customer/customerList?page=1&pageSize=10", "GET"),
        (
            "/api/sysVersion/getSysVersionList?page=1&pageSize=10",
            "GET",
        ),
        ("/api/sysError/getSysErrorList?page=1&pageSize=10", "GET"),
        (
            "/api/sysExportTemplate/getSysExportTemplateList?page=1&pageSize=10",
            "GET",
        ),
        ("/api/sysApiToken/getApiTokenList", "POST"),
    ];

    for (uri, method) in requests {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .expect("request should build");
        let response = app
            .clone()
            .oneshot(request)
            .await
            .expect("router should produce a response");

        assert_eq!(
            response.status(),
            404,
            "route {method} {uri} should be removed"
        );
    }
}

#[tokio::test]
async fn cors_preflight_allows_desktop_dev_origin() {
    let app = api::router(test_state());
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/auth/login")
                .header(header::ORIGIN, "http://localhost:5173")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("router should produce a response");

    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN),
        Some(&header::HeaderValue::from_static("*"))
    );
}
