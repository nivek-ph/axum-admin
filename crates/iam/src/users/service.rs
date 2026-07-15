use auth::password::PasswordService;
use uuid::Uuid;

use super::{
    AuthenticatedUser, ChangePasswordRequest, DeleteUserRequest, GetUserListRequest, LoginError,
    LoginIdentity, LoginRequest, RegisterRequest, ResetPasswordRequest, SetSelfInfoRequest,
    SetSelfSettingRequest, SetUserRolesRequest, UpdateUserRequest, UserInfoView, UserRecord,
};
use crate::access::DataScopeFilter;
use crate::roles::RoleSummary;

#[derive(Clone)]
pub struct UserService {
    pool: sqlx::PgPool,
    passwords: PasswordService,
}

impl UserService {
    pub fn new(pool: sqlx::PgPool, passwords: PasswordService) -> Self {
        Self { pool, passwords }
    }

    pub async fn list(
        &self,
        query: GetUserListRequest,
        actor_user_id: i64,
    ) -> Result<(Vec<UserInfoView>, i64), LoginError> {
        get_user_list(&self.pool, query, Some(actor_user_id)).await
    }

    pub async fn list_with_scope(
        &self,
        query: GetUserListRequest,
        scope: DataScopeFilter,
    ) -> Result<(Vec<UserInfoView>, i64), LoginError> {
        get_user_list_with_scope(&self.pool, query, scope).await
    }

    pub async fn info(&self, user_id: i64) -> Result<UserInfoView, LoginError> {
        load_user_info(&self.pool, user_id).await
    }

    pub async fn ensure_admin(
        &self,
        username: &str,
        password: &str,
        nickname: &str,
    ) -> Result<(), LoginError> {
        Ok(ensure_admin_user(&self.pool, &self.passwords, username, password, nickname).await?)
    }

    pub async fn register(&self, payload: RegisterRequest) -> Result<(), LoginError> {
        register_user(&self.pool, &self.passwords, payload).await
    }

    pub async fn register_as(
        &self,
        actor_user_id: i64,
        payload: RegisterRequest,
    ) -> Result<(), LoginError> {
        let role_ids = normalize_role_ids(payload.role_ids.as_ref())?;
        ensure_role_assignment_actor(&self.pool, actor_user_id, &role_ids).await?;
        register_user(&self.pool, &self.passwords, payload).await
    }

    pub async fn authenticate(&self, payload: LoginRequest) -> Result<LoginIdentity, LoginError> {
        login(&self.pool, &self.passwords, payload).await
    }

    pub async fn change_password(
        &self,
        user_id: i64,
        payload: ChangePasswordRequest,
    ) -> Result<(), LoginError> {
        change_password(&self.pool, &self.passwords, user_id, payload).await
    }

    pub async fn update_as(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        mut payload: UpdateUserRequest,
    ) -> Result<(), LoginError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        payload.id = target_user_id;
        update_user(&self.pool, payload).await
    }

    pub async fn set_self_info(
        &self,
        user_id: i64,
        payload: SetSelfInfoRequest,
    ) -> Result<(), LoginError> {
        set_self_info(&self.pool, user_id, payload).await
    }

    pub async fn set_self_setting(
        &self,
        user_id: i64,
        payload: SetSelfSettingRequest,
    ) -> Result<(), LoginError> {
        set_self_setting(&self.pool, user_id, payload).await
    }

    pub async fn delete_as(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
    ) -> Result<(), LoginError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        delete_user(&self.pool, DeleteUserRequest { id: target_user_id }).await
    }

    pub async fn reset_password_as(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        mut payload: ResetPasswordRequest,
    ) -> Result<(), LoginError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        payload.id = target_user_id;
        reset_password(&self.pool, &self.passwords, payload).await
    }

    pub async fn set_roles_as(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        payload: SetUserRolesRequest,
    ) -> Result<(), LoginError> {
        ensure_user_in_scope(&self.pool, actor_user_id, target_user_id).await?;
        ensure_role_assignment_actor(&self.pool, actor_user_id, &payload.role_ids).await?;
        set_user_roles(&self.pool, target_user_id, payload).await
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
        .bind("https://qmplusimg.henrongyi.top/gva_header.jpg")
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

pub(crate) async fn register_user(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: RegisterRequest,
) -> Result<(), LoginError> {
    if find_by_username(pool, &payload.user_name).await?.is_some() {
        return Err(LoginError::UserAlreadyExists);
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
    .bind(
        payload
            .header_img
            .unwrap_or_else(|| "https://qmplusimg.henrongyi.top/gva_header.jpg".to_string()),
    )
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
) -> Result<LoginIdentity, LoginError> {
    let record = find_by_username(pool, &payload.username)
        .await?
        .ok_or(LoginError::InvalidCredentials)?;

    if !record.enable {
        return Err(LoginError::Disabled);
    }

    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(LoginError::InvalidCredentials);
    }

    let roles = get_roles_by_user_id(pool, record.id).await?;

    Ok(LoginIdentity {
        id: record.id,
        username: record.username.clone(),
        user: build_user_info(&record, roles),
    })
}

pub(crate) async fn load_authenticated_user(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<AuthenticatedUser, LoginError> {
    let exists = sqlx::query_scalar::<_, bool>("SELECT enable FROM sys_users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        .ok_or(LoginError::UserNotFound)?;
    if !exists {
        return Err(LoginError::Disabled);
    }
    Ok(AuthenticatedUser {
        id: user_id,
        data_scope: DataScopeFilter::All,
    })
}

pub(crate) async fn get_user_list(
    pool: &sqlx::PgPool,
    query: GetUserListRequest,
    actor_user_id: Option<i64>,
) -> Result<(Vec<UserInfoView>, i64), LoginError> {
    let scope_filter = match actor_user_id {
        Some(user_id) => crate::access::resolve_user_data_scope(pool, user_id, "users").await?,
        None => DataScopeFilter::All,
    };
    get_user_list_with_scope(pool, query, scope_filter).await
}

async fn get_user_list_with_scope(
    pool: &sqlx::PgPool,
    query: GetUserListRequest,
    scope_filter: DataScopeFilter,
) -> Result<(Vec<UserInfoView>, i64), LoginError> {
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
    if matches!(&scope_filter, DataScopeFilter::DeptIds(dept_ids) if dept_ids.is_empty()) {
        return Ok((Vec::new(), 0));
    }
    let scope_clause = scope_sql_clause(&scope_filter);

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
    total_query = match &scope_filter {
        DataScopeFilter::All => total_query,
        DataScopeFilter::DeptIds(dept_ids) => total_query.bind(dept_ids),
        DataScopeFilter::Owner(owner_id) => total_query.bind(owner_id),
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
        limit_placeholder = if matches!(&scope_filter, DataScopeFilter::All) {
            5
        } else {
            6
        },
        offset_placeholder = if matches!(&scope_filter, DataScopeFilter::All) {
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
    rows_query = match &scope_filter {
        DataScopeFilter::All => rows_query,
        DataScopeFilter::DeptIds(dept_ids) => rows_query.bind(dept_ids),
        DataScopeFilter::Owner(owner_id) => rows_query.bind(owner_id),
    };
    let rows = rows_query
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;

    let mut list = Vec::with_capacity(rows.len());
    for record in rows {
        let roles = get_roles_by_user_id(pool, record.id).await?;
        list.push(build_user_info(&record, roles));
    }

    Ok((list, total))
}

fn scope_sql_clause(filter: &DataScopeFilter) -> &'static str {
    match filter {
        DataScopeFilter::All => "",
        DataScopeFilter::DeptIds(_) => "and u.dept_id = any($5)",
        DataScopeFilter::Owner(_) => "and u.id = $5",
    }
}

pub(crate) async fn ensure_user_in_scope(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    target_user_id: i64,
) -> Result<(), LoginError> {
    let filter = crate::access::resolve_user_data_scope(pool, actor_user_id, "users").await?;
    let visible = match filter {
        DataScopeFilter::All => true,
        DataScopeFilter::Owner(owner_id) => owner_id == target_user_id,
        DataScopeFilter::DeptIds(dept_ids) => {
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
        Err(LoginError::UserNotFound)
    }
}

pub(crate) async fn update_user(
    pool: &sqlx::PgPool,
    payload: UpdateUserRequest,
) -> Result<(), LoginError> {
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
    .bind(payload.id)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn delete_user(
    pool: &sqlx::PgPool,
    payload: DeleteUserRequest,
) -> Result<(), LoginError> {
    sqlx::query("delete from sys_users where id = $1")
        .bind(payload.id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn reset_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    payload: ResetPasswordRequest,
) -> Result<(), LoginError> {
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
    .bind(payload.id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn set_user_roles(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetUserRolesRequest,
) -> Result<(), LoginError> {
    let role_ids = normalize_role_ids(Some(&payload.role_ids))?;
    ensure_assignable_roles(pool, &role_ids).await?;
    replace_user_roles(pool, user_id, role_ids).await?;
    Ok(())
}

pub(crate) async fn change_password(
    pool: &sqlx::PgPool,
    password_service: &PasswordService,
    user_id: i64,
    payload: ChangePasswordRequest,
) -> Result<(), LoginError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(LoginError::UserNotFound)?;
    let verified = password_service.verify_password(&payload.password, &record.password_hash)?;
    if !verified {
        return Err(LoginError::InvalidPassword);
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
) -> Result<(), LoginError> {
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

pub(crate) async fn set_self_setting(
    pool: &sqlx::PgPool,
    user_id: i64,
    payload: SetSelfSettingRequest,
) -> Result<(), LoginError> {
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

async fn load_user_info(pool: &sqlx::PgPool, user_id: i64) -> Result<UserInfoView, LoginError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(LoginError::UserNotFound)?;
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

async fn get_roles_by_user_id(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<Vec<RoleSummary>, sqlx::Error> {
    sqlx::query_as::<_, RoleSummary>(
        r#"
        select r.id, r.code, r.name, r.status, r.sort, r.data_scope, r.is_system
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        where ur.user_id = $1 and r.status = 'enabled'
        order by r.sort, r.id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

async fn replace_user_roles(
    pool: &sqlx::PgPool,
    user_id: i64,
    role_ids: Vec<i64>,
) -> Result<(), LoginError> {
    let mut tx = pool.begin().await?;
    sqlx::query("delete from sys_user_roles where user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    for role_id in role_ids {
        sqlx::query(
            r#"
            insert into sys_user_roles (user_id, role_id)
            values ($1, $2)
            on conflict do nothing
            "#,
        )
        .bind(user_id)
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

fn normalize_role_ids(role_ids: Option<&Vec<i64>>) -> Result<Vec<i64>, LoginError> {
    let ids = role_ids.cloned().unwrap_or_default();
    let ids = ids
        .into_iter()
        .filter(|id| *id > 0)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if ids.is_empty() {
        return Err(LoginError::InvalidRoles);
    }
    Ok(ids)
}

async fn ensure_assignable_roles(pool: &sqlx::PgPool, role_ids: &[i64]) -> Result<(), LoginError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM sys_roles WHERE id = ANY($1) AND status = 'enabled'",
    )
    .bind(role_ids)
    .fetch_one(pool)
    .await?;
    if count != role_ids.len() as i64 {
        return Err(LoginError::InvalidRoles);
    }
    Ok(())
}

async fn ensure_role_assignment_actor(
    pool: &sqlx::PgPool,
    actor_user_id: i64,
    role_ids: &[i64],
) -> Result<(), LoginError> {
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
        Err(LoginError::InvalidRoles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_sql_clause_matches_filter_shape() {
        assert_eq!(scope_sql_clause(&DataScopeFilter::All), "");
        assert_eq!(
            scope_sql_clause(&DataScopeFilter::DeptIds(vec![1, 2])),
            "and u.dept_id = any($5)"
        );
        assert_eq!(
            scope_sql_clause(&DataScopeFilter::Owner(7)),
            "and u.id = $5"
        );
    }

    #[test]
    fn normalize_role_ids_requires_explicit_roles() {
        assert!(matches!(
            normalize_role_ids(None),
            Err(LoginError::InvalidRoles)
        ));
        assert_eq!(
            normalize_role_ids(Some(&vec![2, 1, 2])).unwrap(),
            vec![1, 2]
        );
    }
}
