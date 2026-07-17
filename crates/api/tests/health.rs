use axum::{
    body::{Body, to_bytes},
    http::{Method, Request, header},
};
use iam::{
    access::AccessService, departments::DepartmentService, roles::RoleService, users::UserService,
};
use serde_json::json;
use tower::ServiceExt;

fn test_state() -> api::AppState {
    let database_url = "postgres://postgres:postgres@127.0.0.1/ava";
    let pool = db::DbPool::connect_lazy(database_url).expect("lazy pool should construct");
    let passwords = auth::password::PasswordService::new();
    let tokens = auth::token::TokenService::without_revocation_store("test-secret");
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
