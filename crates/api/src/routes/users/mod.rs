pub(crate) mod dto;
mod handler;

use axum::{
    Router,
    routing::{get, post, put},
};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_user_info).put(set_self_info))
        .route("/me/password", put(change_password))
        .route("/me/settings", put(set_self_setting))
        .route("/", get(get_user_list_by_query).post(admin_register))
        .route("/{id}", put(set_user_info_by_id).delete(delete_user_by_id))
        .route("/{id}/password/reset", post(reset_password_by_id))
        .route("/{id}/roles", put(set_user_roles_by_id))
        .route(
            "/{id}/permissions",
            get(get_user_access).put(set_user_permissions),
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
        .expect("test user should insert");
    }

    async fn insert_policy(pool: &sqlx::PgPool, ptype: &str, subject: &str, value: &str) {
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ($1, $2, $3, '', '', '', '')
            "#,
        )
        .bind(ptype)
        .bind(subject)
        .bind(value)
        .execute(pool)
        .await
        .expect("test policy should insert");
    }

    async fn token_service() -> TokenService {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).expect("Redis test client should construct");
        TokenService::new(
            "test-secret",
            client
                .get_multiplexed_async_connection()
                .await
                .expect("Redis test connection should open"),
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
                    .expect("test peer address should be valid"),
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
            .oneshot(request.body(body).expect("request should build"))
            .await
            .expect("router should respond");
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        (
            status,
            serde_json::from_slice(&body).expect("response body should be JSON"),
        )
    }

    async fn access_test_app(
        pool: &sqlx::PgPool,
    ) -> (Router, TokenService, String, String, i64, i64) {
        const SUPER_USER_ID: i64 = 9_101;
        const TARGET_USER_ID: i64 = 9_102;
        const RESTRICTED_USER_ID: i64 = 9_103;

        insert_user(pool, SUPER_USER_ID, "http-super").await;
        insert_user(pool, TARGET_USER_ID, "http-target").await;
        insert_user(pool, RESTRICTED_USER_ID, "http-restricted").await;
        sqlx::query(
            "insert into sys_roles (id, code, name, status, sort) values (2, 'developer', 'Developer', 'enabled', 10)",
        )
        .execute(pool)
        .await
        .expect("test role should insert");
        insert_policy(pool, "g", "user:9101", "role:1").await;
        insert_policy(pool, "g", "user:9102", "role:2").await;
        insert_policy(pool, "p", "role:2", "system:user:update").await;
        insert_policy(pool, "p", "user:9103", "system:user:permissions-read").await;
        insert_policy(pool, "p", "user:9103", "system:user:list").await;

        let tokens = token_service().await;
        let super_token = tokens
            .create_session(SUPER_USER_ID, "http-super")
            .await
            .expect("super session should be issued")
            .access_token;
        let restricted_token = tokens
            .create_session(RESTRICTED_USER_ID, "http-restricted")
            .await
            .expect("restricted session should be issued")
            .access_token;
        let mut state = crate::state::tests::test_state(pool.clone()).await;
        state.tokens = tokens.clone();

        (
            crate::router::router(state),
            tokens,
            super_token,
            restricted_token,
            SUPER_USER_ID,
            TARGET_USER_ID,
        )
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn employee_access_routes_replace_and_read_the_updated_access(pool: sqlx::PgPool) {
        let (app, tokens, super_token, restricted_token, _, target_user_id) =
            access_test_app(&pool).await;

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            &format!("/api/users/{target_user_id}/roles"),
            Some(json!({ "roleIds": [2] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            json!({ "code": "OK", "message": "roles updated", "data": null })
        );

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            &format!("/api/users/{target_user_id}/permissions"),
            Some(json!({ "permissions": ["system:dashboard:view"] })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            body,
            json!({
                "code": "OK",
                "message": "direct permissions updated",
                "data": null
            })
        );

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::GET,
            &format!("/api/users/{target_user_id}/permissions"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["code"], "OK");
        assert_eq!(body["message"], "ok");
        assert_eq!(body["data"]["roleIds"], json!([2]));
        assert_eq!(
            body["data"]["directPermissions"],
            json!(["system:dashboard:view"])
        );
        let effective = body["data"]["effectivePermissions"]
            .as_array()
            .expect("effective permissions should be an array");
        assert!(effective.iter().any(|item| {
            item["permission"] == "system:dashboard:view"
                && item["direct"] == true
                && item["roles"] == json!([])
        }));
        assert!(effective.iter().any(|item| {
            item["permission"] == "system:user:update"
                && item["direct"] == false
                && item["roles"][0]["id"] == 2
                && item["roles"][0]["code"] == "developer"
        }));

        tokens
            .revoke(&super_token)
            .await
            .expect("super session should revoke");
        tokens
            .revoke(&restricted_token)
            .await
            .expect("restricted session should revoke");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn employee_access_routes_enforce_bindings_and_stable_errors(pool: sqlx::PgPool) {
        let (app, tokens, super_token, restricted_token, super_user_id, target_user_id) =
            access_test_app(&pool).await;

        let (status, body) = request_json(
            &app,
            &restricted_token,
            Method::GET,
            &format!("/api/users/{target_user_id}/permissions"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["code"], "PERMISSION_DENIED");
        let denied_count: i64 = sqlx::query_scalar(
            "select count(*) from sys_audit_events where action = 'auth.access_denied'",
        )
        .fetch_one(&pool)
        .await
        .expect("access-denied audit count should load");
        assert_eq!(
            denied_count, 0,
            "the read permission must pass middleware before the super_admin guard denies it"
        );

        for (path, payload) in [
            (
                format!("/api/users/{target_user_id}/roles"),
                json!({ "roleIds": [2] }),
            ),
            (
                format!("/api/users/{target_user_id}/permissions"),
                json!({ "permissions": ["system:dashboard:view"] }),
            ),
        ] {
            let (status, body) =
                request_json(&app, &restricted_token, Method::PUT, &path, Some(payload)).await;
            assert_eq!(status, StatusCode::FORBIDDEN);
            assert_eq!(body["code"], "PERMISSION_DENIED");
        }
        let denied_count: i64 = sqlx::query_scalar(
            "select count(*) from sys_audit_events where action = 'auth.access_denied'",
        )
        .fetch_one(&pool)
        .await
        .expect("access-denied audit count should load");
        assert_eq!(
            denied_count, 2,
            "role and direct mutations must use their distinct write permissions"
        );

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            "/api/users/999999/permissions",
            Some(json!({ "permissions": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(
            body,
            json!({
                "code": "USER_NOT_FOUND",
                "message": "user not found",
                "data": null
            })
        );

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::PUT,
            &format!("/api/users/{super_user_id}/roles"),
            Some(json!({ "roleIds": [] })),
        )
        .await;
        assert_eq!(status, StatusCode::PRECONDITION_FAILED);
        assert_eq!(
            body,
            json!({
                "code": "LAST_SUPER_ADMIN",
                "message": "the final active super_admin cannot be removed",
                "data": null
            })
        );

        tokens
            .revoke(&super_token)
            .await
            .expect("super session should revoke");
        tokens
            .revoke(&restricted_token)
            .await
            .expect("restricted session should revoke");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ordinary_user_list_and_self_info_do_not_expose_assigned_roles(pool: sqlx::PgPool) {
        let (app, tokens, super_token, restricted_token, _, target_user_id) =
            access_test_app(&pool).await;

        let (status, body) = request_json(
            &app,
            &restricted_token,
            Method::GET,
            "/api/users?page=1&pageSize=20",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let target = body["data"]["list"]
            .as_array()
            .unwrap()
            .iter()
            .find(|user| user["id"] == target_user_id)
            .unwrap();
        assert_eq!(target["roles"], json!([]));
        assert_eq!(target["roleIds"], json!([]));

        let (status, body) =
            request_json(&app, &restricted_token, Method::GET, "/api/users/me", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["data"]["userInfo"]["roles"], json!([]));
        assert_eq!(body["data"]["userInfo"]["roleIds"], json!([]));

        let (status, body) = request_json(
            &app,
            &super_token,
            Method::GET,
            "/api/users?page=1&pageSize=20",
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let target = body["data"]["list"]
            .as_array()
            .unwrap()
            .iter()
            .find(|user| user["id"] == target_user_id)
            .unwrap();
        assert_eq!(target["roleIds"], json!([2]));

        tokens.revoke(&super_token).await.unwrap();
        tokens.revoke(&restricted_token).await.unwrap();
    }
}
