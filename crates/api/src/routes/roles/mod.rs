mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, put},
};
pub(crate) use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handler::get_roles).post(handler::create_role))
        .route(
            "/{id}",
            put(handler::update_role).delete(handler::delete_role),
        )
        .route(
            "/{id}/access",
            get(handler::get_role_access).put(handler::set_role_access),
        )
}

#[cfg(test)]
mod tests {
    use std::{
        net::SocketAddr,
        sync::atomic::{AtomicU8, Ordering},
    };

    use auth::token::TokenService;
    use axum::{
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{
            Method, Request, StatusCode,
            header::{AUTHORIZATION, CONTENT_TYPE},
        },
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;

    static NEXT_TEST_IP: AtomicU8 = AtomicU8::new(80);

    async fn insert_user(pool: &sqlx::PgPool, id: i64, username: &str) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id
            ) values ($1, $2, $2, 'hash', $2, '', 'dashboard', true, 1)
            "#,
        )
        .bind(id)
        .bind(username)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn request_json(
        app: &Router,
        token: &str,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> (StatusCode, Value) {
        let host = NEXT_TEST_IP.fetch_add(1, Ordering::Relaxed);
        let mut request = Request::builder()
            .extension(ConnectInfo(
                format!("192.0.2.{host}:3000")
                    .parse::<SocketAddr>()
                    .unwrap(),
            ))
            .method(method)
            .uri(path)
            .header(AUTHORIZATION, format!("Bearer {token}"));
        let body = match body {
            Some(value) => {
                request = request.header(CONTENT_TYPE, "application/json");
                Body::from(value.to_string())
            }
            None => Body::empty(),
        };
        let response = app
            .clone()
            .oneshot(request.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    async fn app_with_super(pool: &sqlx::PgPool) -> (Router, TokenService, String) {
        insert_user(pool, 9_201, "role-http-super").await;
        sqlx::query(
            "insert into sys_roles (id, code, name, status, sort) values (9202, 'http-role', 'HTTP Role', 'enabled', 20)",
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:9201', 'role:1', '', '', '', '')",
        )
        .execute(pool)
        .await
        .unwrap();
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        let tokens = TokenService::new(
            "test-secret",
            client.get_multiplexed_async_connection().await.unwrap(),
        );
        let token = tokens
            .create_session(9_201, "role-http-super")
            .await
            .unwrap()
            .access_token;
        let mut state = crate::state::tests::test_state(pool.clone()).await;
        state.tokens = tokens.clone();
        (crate::router::router(state), tokens, token)
    }

    #[test]
    fn role_routes_exclude_department_scope_and_reverse_membership() {
        let _ = routes();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_access_route_normalizes_actions_and_protects_super_admin(pool: sqlx::PgPool) {
        let (app, tokens, token) = app_with_super(&pool).await;
        let (status, body) = request_json(
            &app,
            &token,
            Method::PUT,
            "/api/roles/9202/access",
            Some(json!({ "permissions": ["system:user:create"] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], "OK");

        let (status, body) =
            request_json(&app, &token, Method::GET, "/api/roles/9202/access", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body["data"]["permissions"],
            json!(["system:user:create", "system:user:list"])
        );

        let (status, body) = request_json(
            &app,
            &token,
            Method::PUT,
            "/api/roles/1/access",
            Some(json!({ "permissions": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::PRECONDITION_FAILED);
        assert_eq!(body["code"], "ROLE_IMMUTABLE");
        tokens.revoke(&token).await.unwrap();
    }
}
