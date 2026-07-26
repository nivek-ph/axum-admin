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
            "/{id}/menus",
            get(handler::get_role_menus).put(handler::set_role_menus),
        )
        .route(
            "/{id}/permissions",
            get(handler::get_role_permissions).put(handler::set_role_permissions),
        )
        .route(
            "/{id}/depts",
            get(handler::get_role_depts).put(handler::set_role_depts),
        )
        .route(
            "/{id}/users",
            get(handler::get_role_users).put(handler::set_role_users),
        )
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header::AUTHORIZATION},
    };
    use tower::ServiceExt;

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_permission_endpoint_is_guarded_by_persisted_policy(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'permission-admin', 'Permission Admin', 'enabled', 10, 'self', false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values (
                100, 'permission-admin-uuid', 'permission-admin', 'hash',
                'Permission Admin', '', 'dashboard', true, 1, false
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values
                ('g', 'user:100', 'role:2', '', '', '', ''),
                ('p', 'role:2', 'system:role:permissions-read', '', '', '', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let client = redis::Client::open(redis_url).unwrap();
        let access = iam::access::AccessService::load_without_cache(pool.clone())
            .await
            .unwrap();
        let policy = iam::authorization::Authorization::load(pool.clone())
            .await
            .unwrap();
        let tokens = auth::token::TokenService::new(
            "test-secret",
            client.get_multiplexed_async_connection().await.unwrap(),
        );
        let session = tokens
            .create_session(100, "permission-admin")
            .await
            .unwrap();
        let mut state = crate::state::test_state(pool.clone());
        state.tokens = tokens.clone();
        state.access = access.clone();
        state.authorization = policy.clone();
        state.roles = iam::roles::RoleService::new(pool.clone(), access, policy);
        let app = crate::router::router(state.clone());
        let authorization = format!("Bearer {}", session.access_token);

        let allowed = app
            .clone()
            .oneshot(
                Request::get("/api/roles/2/permissions")
                    .header(AUTHORIZATION, &authorization)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(allowed.status(), StatusCode::OK);

        state.roles.set_permissions(2, Vec::new()).await.unwrap();
        let denied = app
            .oneshot(
                Request::get("/api/roles/2/permissions")
                    .header(AUTHORIZATION, authorization)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(denied.into_body(), usize::MAX).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["code"], "PERMISSION_DENIED");

        tokens.revoke(&session.access_token).await.unwrap();
    }
}
