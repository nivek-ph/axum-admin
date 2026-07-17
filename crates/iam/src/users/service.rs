use std::collections::HashMap;

use audit::{
    AuditAction, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
    AuditValue, FieldChange,
};
use auth::password::PasswordService;
use uuid::Uuid;

use super::{
    AuthenticateError, ChangePasswordRequest, DeleteUserRequest, GetUserListRequest, LoginIdentity,
    LoginRequest, RegisterRequest, ResetPasswordInput, SetSelfInfoRequest, SetSelfSettingRequest,
    SetUserRolesRequest, UpdateUserInput, UserError, UserInfoView, UserRecord,
};
use crate::{
    access::{AccessService, ResolvedDataScope},
    roles::RoleSummary,
};

/// Default avatar when none is provided. Empty so the UI falls back to initials
const HEADER_IMG: &str = "";

#[derive(Clone)]
pub struct UserService {
    pool: sqlx::PgPool,
    passwords: PasswordService,
    access: AccessService,
    audit: AuditService,
}

impl UserService {
    pub fn new(
        pool: sqlx::PgPool,
        access: AccessService,
        audit: AuditService,
        passwords: PasswordService,
    ) -> Self {
        Self {
            pool,
            access,
            audit,
            passwords,
        }
    }

    pub async fn list(
        &self,
        query: GetUserListRequest,
        actor_user_id: i64,
    ) -> Result<(Vec<UserInfoView>, i64), UserError> {
        get_user_list(&self.pool, query, Some(actor_user_id)).await
    }

    pub async fn list_with_scope(
        &self,
        query: GetUserListRequest,
        scope: ResolvedDataScope,
    ) -> Result<(Vec<UserInfoView>, i64), UserError> {
        get_user_list_with_scope(&self.pool, query, scope).await
    }

    pub async fn info(&self, user_id: i64) -> Result<UserInfoView, UserError> {
        load_user_info(&self.pool, user_id).await
    }

    pub async fn ensure_admin(
        &self,
        username: &str,
        password: &str,
        nickname: &str,
    ) -> Result<(), UserError> {
        Ok(ensure_admin_user(&self.pool, &self.passwords, username, password, nickname).await?)
    }

    pub async fn register(&self, payload: RegisterRequest) -> Result<(), UserError> {
        create_user(&self.pool, &self.passwords, payload).await?;
        self.bump_access_version().await
    }

    pub async fn create(
        &self,
        actor_user_id: i64,
        payload: RegisterRequest,
    ) -> Result<(), UserError> {
        let role_ids = normalize_role_ids(payload.role_ids.as_ref())?;
        ensure_role_assignment_actor(&self.pool, actor_user_id, &role_ids).await?;
        create_user(&self.pool, &self.passwords, payload).await?;
        self.bump_access_version().await
    }

    pub async fn authenticate(
        &self,
        payload: LoginRequest,
    ) -> Result<LoginIdentity, AuthenticateError> {
        login(&self.pool, &self.passwords, payload).await
    }

    pub async fn change_password(
        &self,
        user_id: i64,
        payload: ChangePasswordRequest,
    ) -> Result<(), UserError> {
        change_password(&self.pool, &self.passwords, user_id, payload).await
    }

    pub async fn update(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        payload: UpdateUserInput,
    ) -> Result<(), UserError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        update_user(&self.pool, target_user_id, payload).await?;
        self.bump_access_version().await
    }

    pub async fn set_self_info(
        &self,
        user_id: i64,
        payload: SetSelfInfoRequest,
    ) -> Result<(), UserError> {
        set_self_info(&self.pool, user_id, payload).await
    }

    pub async fn set_self_setting(
        &self,
        user_id: i64,
        payload: SetSelfSettingRequest,
    ) -> Result<(), UserError> {
        update_setting_by_user_id(&self.pool, user_id, payload).await
    }

    pub async fn delete(&self, actor_user_id: i64, target_user_id: i64) -> Result<(), UserError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        delete_user(&self.pool, DeleteUserRequest { id: target_user_id }).await?;
        self.bump_access_version().await
    }

    pub async fn reset_password_by_id(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        payload: ResetPasswordInput,
    ) -> Result<(), UserError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        reset_password(&self.pool, &self.passwords, target_user_id, payload).await
    }

    pub async fn set_user_roles_by_id(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        payload: SetUserRolesRequest,
        audit_context: AuditContext,
    ) -> Result<(), UserError> {
        if let Err(error) = ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await {
            self.record_role_assignment_failure(&audit_context, target_user_id, &error)
                .await;
            return Err(error);
        }
        if let Err(error) =
            ensure_role_assignment_actor(&self.pool, actor_user_id, &payload.role_ids).await
        {
            self.record_role_assignment_failure(&audit_context, target_user_id, &error)
                .await;
            return Err(error);
        }
        if let Err(error) =
            set_user_roles_with_audit(&self.pool, target_user_id, payload, audit_context.clone())
                .await
        {
            self.record_role_assignment_failure(&audit_context, target_user_id, &error)
                .await;
            return Err(error);
        }
        self.bump_access_version().await
    }

    async fn record_role_assignment_failure(
        &self,
        context: &AuditContext,
        target_user_id: i64,
        error: &UserError,
    ) {
        let (result, reason_code) = match error {
            UserError::NotFound | UserError::InvalidRoles => {
                (AuditResult::Denied, AuditReason::InvalidRoleAssignment)
            }
            UserError::AlreadyExists
            | UserError::InvalidPassword
            | UserError::Password(_)
            | UserError::Database(_)
            | UserError::Audit(_)
            | UserError::AccessPropagation(_) => (AuditResult::Failed, AuditReason::InternalError),
        };
        self.audit
            .record_best_effort(AuditEvent {
                actor: context.actor.clone(),
                action: AuditAction::AssignUserRoles,
                resource: AuditResource::User(target_user_id),
                result,
                reason_code: Some(reason_code),
                source: context.source.clone(),
                changes: Vec::new(),
            })
            .await;
    }

    async fn bump_access_version(&self) -> Result<(), UserError> {
        self.access.bump_version().await?;
        Ok(())
    }
}

pub(crate) async fn ensure_admin_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    username: &str,
    password: &str,
    nick_name: &str,
) -> Result<(), sqlx::Error> {
    ensure_user_with_role(
        pool,
        password_service,
        username,
        password,
        nick_name,
        1,
        true,
    )
    .await
}

async fn ensure_user_with_role(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    username: &str,
    password: &str,
    nick_name: &str,
    role_id: i64,
    is_system: bool,
) -> Result<(), sqlx::Error> {
    let role = crate::roles::find(pool, role_id)
        .await?
        .ok_or(sqlx::Error::RowNotFound)?;

    let user_id = if let Some(existing) = find_by_username(pool, username).await? {
        sqlx::query(
            r#"
            update sys_users
            set nick_name = $1,
                home_route = $2,
                enable = true,
                dept_id = 1,
                is_system = $3,
                updated_at = now()
            where id = $4
            "#,
        )
        .bind(nick_name)
        .bind("dashboard")
        .bind(is_system)
        .bind(existing.id)
        .execute(pool)
        .await?;
        existing.id
    } else {
        let password_hash = password_service
            .hash_password(password)
            .map_err(|err| sqlx::Error::Protocol(err.to_string()))?;

        sqlx::query_scalar(
            r#"
        insert into sys_users (
            uuid,
            username,
            password_hash,
            nick_name,
            header_img,
            home_route,
            enable,
            phone,
            email,
            origin_setting,
            dept_id,
            is_system
        ) values ($1, $2, $3, $4, $5, $6, true, null, null, null, 1, $7)
        returning id
        "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(username)
        .bind(password_hash)
        .bind(nick_name)
        .bind(HEADER_IMG)
        .bind("dashboard")
        .bind(is_system)
        .fetch_one(pool)
        .await?
    };

    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await?;

    sqlx::query(
        r#"
        insert into sys_user_roles (user_id, role_id)
        values ($1, $2)
        on conflict do nothing
        "#,
    )
    .bind(user_id)
    .bind(role.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn create_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: RegisterRequest,
) -> Result<(), UserError> {
    if find_by_username(pool, &payload.user_name).await?.is_some() {
        return Err(UserError::AlreadyExists);
    }
    let role_ids = normalize_role_ids(payload.role_ids.as_ref())?;
    ensure_assignable_roles(pool, &role_ids).await?;
    let password_hash = password_service.hash_password(&payload.password)?;

    let user_id: i64 = sqlx::query_scalar(
        r#"
        insert into sys_users (
            uuid,
            username,
            password_hash,
            nick_name,
            header_img,
            home_route,
            enable,
            phone,
            email,
            origin_setting,
            dept_id
        ) values ($1, $2, $3, $4, $5, 'dashboard', $6, $7, $8, null, $9)
        returning id
        "#,
    )
    .bind(Uuid::new_v4().to_string())
    .bind(&payload.user_name)
    .bind(password_hash)
    .bind(&payload.nick_name)
    .bind(payload.header_img.unwrap_or_else(|| HEADER_IMG.to_string()))
    .bind(payload.enable.unwrap_or(1) == 1)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.dept_id.or(Some(1)))
    .fetch_one(pool)
    .await?;

    replace_user_roles(pool, user_id, role_ids).await?;

    Ok(())
}

pub(crate) async fn login(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: LoginRequest,
) -> Result<LoginIdentity, AuthenticateError> {
    let record = find_by_username(pool, &payload.username)
        .await?
        .ok_or(AuthenticateError::InvalidCredentials)?;

    if !record.enable {
        return Err(AuthenticateError::Disabled);
    }

    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(AuthenticateError::InvalidCredentials);
    }

    let roles = get_roles_by_user_id(pool, record.id).await?;

    Ok(LoginIdentity {
        id: record.id,
        username: record.username.clone(),
        user: build_user_info(&record, roles),
    })
}

pub(crate) async fn get_user_list(
    pool: &sqlx::PgPool,
    query: GetUserListRequest,
    actor_user_id: Option<i64>,
) -> Result<(Vec<UserInfoView>, i64), UserError> {
    let scope_filter = match actor_user_id {
        Some(user_id) => crate::access::resolve_user_data_scope(pool, user_id, "users").await?,
        None => ResolvedDataScope::All,
    };
    get_user_list_with_scope(pool, query, scope_filter).await
}

async fn get_user_list_with_scope(
    pool: &sqlx::PgPool,
    query: GetUserListRequest,
    resolved_data_scope: ResolvedDataScope,
) -> Result<(Vec<UserInfoView>, i64), UserError> {
    let page = query.page.max(1);
    let page_size = query.page_size.max(1);
    let offset = (page - 1) * page_size;
    let order_key = match query.order_key.as_deref() {
        Some("username") => "u.username",
        Some("nick_name") => "u.nick_name",
        Some("phone") => "u.phone",
        Some("email") => "u.email",
        _ => "u.id",
    };
    let order_dir = if query.desc.unwrap_or(true) {
        "desc"
    } else {
        "asc"
    };
    let order_clause = format!("{order_key} {order_dir}");
    if matches!(&resolved_data_scope, ResolvedDataScope::DeptIds(dept_ids) if dept_ids.is_empty()) {
        return Ok((Vec::new(), 0));
    }
    let scope_clause = scope_sql_clause(&resolved_data_scope);

    let total_sql = format!(
        r#"
        select count(*) from sys_users u
        where ($1::text is null or u.username ilike '%' || $1 || '%')
          and ($2::text is null or u.nick_name ilike '%' || $2 || '%')
          and ($3::text is null or coalesce(u.phone, '') ilike '%' || $3 || '%')
          and ($4::text is null or coalesce(u.email, '') ilike '%' || $4 || '%')
          {scope_clause}
        "#
    );
    let mut total_query = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(total_sql))
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref());

    total_query = match &resolved_data_scope {
        ResolvedDataScope::All => total_query,
        ResolvedDataScope::DeptIds(dept_ids) => total_query.bind(dept_ids),
        ResolvedDataScope::Owner(owner_id) => total_query.bind(owner_id),
    };
    let total = total_query.fetch_one(pool).await?;

    let sql = format!(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.home_route,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where ($1::text is null or u.username ilike '%' || $1 || '%')
          and ($2::text is null or u.nick_name ilike '%' || $2 || '%')
          and ($3::text is null or coalesce(u.phone, '') ilike '%' || $3 || '%')
          and ($4::text is null or coalesce(u.email, '') ilike '%' || $4 || '%')
          {scope_clause}
        order by {order_clause}
        limit ${limit_placeholder} offset ${offset_placeholder}
        "#,
        limit_placeholder = if matches!(&resolved_data_scope, ResolvedDataScope::All) {
            5
        } else {
            6
        },
        offset_placeholder = if matches!(&resolved_data_scope, ResolvedDataScope::All) {
            6
        } else {
            7
        }
    );

    let mut rows_query = sqlx::query_as::<_, UserRecord>(sqlx::AssertSqlSafe(sql))
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref());
    rows_query = match &resolved_data_scope {
        ResolvedDataScope::All => rows_query,
        ResolvedDataScope::DeptIds(dept_ids) => rows_query.bind(dept_ids),
        ResolvedDataScope::Owner(owner_id) => rows_query.bind(owner_id),
    };
    let rows = rows_query
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let user_ids = rows.iter().map(|record| record.id).collect::<Vec<_>>();
    let mut roles_by_user_id = get_roles_by_user_ids(pool, &user_ids).await?;
    let mut list = Vec::with_capacity(rows.len());
    for record in rows {
        let roles = roles_by_user_id.remove(&record.id).unwrap_or_default();
        list.push(build_user_info(&record, roles));
    }

    Ok((list, total))
}

fn scope_sql_clause(filter: &ResolvedDataScope) -> &'static str {
    match filter {
        ResolvedDataScope::All => "",
        ResolvedDataScope::DeptIds(_) => "and u.dept_id = any($5)",
        ResolvedDataScope::Owner(_) => "and u.id = $5",
    }
}

pub(crate) async fn ensure_user_in_scope(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    target_user_id: i64,
) -> Result<(), UserError> {
    let filter = crate::access::resolve_user_data_scope(pool, actor_user_id, "users").await?;
    let visible = match filter {
        ResolvedDataScope::All => true,
        ResolvedDataScope::Owner(owner_id) => owner_id == target_user_id,
        ResolvedDataScope::DeptIds(dept_ids) => {
            if dept_ids.is_empty() {
                false
            } else {
                sqlx::query_scalar::<_, bool>(
                    r#"
                    select exists(
                        select 1 from sys_users
                        where id = $1 and dept_id = any($2)
                    )
                    "#,
                )
                .bind(target_user_id)
                .bind(&dept_ids)
                .fetch_one(pool)
                .await?
            }
        }
    };

    if visible {
        Ok(())
    } else {
        Err(UserError::NotFound)
    }
}

pub(crate) async fn update_user(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: UpdateUserInput,
) -> Result<(), UserError> {
    sqlx::query(
        r#"
        update sys_users
        set nick_name = $1,
            header_img = $2,
            enable = $3,
            phone = $4,
            email = $5,
            dept_id = coalesce($6, dept_id),
            updated_at = now()
        where id = $7
        "#,
    )
    .bind(payload.nick_name)
    .bind(payload.header_img)
    .bind(payload.enable == 1)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.dept_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn delete_user(
    pool: &sqlx::PgPool,
    payload: DeleteUserRequest,
) -> Result<(), UserError> {
    sqlx::query("delete from sys_users where id = $1")
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn reset_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    user_id: i64,
    payload: ResetPasswordInput,
) -> Result<(), UserError> {
    let password_hash = password_service.hash_password(&payload.password)?;
    sqlx::query(
        r#"
        update sys_users
        set password_hash = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn set_user_roles_with_audit(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetUserRolesRequest,
    audit_context: AuditContext,
) -> Result<(), UserError> {
    let role_ids = normalize_role_ids(Some(&payload.role_ids))?;
    ensure_assignable_roles(pool, &role_ids).await?;

    let mut tx = pool.begin().await?;
    let previous_role_ids = sqlx::query_scalar::<_, i64>(
        "select role_id from sys_user_roles where user_id = $1 order by role_id",
    )
    .bind(user_id)
    .fetch_all(&mut *tx)
    .await?;

    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        insert into sys_user_roles (user_id, role_id)
        select $1, unnest($2::bigint[])
        on conflict do nothing
        "#,
    )
    .bind(user_id)
    .bind(&role_ids)
    .execute(&mut *tx)
    .await?;

    AuditService::record_in(
        &mut tx,
        AuditEvent {
            actor: audit_context.actor,
            action: AuditAction::AssignUserRoles,
            resource: AuditResource::User(user_id),
            result: AuditResult::Succeeded,
            reason_code: None,
            source: audit_context.source,
            changes: vec![FieldChange {
                field: "role_ids".to_string(),
                before: AuditValue::Ids(previous_role_ids),
                after: AuditValue::Ids(role_ids),
            }],
        },
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) async fn change_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    user_id: i64,
    payload: ChangePasswordRequest,
) -> Result<(), UserError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(UserError::NotFound)?;
    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(UserError::InvalidPassword);
    }

    let password_hash = password_service.hash_password(&payload.new_password)?;
    sqlx::query(
        r#"
        update sys_users
        set password_hash = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(password_hash)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn set_self_info(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetSelfInfoRequest,
) -> Result<(), UserError> {
    sqlx::query(
        r#"
        update sys_users
        set nick_name = coalesce($1, nick_name),
            header_img = coalesce($2, header_img),
            phone = coalesce($3, phone),
            email = coalesce($4, email),
            updated_at = now()
        where id = $5
        "#,
    )
    .bind(payload.nick_name)
    .bind(payload.header_img)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn update_setting_by_user_id(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetSelfSettingRequest,
) -> Result<(), UserError> {
    sqlx::query(
        r#"
        update sys_users
        set origin_setting = $1,
            updated_at = now()
        where id = $2
        "#,
    )
    .bind(payload.origin_setting)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn find_by_username(
    pool: &sqlx::PgPool,
    username: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.home_route,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where u.username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
}

async fn find_by_id(pool: &sqlx::PgPool, user_id: i64) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        select
            u.id,
            u.uuid,
            u.username,
            u.password_hash,
            u.nick_name,
            u.header_img,
            u.home_route,
            u.enable,
            u.phone,
            u.email,
            u.origin_setting,
            u.dept_id,
            d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        where u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

async fn load_user_info(pool: &sqlx::PgPool, user_id: i64) -> Result<UserInfoView, UserError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(UserError::NotFound)?;
    let roles = get_roles_by_user_id(pool, user_id).await?;
    Ok(build_user_info(&record, roles))
}

fn build_user_info(record: &UserRecord, roles: Vec<RoleSummary>) -> UserInfoView {
    UserInfoView {
        id: record.id,
        uuid: record.uuid.clone(),
        user_name: record.username.clone(),
        nick_name: record.nick_name.clone(),
        header_img: record.header_img.clone(),
        home_route: record.home_route.clone(),
        enable: if record.enable { 1 } else { 2 },
        phone: record.phone.clone().unwrap_or_default(),
        email: record.email.clone().unwrap_or_default(),
        origin_setting: record.origin_setting.clone(),
        dept_id: record.dept_id,
        dept_name: record.dept_name.clone().unwrap_or_default(),
        role_ids: roles.iter().map(|role| role.id).collect(),
        roles,
    }
}

#[derive(sqlx::FromRow)]
struct UserRoleRow {
    user_id: i64,
    id: i64,
    code: String,
    name: String,
    status: String,
    sort: i32,
    data_scope: String,
    is_system: bool,
}

impl From<UserRoleRow> for RoleSummary {
    fn from(row: UserRoleRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            status: row.status,
            sort: row.sort,
            data_scope: row.data_scope,
            is_system: row.is_system,
        }
    }
}

async fn get_roles_by_user_ids(
    pool: &sqlx::PgPool,
    user_ids: &[i64],
) -> Result<HashMap<i64, Vec<RoleSummary>>, sqlx::Error> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = sqlx::query_as::<_, UserRoleRow>(
        r#"
        select ur.user_id, r.id, r.code, r.name, r.status, r.sort, r.data_scope, r.is_system
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        where ur.user_id = any($1) and r.status = 'enabled'
        order by ur.user_id, r.sort, r.id
        "#,
    )
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    let mut roles_by_user_id = HashMap::<i64, Vec<RoleSummary>>::new();
    for row in rows {
        roles_by_user_id
            .entry(row.user_id)
            .or_default()
            .push(row.into());
    }
    Ok(roles_by_user_id)
}

async fn get_roles_by_user_id(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<Vec<RoleSummary>, sqlx::Error> {
    Ok(get_roles_by_user_ids(pool, &[user_id])
        .await?
        .remove(&user_id)
        .unwrap_or_default())
}

async fn replace_user_roles(
    pool: &sqlx::PgPool,
    user_id: i64,
    role_ids: Vec<i64>,
) -> Result<(), UserError> {
    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        insert into sys_user_roles (user_id, role_id)
        select $1, unnest($2::bigint[])
        on conflict do nothing
        "#,
    )
    .bind(user_id)
    .bind(&role_ids)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

fn normalize_role_ids(role_ids: Option<&Vec<i64>>) -> Result<Vec<i64>, UserError> {
    let ids = role_ids.cloned().unwrap_or_default();
    let ids = ids
        .into_iter()
        .filter(|id| *id > 0)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err(UserError::InvalidRoles);
    }
    Ok(ids)
}

async fn ensure_assignable_roles(pool: &sqlx::PgPool, role_ids: &[i64]) -> Result<(), UserError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM sys_roles WHERE id = ANY($1) AND status = 'enabled'",
    )
    .bind(role_ids)
    .fetch_one(pool)
    .await?;
    if count != role_ids.len() as i64 {
        return Err(UserError::InvalidRoles);
    }
    Ok(())
}

async fn ensure_role_assignment_actor(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    role_ids: &[i64],
) -> Result<(), UserError> {
    let assigns_super = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sys_roles WHERE id = ANY($1) AND code = 'super_admin')",
    )
    .bind(role_ids)
    .fetch_one(pool)
    .await?;
    if !assigns_super {
        return Ok(());
    }
    let actor_is_super = sqlx::query_scalar::<_, bool>(
        r#"SELECT EXISTS(
            SELECT 1 FROM sys_user_roles ur
            JOIN sys_roles r ON r.id = ur.role_id
            WHERE ur.user_id = $1 AND r.code = 'super_admin' AND r.status = 'enabled'
        )"#,
    )
    .bind(actor_user_id)
    .fetch_one(pool)
    .await?;
    if actor_is_super {
        Ok(())
    } else {
        Err(UserError::InvalidRoles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seed_role_assignment(pool: &sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values
                (100, 'actor-uuid', 'actor', 'hash', 'Actor', '', 'dashboard', true, 1, false),
                (101, 'target-uuid', 'target', 'hash', 'Target', '', 'dashboard', true, 1, false)
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'audited_role', 'Audited Role', 'enabled', 10, 'self', false)
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query("insert into sys_user_roles (user_id, role_id) values (101, 1)")
            .execute(pool)
            .await
            .unwrap();
    }

    fn audit_context() -> AuditContext {
        AuditContext {
            actor: audit::AuditActor {
                id: Some(100),
                label: "actor".to_string(),
            },
            source: audit::AuditSource {
                ip: "127.0.0.1".to_string(),
                user_agent: "iam-test".to_string(),
            },
        }
    }

    #[test]
    fn scope_sql_clause_matches_filter_shape() {
        assert_eq!(scope_sql_clause(&ResolvedDataScope::All), "");
        assert_eq!(
            scope_sql_clause(&ResolvedDataScope::DeptIds(vec![1, 2])),
            "and u.dept_id = any($5)"
        );
        assert_eq!(
            scope_sql_clause(&ResolvedDataScope::Owner(7)),
            "and u.id = $5"
        );
    }

    #[test]
    fn normalize_role_ids_requires_explicit_roles() {
        assert!(matches!(
            normalize_role_ids(None),
            Err(UserError::InvalidRoles)
        ));
        assert_eq!(
            normalize_role_ids(Some(&vec![2, 1, 2])).unwrap(),
            vec![1, 2]
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn paged_users_keep_their_roles_and_order(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values
                (200, 'batch-a-uuid', 'batch-a', 'hash', 'Batch A', '', 'dashboard', true, 1, false),
                (201, 'batch-b-uuid', 'batch-b', 'hash', 'Batch B', '', 'dashboard', true, 1, false),
                (202, 'batch-c-uuid', 'batch-c', 'hash', 'Batch C', '', 'dashboard', true, 1, false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values
                (10, 'batch-later', 'Batch Later', 'enabled', 20, 'self', false),
                (11, 'batch-first', 'Batch First', 'enabled', 10, 'self', false),
                (12, 'batch-disabled', 'Batch Disabled', 'disabled', 0, 'self', false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_user_roles (user_id, role_id)
            values (200, 10), (200, 11), (201, 10), (202, 12)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let access = AccessService::new(pool.clone());
        let audit = AuditService::new(pool.clone());
        let service = UserService::new(pool, access, audit, PasswordService::new());
        let first_page = GetUserListRequest {
            page: 1,
            page_size: 2,
            username: Some("batch-".to_string()),
            nick_name: None,
            phone: None,
            email: None,
            order_key: Some("username".to_string()),
            desc: Some(false),
        };
        let (users, total) = service
            .list_with_scope(first_page.clone(), ResolvedDataScope::All)
            .await
            .unwrap();

        assert_eq!(total, 3);
        assert_eq!(
            users
                .iter()
                .map(|user| user.user_name.as_str())
                .collect::<Vec<_>>(),
            vec!["batch-a", "batch-b"]
        );
        assert_eq!(users[0].role_ids, vec![11, 10]);
        assert_eq!(users[1].role_ids, vec![10]);

        let (users, total) = service
            .list_with_scope(
                GetUserListRequest {
                    page: 2,
                    ..first_page
                },
                ResolvedDataScope::All,
            )
            .await
            .unwrap();

        assert_eq!(total, 3);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].user_name, "batch-c");
        assert!(users[0].roles.is_empty());
        assert!(users[0].role_ids.is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_assignment_and_audit_event_commit_together(pool: sqlx::PgPool) {
        seed_role_assignment(&pool).await;

        set_user_roles_with_audit(
            &pool,
            101,
            SetUserRolesRequest { role_ids: vec![2] },
            audit_context(),
        )
        .await
        .unwrap();

        let role_ids = sqlx::query_scalar::<_, i64>(
            "select role_id from sys_user_roles where user_id = 101 order by role_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(role_ids, vec![2]);

        let (action, result, changes): (String, String, serde_json::Value) = sqlx::query_as(
            "select action, result, changes from sys_audit_events where resource_type = 'user' and resource_id = '101'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(action, "user.assign_roles");
        assert_eq!(result, "succeeded");
        assert_eq!(changes[0]["field"], "role_ids");
        assert_eq!(changes[0]["before"]["value"], serde_json::json!([1]));
        assert_eq!(changes[0]["after"]["value"], serde_json::json!([2]));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_assignment_rolls_back_when_audit_insert_fails(pool: sqlx::PgPool) {
        seed_role_assignment(&pool).await;
        sqlx::query("drop table sys_audit_events")
            .execute(&pool)
            .await
            .unwrap();

        let error = set_user_roles_with_audit(
            &pool,
            101,
            SetUserRolesRequest { role_ids: vec![2] },
            audit_context(),
        )
        .await
        .expect_err("missing audit store should fail the role assignment");
        assert!(matches!(error, UserError::Audit(_)));

        let role_ids = sqlx::query_scalar::<_, i64>(
            "select role_id from sys_user_roles where user_id = 101 order by role_id",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(role_ids, vec![1]);
    }
}
