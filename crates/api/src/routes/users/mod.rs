pub(crate) mod dto;
mod handler;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub use handler::*;

use crate::{middleware::permission::PermissionRouteExt, state::AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_user_info).put(set_self_info))
        .route("/me/password", put(change_password))
        .route("/me/settings", put(set_self_setting))
        .route(
            "/",
            get(get_user_list_by_query).permission("system:user:list"),
        )
        .route("/", post(admin_register).permission("system:user:create"))
        .route(
            "/{id}",
            put(set_user_info_by_id).permission("system:user:update"),
        )
        .route(
            "/{id}",
            delete(delete_user_by_id).permission("system:user:delete"),
        )
        .route(
            "/{id}/password/reset",
            post(reset_password_by_id).permission("system:user:reset-password"),
        )
        .route(
            "/{id}/roles",
            put(set_user_roles_by_id).permission("system:user:assign-roles"),
        )
        .route(
            "/{id}/access",
            get(get_user_access).permission("system:user:access-read"),
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
        Router,
        body::{Body, to_bytes},
        extract::ConnectInfo,
        http::{
            Method, Request, StatusCode,
            header::{AUTHORIZATION, CONTENT_TYPE},
        },
    };
    use serde_json::{Value, json};
    use tower::ServiceExt;

    static NEXT_TEST_IP: AtomicU8 = AtomicU8::new(1);

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

    async fn insert_policy(pool: &sqlx::PgPool, ptype: &str, left: &str, right: &str) {
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ($1, $2, $3, '', '', '', '')
            "#,
        )
        .bind(ptype)
        .bind(left)
        .bind(right)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn token_service() -> TokenService {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        TokenService::new(
            "test-secret",
            client.get_multiplexed_async_connection().await.unwrap(),
        )
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

    async fn app_with_tokens(pool: &sqlx::PgPool) -> (Router, TokenService, String, String) {
        insert_user(pool, 9_101, "http-super").await;
        insert_user(pool, 9_102, "http-target").await;
        insert_user(pool, 9_103, "http-ordinary").await;
        sqlx::query(
            "insert into sys_roles (id, code, name, status, sort) values (2, 'operator', 'Operator', 'enabled', 10)",
        )
        .execute(pool)
        .await
        .unwrap();
        insert_policy(pool, "g", "user:9101", "role:1").await;
        insert_policy(pool, "g", "user:9103", "role:2").await;
        insert_policy(pool, "p", "role:2", "system:user:list").await;
        insert_policy(pool, "p", "role:2", "system:user:access-read").await;
        let tokens = token_service().await;
        let super_token = tokens
            .create_session(9_101, "http-super")
            .await
            .unwrap()
            .access_token;
        let ordinary_token = tokens
            .create_session(9_103, "http-ordinary")
            .await
            .unwrap()
            .access_token;
        let mut state = crate::state::tests::test_state(pool.clone()).await;
        state.tokens = tokens.clone();
        (
            crate::router::router(state),
            tokens,
            super_token,
            ordinary_token,
        )
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_only_access_routes_replace_and_read_stable_envelopes(pool: sqlx::PgPool) {
        let (app, tokens, super_token, _) = app_with_tokens(&pool).await;
        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            "/api/users/9102/roles",
            Some(json!({ "roleIds": [2] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], "OK");

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::GET,
            "/api/users/9102/access",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["assignedRoles"][0]["id"], 2);
        assert!(body["data"].get("directPermissions").is_none());
        assert!(
            body["data"]["effectivePermissions"][0]
                .get("direct")
                .is_none()
        );

        let (status, _) = request_json(
            &app,
            &super_token,
            Method::PUT,
            "/api/users/9102/permissions",
            Some(json!({ "permissions": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        tokens.revoke(&super_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn concrete_permission_does_not_delegate_access_administration(pool: sqlx::PgPool) {
        let (app, tokens, _, ordinary_token) = app_with_tokens(&pool).await;
        let (status, body) = request_json(
            &app,
            &ordinary_token,
            Method::HEAD,
            "/api/users?page=1&pageSize=10",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(body.is_null(), "HEAD response body must be empty");

        let (status, body) = request_json(
            &app,
            &ordinary_token,
            Method::GET,
            "/api/users/9102/access",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "PERMISSION_DENIED");
        tokens.revoke(&ordinary_token).await.unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn final_super_admin_membership_removal_succeeds(pool: sqlx::PgPool) {
        let (app, tokens, super_token, _) = app_with_tokens(&pool).await;
        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            "/api/users/9101/roles",
            Some(json!({ "roleIds": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], "OK");
        tokens.revoke(&super_token).await.unwrap();
    }
}
