use std::collections::{BTreeSet, HashMap};

use audit::AuditContext;
use uuid::Uuid;

use super::{
    AccountAccessView, AccountError, CreateAccountInput, EffectivePermissionSource,
    EffectiveRoleSource, LoginAccount, RefreshIdentity, RefreshIdentityError, SetSelfInfoRequest,
    SetSelfSettingRequest, UpdateUserInput, UserInfoView, UserListQuery, UserRecord,
};
use crate::{
    authorization::{Authorization, ReplaceUserRoles},
    roles::RoleSummary,
};

const HEADER_IMG: &str = "";

#[derive(Clone)]
pub struct Accounts {
    pool: sqlx::PgPool,
    authorization: Authorization,
}

#[derive(Debug, Clone, Copy)]
enum AccountBoundary {
    All,
    Department(i64),
    SelfOnly(i64),
}

impl Accounts {
    pub(crate) fn new(pool: sqlx::PgPool, authorization: Authorization) -> Self {
        Self {
            pool,
            authorization,
        }
    }

    pub async fn list(
        &self,
        actor_user_id: i64,
        query: UserListQuery,
    ) -> Result<(Vec<UserInfoView>, i64), AccountError> {
        let boundary = self.boundary(actor_user_id).await?;
        let include_roles = matches!(boundary, AccountBoundary::All);
        get_user_list(
            &self.pool,
            &self.authorization,
            query,
            boundary,
            include_roles,
        )
        .await
    }

    pub async fn info(&self, user_id: i64) -> Result<UserInfoView, AccountError> {
        let include_roles = self.authorization.is_active_super_admin(user_id).await?;
        load_user_info(&self.pool, &self.authorization, user_id, include_roles).await
    }

    pub async fn ensure_admin(
        &self,
        username: &str,
        password_hash: String,
        nickname: &str,
    ) -> Result<(), AccountError> {
        let user_id = ensure_admin_user(&self.pool, username, password_hash, nickname).await?;
        self.authorization.set_user_status(user_id, true).await;
        self.authorization
            .ensure_bootstrap_role(user_id, super_admin_role_id(&self.pool).await?)
            .await?;
        Ok(())
    }

    pub async fn create(
        &self,
        actor_user_id: i64,
        payload: CreateAccountInput,
        audit_context: AuditContext,
    ) -> Result<(), AccountError> {
        if find_by_username(&self.pool, &payload.user_name)
            .await?
            .is_some()
        {
            return Err(AccountError::AlreadyExists);
        }

        let boundary = self.boundary(actor_user_id).await?;
        let role_ids = payload.role_ids.unwrap_or_default();
        let dept_id = match boundary {
            AccountBoundary::All => payload.dept_id.or(Some(1)),
            AccountBoundary::Department(actor_dept_id) => {
                let target_dept_id = payload.dept_id.unwrap_or(actor_dept_id);
                if target_dept_id != actor_dept_id || !role_ids.is_empty() {
                    return Err(AccountError::AccessDenied);
                }
                Some(target_dept_id)
            }
            AccountBoundary::SelfOnly(_) => return Err(AccountError::AccessDenied),
        };
        let role_ids = self
            .authorization
            .prepare_initial_user_roles(actor_user_id, &role_ids)
            .await?;
        let enabled = payload.enable.unwrap_or(1) == 1;

        let user_id: i64 = sqlx::query_scalar(
            r#"
            insert into sys_users (
                uuid, username, password_hash, nick_name, header_img, home_route,
                enable, phone, email, origin_setting, dept_id
            ) values ($1, $2, $3, $4, $5, 'dashboard', $6, $7, $8, null, $9)
            returning id
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(&payload.user_name)
        .bind(payload.password_hash)
        .bind(&payload.nick_name)
        .bind(payload.header_img.unwrap_or_else(|| HEADER_IMG.to_string()))
        .bind(enabled)
        .bind(payload.phone)
        .bind(payload.email)
        .bind(dept_id)
        .fetch_one(&self.pool)
        .await?;
        self.authorization.set_user_status(user_id, enabled).await;
        self.authorization
            .assign_initial_user_roles(user_id, role_ids, audit_context)
            .await?;
        Ok(())
    }

    pub async fn login_account(
        &self,
        username: &str,
    ) -> Result<Option<LoginAccount>, AccountError> {
        Ok(find_by_username(&self.pool, username)
            .await?
            .map(|account| LoginAccount {
                id: account.id,
                username: account.username,
                password_hash: account.password_hash,
                enable: account.enable,
            }))
    }

    pub async fn refresh_identity(
        &self,
        user_id: i64,
    ) -> Result<RefreshIdentity, RefreshIdentityError> {
        let identity = sqlx::query_as::<_, (String, bool)>(
            "select username, enable from sys_users where id = $1",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(RefreshIdentityError::NotFound)?;
        if !identity.1 {
            return Err(RefreshIdentityError::Disabled);
        }
        Ok(RefreshIdentity {
            username: identity.0,
        })
    }

    pub async fn password_hash(&self, user_id: i64) -> Result<String, AccountError> {
        Ok(find_by_id(&self.pool, user_id)
            .await?
            .ok_or(AccountError::NotFound)?
            .password_hash)
    }

    pub async fn update(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        payload: UpdateUserInput,
    ) -> Result<(), AccountError> {
        self.require_visible_user(actor_user_id, target_user_id)
            .await?;
        if let Some(target_department_id) = payload.dept_id {
            match self.boundary(actor_user_id).await? {
                AccountBoundary::All => {}
                AccountBoundary::Department(actor_department_id)
                    if actor_department_id == target_department_id => {}
                AccountBoundary::Department(_) | AccountBoundary::SelfOnly(_) => {
                    return Err(AccountError::AccessDenied);
                }
            }
        }
        let enabled = payload.enable == 1;
        let updated = sqlx::query(
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
        .bind(enabled)
        .bind(payload.phone)
        .bind(payload.email)
        .bind(payload.dept_id)
        .bind(target_user_id)
        .execute(&self.pool)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(AccountError::NotFound);
        }
        self.authorization
            .set_user_status(target_user_id, enabled)
            .await;
        Ok(())
    }

    pub async fn update_current_user(
        &self,
        user_id: i64,
        payload: SetSelfInfoRequest,
    ) -> Result<(), AccountError> {
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
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_current_user_settings(
        &self,
        user_id: i64,
        payload: SetSelfSettingRequest,
    ) -> Result<(), AccountError> {
        sqlx::query("update sys_users set origin_setting = $1, updated_at = now() where id = $2")
            .bind(payload.origin_setting)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
    ) -> Result<(), AccountError> {
        self.require_visible_user(actor_user_id, target_user_id)
            .await?;
        self.authorization.remove_user(target_user_id).await?;
        let deleted = sqlx::query("delete from sys_users where id = $1")
            .bind(target_user_id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(AccountError::NotFound);
        }
        self.authorization.notify_policy_changed();
        Ok(())
    }

    pub async fn require_visible_user(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
    ) -> Result<(), AccountError> {
        let visible = match self.boundary(actor_user_id).await? {
            AccountBoundary::All => {
                sqlx::query_scalar::<_, bool>(
                    "select exists(select 1 from sys_users where id = $1)",
                )
                .bind(target_user_id)
                .fetch_one(&self.pool)
                .await?
            }
            AccountBoundary::Department(dept_id) => {
                sqlx::query_scalar::<_, bool>(
                    "select exists(select 1 from sys_users where id = $1 and dept_id = $2)",
                )
                .bind(target_user_id)
                .bind(dept_id)
                .fetch_one(&self.pool)
                .await?
            }
            AccountBoundary::SelfOnly(user_id) => user_id == target_user_id,
        };
        if visible {
            Ok(())
        } else {
            Err(AccountError::NotFound)
        }
    }

    pub async fn update_password(
        &self,
        user_id: i64,
        password_hash: String,
    ) -> Result<(), AccountError> {
        sqlx::query("update sys_users set password_hash = $1, updated_at = now() where id = $2")
            .bind(password_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn access(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
    ) -> Result<AccountAccessView, AccountError> {
        self.authorization
            .require_access_manager(actor_user_id, target_user_id)
            .await?;
        let role_ids = self.authorization.user_role_ids(target_user_id).await?;
        let assigned_roles = load_roles(&self.pool, &role_ids).await?;
        let active_role_ids = self
            .authorization
            .active_user_role_ids(target_user_id)
            .await?;
        let effective_permissions = self
            .authorization
            .effective_permission_grants(&active_role_ids)
            .await?
            .into_iter()
            .map(|grant| EffectivePermissionSource {
                permission: grant.permission,
                roles: grant
                    .roles
                    .into_iter()
                    .map(|role| EffectiveRoleSource {
                        id: role.id,
                        code: role.code,
                        name: role.name,
                    })
                    .collect(),
            })
            .collect();
        Ok(AccountAccessView {
            assigned_roles,
            effective_permissions,
        })
    }

    pub async fn replace_roles(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
        role_ids: Vec<i64>,
        audit_context: AuditContext,
    ) -> Result<(), AccountError> {
        self.authorization
            .replace_user_roles(ReplaceUserRoles {
                actor_user_id,
                user_id: target_user_id,
                role_ids,
                audit_context,
            })
            .await?;
        Ok(())
    }

    async fn boundary(&self, actor_user_id: i64) -> Result<AccountBoundary, AccountError> {
        let (enable, dept_id) = sqlx::query_as::<_, (bool, Option<i64>)>(
            "select enable, dept_id from sys_users where id = $1",
        )
        .bind(actor_user_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AccountError::NotFound)?;
        if !enable {
            return Err(AccountError::AccessDenied);
        }
        if self
            .authorization
            .is_active_super_admin(actor_user_id)
            .await?
        {
            Ok(AccountBoundary::All)
        } else if let Some(dept_id) = dept_id {
            Ok(AccountBoundary::Department(dept_id))
        } else {
            Ok(AccountBoundary::SelfOnly(actor_user_id))
        }
    }
}

async fn ensure_admin_user(
    pool: &sqlx::PgPool,
    username: &str,
    password_hash: String,
    nick_name: &str,
) -> Result<i64, sqlx::Error> {
    super_admin_role_id(pool).await?;
    if let Some(existing) = find_by_username(pool, username).await? {
        sqlx::query(
            r#"
            update sys_users
            set nick_name = $1, home_route = 'dashboard', enable = true, dept_id = 1,
                updated_at = now()
            where id = $2
            "#,
        )
        .bind(nick_name)
        .bind(existing.id)
        .execute(pool)
        .await?;
        Ok(existing.id)
    } else {
        Ok(sqlx::query_scalar(
            r#"
            insert into sys_users (
                uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id
            ) values ($1, $2, $3, $4, $5, 'dashboard', true, 1)
            returning id
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(username)
        .bind(password_hash)
        .bind(nick_name)
        .bind(HEADER_IMG)
        .fetch_one(pool)
        .await?)
    }
}

async fn super_admin_role_id(pool: &sqlx::PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("select id from sys_roles where code = 'super_admin' and status = 'enabled'")
        .fetch_one(pool)
        .await
}

async fn get_user_list(
    pool: &sqlx::PgPool,
    authorization: &Authorization,
    query: UserListQuery,
    boundary: AccountBoundary,
    include_roles: bool,
) -> Result<(Vec<UserInfoView>, i64), AccountError> {
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
    let (all_departments, department_id, self_user_id) = match boundary {
        AccountBoundary::All => (true, None, None),
        AccountBoundary::Department(dept_id) => (false, Some(dept_id), None),
        AccountBoundary::SelfOnly(user_id) => (false, None, Some(user_id)),
    };
    let filter = r#"
        where (
              $1::text is null
              or u.username ilike '%' || $1 || '%'
              or u.nick_name ilike '%' || $1 || '%'
              or coalesce(u.phone, '') ilike '%' || $1 || '%'
              or coalesce(u.email, '') ilike '%' || $1 || '%'
          )
          and ($2::text is null or u.username ilike '%' || $2 || '%')
          and ($3::text is null or u.nick_name ilike '%' || $3 || '%')
          and ($4::text is null or coalesce(u.phone, '') ilike '%' || $4 || '%')
          and ($5::text is null or coalesce(u.email, '') ilike '%' || $5 || '%')
          and ($6 or ($7::bigint is not null and u.dept_id = $7) or u.id = $8)
    "#;
    let total_sql = format!("select count(*) from sys_users u {filter}");
    let total = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(total_sql))
        .bind(query.keyword.as_deref())
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref())
        .bind(all_departments)
        .bind(department_id)
        .bind(self_user_id)
        .fetch_one(pool)
        .await?;
    let rows_sql = format!(
        r#"
        select u.id, u.uuid, u.username, u.password_hash, u.nick_name, u.header_img,
               u.home_route, u.enable, u.phone, u.email, u.origin_setting, u.dept_id,
               d.name as dept_name
        from sys_users u
        left join sys_depts d on d.id = u.dept_id
        {filter}
        order by {order_key} {order_dir}
        limit $9 offset $10
        "#
    );
    let rows = sqlx::query_as::<_, UserRecord>(sqlx::AssertSqlSafe(rows_sql))
        .bind(query.keyword.as_deref())
        .bind(query.username.as_deref())
        .bind(query.nick_name.as_deref())
        .bind(query.phone.as_deref())
        .bind(query.email.as_deref())
        .bind(all_departments)
        .bind(department_id)
        .bind(self_user_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    let mut roles_by_user_id = if include_roles {
        let user_ids = rows.iter().map(|record| record.id).collect::<Vec<_>>();
        get_roles_by_user_ids(pool, authorization, &user_ids).await?
    } else {
        HashMap::new()
    };
    let list = rows
        .into_iter()
        .map(|record| {
            let roles = roles_by_user_id.remove(&record.id).unwrap_or_default();
            build_user_info(&record, roles)
        })
        .collect();
    Ok((list, total))
}

async fn find_by_username(
    pool: &sqlx::PgPool,
    username: &str,
) -> Result<Option<UserRecord>, sqlx::Error> {
    sqlx::query_as::<_, UserRecord>(
        r#"
        select u.id, u.uuid, u.username, u.password_hash, u.nick_name, u.header_img,
               u.home_route, u.enable, u.phone, u.email, u.origin_setting, u.dept_id,
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
        select u.id, u.uuid, u.username, u.password_hash, u.nick_name, u.header_img,
               u.home_route, u.enable, u.phone, u.email, u.origin_setting, u.dept_id,
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

async fn load_user_info(
    pool: &sqlx::PgPool,
    authorization: &Authorization,
    user_id: i64,
    include_roles: bool,
) -> Result<UserInfoView, AccountError> {
    let record = find_by_id(pool, user_id)
        .await?
        .ok_or(AccountError::NotFound)?;
    let roles = if include_roles {
        get_roles_by_user_ids(pool, authorization, &[user_id])
            .await?
            .remove(&user_id)
            .unwrap_or_default()
    } else {
        Vec::new()
    };
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

async fn get_roles_by_user_ids(
    pool: &sqlx::PgPool,
    authorization: &Authorization,
    user_ids: &[i64],
) -> Result<HashMap<i64, Vec<RoleSummary>>, AccountError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let mut memberships = HashMap::new();
    for user_id in user_ids {
        memberships.insert(*user_id, authorization.user_role_ids(*user_id).await?);
    }
    let role_ids = memberships
        .values()
        .flatten()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let roles = sqlx::query_as::<_, RoleSummary>(
        "select id, code, name, status, sort from sys_roles where id = any($1) order by sort, id",
    )
    .bind(&role_ids)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|role| (role.id, role))
    .collect::<HashMap<_, _>>();
    Ok(memberships
        .into_iter()
        .map(|(user_id, role_ids)| {
            let mut user_roles = role_ids
                .into_iter()
                .filter_map(|role_id| roles.get(&role_id).cloned())
                .collect::<Vec<_>>();
            user_roles.sort_by_key(|role| (role.sort, role.id));
            (user_id, user_roles)
        })
        .collect())
}

async fn load_roles(
    pool: &sqlx::PgPool,
    role_ids: &[i64],
) -> Result<Vec<RoleSummary>, sqlx::Error> {
    if role_ids.is_empty() {
        return Ok(Vec::new());
    }
    sqlx::query_as(
        "select id, code, name, status, sort from sys_roles where id = any($1) order by sort, id",
    )
    .bind(role_ids)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_boundary_has_no_child_department_variant() {
        assert!(matches!(
            AccountBoundary::Department(7),
            AccountBoundary::Department(7)
        ));
    }
}
