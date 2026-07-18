use axum::{
    Json,
    extract::{Extension, Path, Query, State},
};

use super::dto::*;
use crate::{
    ApiResponse, AppResult, EmptyData, extractors::current_user::CurrentUser, state::AppState,
};

#[utoipa::path(
    get,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = ApiResponse<UserInfoData>),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_user_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<ApiResponse<UserInfoData>>> {
    let user = UserResponse::from(state.users.info(user.id).await?);
    Ok(Json(ApiResponse::ok(UserInfoData { user_info: user })))
}

#[utoipa::path(
    get,
    path = "/users",
    tag = "user",
    security(("bearer_auth" = [])),
    params(UserListRequest),
    responses((status = 200, description = "User list", body = ApiResponse<UserListData>))
)]
pub async fn get_user_list_by_query(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(payload): Query<UserListRequest>,
) -> AppResult<Json<ApiResponse<UserListData>>> {
    let page = payload.page.max(1);

    let page_size = payload.page_size.max(1);

    let (list, total) = state
        .users
        .list_with_scope(payload, user.data_scope.clone())
        .await?;

    let list = list.into_iter().map(UserResponse::from).collect::<Vec<_>>();

    Ok(Json(ApiResponse::ok(UserListData {
        list,
        total,
        page,
        page_size,
    })))
}

#[utoipa::path(
    post,
    path = "/users",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = RegisterUserRequest,
    responses((status = 200, description = "User registered", body = ApiResponse<EmptyData>))
)]
pub async fn admin_register(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<RegisterUserRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.users.create(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "registered", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/password",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = ChangePasswordRequest,
    responses((status = 200, description = "Password changed", body = ApiResponse<EmptyData>))
)]
pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    let prepared = state
        .users
        .prepare_password_change(user.id, payload)
        .await?;
    state
        .tokens
        .revoke_user_sessions(prepared.user_id())
        .await?;
    state.users.persist_password_update(prepared).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = UpdateUserRequest,
    responses((status = 200, description = "User updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_user_info_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.users.update(user.id, id, payload.into()).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfRequest,
    responses((status = 200, description = "Current user updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_self_info(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.users.set_self_info(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    put,
    path = "/users/me/settings",
    tag = "user",
    security(("bearer_auth" = [])),
    request_body = UpdateSelfSettingsRequest,
    responses((status = 200, description = "User settings updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_self_setting(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(payload): Json<UpdateSelfSettingsRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.users.set_self_setting(user.id, payload).await?;

    Ok(Json(ApiResponse::new("OK", "updated", None)))
}

#[utoipa::path(
    delete,
    path = "/users/{id}",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    responses((status = 200, description = "User deleted", body = ApiResponse<EmptyData>))
)]
pub async fn delete_user_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state.users.delete(user.id, id).await?;

    Ok(Json(ApiResponse::new("OK", "deleted", None)))
}

#[utoipa::path(
    post,
    path = "/users/{id}/password/reset",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = ResetPasswordRequest,
    responses((status = 200, description = "Password reset", body = ApiResponse<EmptyData>))
)]
pub async fn reset_password_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Json(payload): Json<ResetPasswordRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    let prepared = state
        .users
        .prepare_password_reset(user.id, id, payload.into())
        .await?;
    state
        .tokens
        .revoke_user_sessions(prepared.user_id())
        .await?;
    state.users.persist_password_update(prepared).await?;

    Ok(Json(ApiResponse::new("OK", "password reset", None)))
}

#[utoipa::path(
    put,
    path = "/users/{id}/roles",
    tag = "user",
    security(("bearer_auth" = [])),
    params(("id" = i64, Path, description = "User ID")),
    request_body = SetUserRolesRequest,
    responses((status = 200, description = "User roles updated", body = ApiResponse<EmptyData>))
)]
pub async fn set_user_roles_by_id(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Extension(audit_context): Extension<audit::AuditContext>,
    Path(id): Path<i64>,
    Json(payload): Json<SetUserRolesRequest>,
) -> AppResult<Json<ApiResponse<EmptyData>>> {
    state
        .users
        .set_user_roles_by_id(user.id, id, payload, audit_context)
        .await?;

    Ok(Json(ApiResponse::new("OK", "roles updated", None)))
}

#[cfg(test)]
mod tests {
    use auth::{
        password::PasswordService,
        token::{TokenService, TokenSessionError},
    };
    use iam::{
        access::ResolvedDataScope,
        users::{AuthenticatedUser, ChangePasswordRequest},
    };

    use super::*;

    async fn seed_password_users(pool: &sqlx::PgPool) -> PasswordService {
        let passwords = PasswordService::new();
        let actor_hash = passwords.hash_password("actor-old").unwrap();
        let target_hash = passwords.hash_password("target-old").unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values
                (601, 'password-actor', 'password-actor', $1, 'Actor', '', 'dashboard', true, 1, false),
                (602, 'password-target', 'password-target', $2, 'Target', '', 'dashboard', true, 1, false)
            "#,
        )
        .bind(actor_hash)
        .bind(target_hash)
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("insert into sys_user_roles (user_id, role_id) values (601, 1)")
            .execute(pool)
            .await
            .unwrap();
        passwords
    }

    async fn redis_tokens() -> TokenService {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let redis = redis::Client::open(redis_url)
            .expect("Redis test client should construct")
            .get_multiplexed_async_connection()
            .await
            .expect("Redis test connection should open");
        TokenService::new("test-secret", redis)
    }

    fn current_user(id: i64) -> CurrentUser {
        CurrentUser(AuthenticatedUser {
            id,
            data_scope: ResolvedDataScope::All,
        })
    }

    async fn password_matches(
        pool: &sqlx::PgPool,
        passwords: &PasswordService,
        user_id: i64,
        password: &str,
    ) -> bool {
        let hash =
            sqlx::query_scalar::<_, String>("select password_hash from sys_users where id = $1")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap();
        passwords.verify_password(password, &hash).unwrap()
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn self_password_change_validates_before_revoking_then_terminates_all_sessions(
        pool: sqlx::PgPool,
    ) {
        let passwords = seed_password_users(&pool).await;
        let tokens = redis_tokens().await;
        let first = tokens.create_session(601, "password-actor").await.unwrap();
        let second = tokens.create_session(601, "password-actor").await.unwrap();
        let mut state = crate::state::test_state(pool.clone());
        state.tokens = tokens.clone();

        let error = change_password(
            State(state.clone()),
            current_user(601),
            Json(ChangePasswordRequest {
                password: "wrong".to_string(),
                new_password: "actor-new".to_string(),
            }),
        )
        .await
        .expect_err("wrong current password should fail");
        assert_eq!(error.code(), "INVALID_PASSWORD");
        tokens.decode_active(&first.access_token).await.unwrap();
        tokens.decode_active(&second.access_token).await.unwrap();

        let _ = change_password(
            State(state),
            current_user(601),
            Json(ChangePasswordRequest {
                password: "actor-old".to_string(),
                new_password: "actor-new".to_string(),
            }),
        )
        .await
        .expect("password change should succeed");

        for access_token in [&first.access_token, &second.access_token] {
            assert!(matches!(
                tokens.decode_active(access_token).await,
                Err(TokenSessionError::SessionInvalid)
            ));
        }
        assert!(password_matches(&pool, &passwords, 601, "actor-new").await);
        assert!(!password_matches(&pool, &passwords, 601, "actor-old").await);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn self_password_change_keeps_old_password_when_session_store_is_unavailable(
        pool: sqlx::PgPool,
    ) {
        let passwords = seed_password_users(&pool).await;
        let state = crate::state::test_state(pool.clone());

        let error = change_password(
            State(state),
            current_user(601),
            Json(ChangePasswordRequest {
                password: "actor-old".to_string(),
                new_password: "actor-new".to_string(),
            }),
        )
        .await
        .expect_err("missing session store should fail");

        assert_eq!(error.code(), "AUTHORIZATION_UNAVAILABLE");
        assert!(password_matches(&pool, &passwords, 601, "actor-old").await);
        assert!(!password_matches(&pool, &passwords, 601, "actor-new").await);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn database_failure_after_revocation_keeps_old_password_and_sessions_terminated(
        pool: sqlx::PgPool,
    ) {
        let passwords = seed_password_users(&pool).await;
        let tokens = redis_tokens().await;
        let session = tokens.create_session(601, "password-actor").await.unwrap();
        let mut state = crate::state::test_state(pool.clone());
        state.tokens = tokens.clone();
        sqlx::query(
            r#"
            create function fail_password_update() returns trigger language plpgsql as $$
            begin
                raise exception 'password update failed';
            end;
            $$
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            create trigger fail_password_update
            before update of password_hash on sys_users
            for each row execute function fail_password_update()
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let error = change_password(
            State(state),
            current_user(601),
            Json(ChangePasswordRequest {
                password: "actor-old".to_string(),
                new_password: "actor-new".to_string(),
            }),
        )
        .await
        .expect_err("database write should fail");

        assert_eq!(error.code(), "INTERNAL_SERVER_ERROR");
        assert!(matches!(
            tokens.decode_active(&session.access_token).await,
            Err(TokenSessionError::SessionInvalid)
        ));
        assert!(password_matches(&pool, &passwords, 601, "actor-old").await);
        assert!(!password_matches(&pool, &passwords, 601, "actor-new").await);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn admin_password_reset_terminates_only_the_target_users_sessions(pool: sqlx::PgPool) {
        let passwords = seed_password_users(&pool).await;
        let tokens = redis_tokens().await;
        let admin = tokens.create_session(601, "password-actor").await.unwrap();
        let first_target = tokens.create_session(602, "password-target").await.unwrap();
        let second_target = tokens.create_session(602, "password-target").await.unwrap();
        let mut state = crate::state::test_state(pool.clone());
        state.tokens = tokens.clone();

        let _ = reset_password_by_id(
            State(state),
            current_user(601),
            Path(602),
            Json(ResetPasswordRequest {
                id: 602,
                password: "target-new".to_string(),
            }),
        )
        .await
        .expect("admin password reset should succeed");

        tokens
            .decode_active(&admin.access_token)
            .await
            .expect("admin session should remain active");
        for access_token in [&first_target.access_token, &second_target.access_token] {
            assert!(matches!(
                tokens.decode_active(access_token).await,
                Err(TokenSessionError::SessionInvalid)
            ));
        }
        assert!(password_matches(&pool, &passwords, 602, "target-new").await);
        assert!(!password_matches(&pool, &passwords, 602, "target-old").await);
        tokens.revoke(&admin.access_token).await.unwrap();
    }
}
