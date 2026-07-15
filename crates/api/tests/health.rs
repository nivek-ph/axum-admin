use axum::{
    body::Body,
    http::{Method, Request, header},
};
use tower::ServiceExt;

fn test_state() -> api::AppState {
    let database_url = "postgres://postgres:postgres@localhost/axum_vue_admin";
    let pool = db::DbPool::connect_lazy(database_url).expect("lazy pool should construct");
    let passwords = auth::password::PasswordService::new();
    let tokens = auth::token::TokenService::without_revocation_store("test-secret");
    let captcha = auth::captcha::CaptchaService::without_store();
    let users = iam::users::UserService::new(pool.clone(), passwords);
    let login_logs = audit::login_logs::LoginLogService::new(pool.clone());
    let operation_logs = audit::operation_logs::OperationLogService::new(pool.clone());
    api::AppState {
        tokens,
        captcha,
        users,
        roles: iam::roles::RoleService::new(pool.clone()),
        departments: iam::departments::DepartmentService::new(pool.clone()),
        access: iam::access::AccessService::new(pool.clone()),
        dictionaries: metadata::dictionaries::DictionaryService::new(pool.clone()),
        parameters: metadata::parameters::ParameterService::new(pool.clone()),
        menus: iam::menus::MenuService::new(
            pool.clone(),
            iam::access::AccessService::new(pool.clone()),
        ),
        login_logs,
        operation_logs,
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
