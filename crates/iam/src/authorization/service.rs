//! Casbin-backed authorization implementation.

use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use audit::{
    AuditAction, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
    AuditValue, FieldChange,
};
use casbin::{CoreApi, DefaultModel, Enforcer, EventData, Watcher};
use redis_watcher::{RedisWatcher, WatcherOptions};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use sqlx_adapter::SqlxAdapter;
use tokio::sync::{Mutex, MutexGuard, RwLock};

use crate::access::ResolvedDataScope;

#[derive(Debug, thiserror::Error)]
pub enum AuthorizationError {
    #[error("authorization configuration is invalid")]
    Configuration(String),
    #[error("authorization database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("authorization policy operation failed")]
    Policy(#[from] casbin::Error),
    #[error("authorization watcher failed")]
    Watcher(#[from] redis_watcher::WatcherError),
    #[error("authorization state is unavailable")]
    StateUnavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyAdministrationError {
    #[error("role not found")]
    RoleNotFound,
    #[error("user not found")]
    UserNotFound,
    #[error("system role is immutable")]
    RoleImmutable,
    #[error("selected permissions are invalid")]
    InvalidPermissionAssignment,
    #[error("selected roles are invalid")]
    InvalidRoleAssignment,
    #[error("selected users are invalid")]
    InvalidUserAssignment,
    #[error("authorization administration database operation failed")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Audit(#[from] audit::AuditError),
    #[error(transparent)]
    Authorization(AuthorizationError),
}

impl From<AuthorizationError> for PolicyAdministrationError {
    fn from(error: AuthorizationError) -> Self {
        match error {
            AuthorizationError::Database(source) => Self::Database(source),
            source => Self::Authorization(source),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplaceUserRoles {
    pub actor_user_id: i64,
    pub user_id: i64,
    pub role_ids: Vec<i64>,
    pub data_scope: ResolvedDataScope,
    pub audit_context: AuditContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePermissionPolicy {
    pub permissions: Vec<String>,
    pub system_managed: bool,
}

struct AuthorizationState {
    enforcer: Enforcer,
}

#[derive(Clone)]
pub struct Authorization {
    pool: PgPool,
    state: Arc<RwLock<Option<AuthorizationState>>>,
    reload_lock: Arc<Mutex<()>>,
    watcher: Arc<StdMutex<Option<RedisWatcher>>>,
}

pub(crate) struct AuthorizationMutation<'a> {
    authorization: &'a Authorization,
    transaction: Option<Transaction<'a, Postgres>>,
    _reload_guard: MutexGuard<'a, ()>,
}

impl AuthorizationMutation<'_> {
    pub(crate) fn connection(&mut self) -> &mut PgConnection {
        self.transaction
            .as_mut()
            .expect("authorization mutation is open")
            .as_mut()
    }

    pub(crate) async fn replace_user_roles(
        &mut self,
        user_id: i64,
        role_ids: BTreeSet<i64>,
    ) -> Result<Vec<i64>, AuthorizationError> {
        let previous_roles = replace_grouping_policies_in(
            self.connection(),
            0,
            &user_subject(user_id),
            role_ids.into_iter().map(role_subject).collect(),
        )
        .await?;
        Ok(previous_roles
            .into_iter()
            .map(|role| subject_id(&role))
            .collect())
    }

    pub(crate) async fn remove_user(&mut self, user_id: i64) -> Result<(), AuthorizationError> {
        remove_subject_in(self.connection(), &user_subject(user_id)).await?;
        Ok(())
    }

    pub(crate) async fn remove_role(&mut self, role_id: i64) -> Result<(), AuthorizationError> {
        remove_subject_in(self.connection(), &role_subject(role_id)).await?;
        Ok(())
    }

    pub(crate) async fn commit(mut self) -> Result<(), AuthorizationError> {
        self.transaction
            .take()
            .expect("authorization mutation is open")
            .commit()
            .await?;
        self.authorization.publish_change();
        // Persistence is already committed. Keep that success distinct from a
        // transient local reload failure; enforcement remains fail-closed and
        // the watcher/periodic reload will restore the in-memory state.
        let _ = self.authorization.reload_locked().await;
        Ok(())
    }
}

impl Authorization {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            state: Arc::new(RwLock::new(None)),
            reload_lock: Arc::new(Mutex::new(())),
            watcher: Arc::new(StdMutex::new(None)),
        }
    }

    pub async fn load(pool: PgPool) -> Result<Self, AuthorizationError> {
        let state = load_state(&pool).await?;
        Ok(Self {
            pool,
            state: Arc::new(RwLock::new(Some(state))),
            reload_lock: Arc::new(Mutex::new(())),
            watcher: Arc::new(StdMutex::new(None)),
        })
    }

    pub fn start_redis_watcher(&self, redis_url: &str) -> Result<(), AuthorizationError> {
        let options = WatcherOptions::default()
            .with_channel("/ava/casbin".to_string())
            .with_ignore_self(true);
        let mut watcher = RedisWatcher::new(redis_url, options)?;
        let authorization = self.clone();
        watcher.set_update_callback(Box::new(move |_| {
            let authorization = authorization.clone();
            tokio::spawn(async move {
                let _ = authorization.reload().await;
            });
        }));
        *self.watcher.lock().expect("watcher mutex poisoned") = Some(watcher);
        Ok(())
    }

    pub fn start_periodic_reload(&self, interval: Duration) {
        let authorization = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let _ = authorization.reload().await;
            }
        });
    }

    async fn reload(&self) -> Result<(), AuthorizationError> {
        let _guard = self.reload_lock.lock().await;
        self.reload_locked().await
    }

    async fn reload_locked(&self) -> Result<(), AuthorizationError> {
        match load_state(&self.pool).await {
            Ok(state) => {
                *self.state.write().await = Some(state);
                Ok(())
            }
            Err(error) => {
                *self.state.write().await = None;
                Err(error)
            }
        }
    }

    pub async fn replace_role_permissions(
        &self,
        role_id: i64,
        permissions: Vec<String>,
    ) -> Result<(), PolicyAdministrationError> {
        let permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        let mut mutation = self.begin_mutation().await?;
        lock_policy_table(mutation.connection()).await?;
        let system_role = role_is_system_in(mutation.connection(), role_id)
            .await?
            .ok_or(PolicyAdministrationError::RoleNotFound)?;
        if system_role {
            return Err(PolicyAdministrationError::RoleImmutable);
        }
        if permissions.contains("*")
            || !known_permissions_in(mutation.connection(), &permissions).await?
        {
            return Err(PolicyAdministrationError::InvalidPermissionAssignment);
        }
        replace_subject_policies_in(mutation.connection(), &role_subject(role_id), permissions)
            .await?;
        Ok(mutation.commit().await?)
    }

    pub async fn replace_user_roles(
        &self,
        request: ReplaceUserRoles,
    ) -> Result<(), PolicyAdministrationError> {
        let result = self.replace_user_roles_with_audit(&request).await;
        if let Err(error) = &result {
            self.record_user_role_failure(&request, error).await;
        }
        result
    }

    async fn replace_user_roles_with_audit(
        &self,
        request: &ReplaceUserRoles,
    ) -> Result<(), PolicyAdministrationError> {
        let role_ids = normalize_ids(&request.role_ids);
        if role_ids.is_empty() {
            return Err(PolicyAdministrationError::InvalidRoleAssignment);
        }
        let mut mutation = self.begin_mutation().await?;
        ensure_user_in_scope(
            mutation.connection(),
            request.user_id,
            &request.data_scope,
        )
        .await?;
        ensure_assignable_roles(
            mutation.connection(),
            request.actor_user_id,
            &role_ids,
        )
        .await?;
        let before = mutation
            .replace_user_roles(request.user_id, role_ids.iter().copied().collect())
            .await?;
        AuditService::record_in(
            mutation.connection(),
            AuditEvent {
                req_id: request.audit_context.req_id.clone(),
                actor: request.audit_context.actor.clone(),
                action: AuditAction::AssignUserRoles,
                resource: AuditResource::User(request.user_id),
                result: AuditResult::Succeeded,
                reason_code: None,
                source: request.audit_context.source.clone(),
                changes: vec![FieldChange {
                    field: "role_ids".to_string(),
                    before: AuditValue::Ids(before),
                    after: AuditValue::Ids(role_ids),
                }],
            },
        )
        .await?;
        Ok(mutation.commit().await?)
    }

    async fn record_user_role_failure(
        &self,
        request: &ReplaceUserRoles,
        error: &PolicyAdministrationError,
    ) {
        let (result, reason_code) = match error {
            PolicyAdministrationError::UserNotFound
            | PolicyAdministrationError::InvalidRoleAssignment => {
                (AuditResult::Denied, AuditReason::InvalidRoleAssignment)
            }
            PolicyAdministrationError::RoleNotFound
            | PolicyAdministrationError::RoleImmutable
            | PolicyAdministrationError::InvalidPermissionAssignment
            | PolicyAdministrationError::InvalidUserAssignment
            | PolicyAdministrationError::Database(_)
            | PolicyAdministrationError::Audit(_)
            | PolicyAdministrationError::Authorization(_) => {
                (AuditResult::Failed, AuditReason::InternalError)
            }
        };
        AuditService::new(self.pool.clone())
            .record_best_effort(AuditEvent {
                req_id: request.audit_context.req_id.clone(),
                actor: request.audit_context.actor.clone(),
                action: AuditAction::AssignUserRoles,
                resource: AuditResource::User(request.user_id),
                result,
                reason_code: Some(reason_code),
                source: request.audit_context.source.clone(),
                changes: Vec::new(),
            })
            .await;
    }

    pub async fn replace_role_users(
        &self,
        role_id: i64,
        user_ids: Vec<i64>,
    ) -> Result<(), PolicyAdministrationError> {
        let user_ids = normalize_ids(&user_ids);
        let mut mutation = self.begin_mutation().await?;
        let system_role = role_is_system_in(mutation.connection(), role_id)
            .await?
            .ok_or(PolicyAdministrationError::RoleNotFound)?;
        if system_role {
            return Err(PolicyAdministrationError::RoleImmutable);
        }
        if !users_exist_in(mutation.connection(), &user_ids).await? {
            return Err(PolicyAdministrationError::InvalidUserAssignment);
        }
        replace_grouping_policies_in(
            mutation.connection(),
            1,
            &role_subject(role_id),
            user_ids.into_iter().map(user_subject).collect(),
        )
        .await?;
        Ok(mutation.commit().await?)
    }

    pub async fn role_permissions(
        &self,
        role_id: i64,
    ) -> Result<RolePermissionPolicy, PolicyAdministrationError> {
        let system_managed = sqlx::query_scalar::<_, bool>(
            "select is_system from sys_roles where id = $1",
        )
        .bind(role_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(PolicyAdministrationError::RoleNotFound)?;
        let permissions = if system_managed {
            enabled_permissions(&self.pool).await?
        } else {
            let permissions = sqlx::query_scalar(
            r#"
            select v1
            from casbin_rule
            where ptype = 'p' and v0 = $1
            order by v1
            "#,
        )
        .bind(role_subject(role_id))
        .fetch_all(&self.pool)
        .await?;
            let permission_set = permissions.iter().cloned().collect();
            if !known_permissions(&self.pool, &permission_set).await? {
                return Err(PolicyAdministrationError::Authorization(
                    AuthorizationError::Configuration(
                        "persisted role permission does not exist in the access catalog".to_string(),
                    ),
                ));
            }
            permissions
        };
        Ok(RolePermissionPolicy {
            permissions,
            system_managed,
        })
    }

    pub async fn user_role_ids(
        &self,
        user_id: i64,
    ) -> Result<Vec<i64>, PolicyAdministrationError> {
        if !user_exists(&self.pool, user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        Ok(sqlx::query_scalar(
            r#"
            select split_part(v1, ':', 2)::bigint
            from casbin_rule
            where ptype = 'g' and v0 = $1 and v1 like 'role:%'
            order by split_part(v1, ':', 2)::bigint
            "#,
        )
        .bind(user_subject(user_id))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn active_user_role_ids(&self, user_id: i64) -> Result<Vec<i64>, AuthorizationError> {
        Ok(self
            .active_user_role_ids_for(&[user_id])
            .await?
            .remove(&user_id)
            .unwrap_or_default())
    }

    pub async fn active_user_role_ids_for(
        &self,
        user_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<i64>>, AuthorizationError> {
        if user_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows = sqlx::query_as::<_, (i64, i64)>(
            r#"
            select split_part(membership.v0, ':', 2)::bigint as user_id,
                   split_part(membership.v1, ':', 2)::bigint as role_id
            from casbin_rule membership
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
             and role.status = 'enabled'
            where membership.ptype = 'g'
              and membership.v0 like 'user:%'
              and split_part(membership.v0, ':', 2)::bigint = any($1)
            order by user_id, role_id
            "#,
        )
        .bind(user_ids)
        .fetch_all(&self.pool)
        .await?;
        let mut memberships = HashMap::<i64, Vec<i64>>::new();
        for (user_id, role_id) in rows {
            memberships.entry(user_id).or_default().push(role_id);
        }
        Ok(memberships)
    }

    pub(crate) async fn effective_permissions_for(
        &self,
        user_id: i64,
        active_role_ids: &[i64],
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let subjects = std::iter::once(user_subject(user_id))
            .chain(active_role_ids.iter().copied().map(role_subject))
            .collect::<Vec<_>>();
        Ok(sqlx::query_scalar::<_, String>(
            r#"
            select distinct policy.v1
            from casbin_rule policy
            where policy.ptype = 'p'
              and policy.v0 = any($1)
            order by policy.v1
            "#,
        )
        .bind(subjects)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect())
    }

    pub async fn effective_permissions(
        &self,
        user_id: i64,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let active_role_ids = self.active_user_role_ids(user_id).await?;
        self.effective_permissions_for(user_id, &active_role_ids)
            .await
    }

    pub async fn role_user_ids(
        &self,
        role_id: i64,
    ) -> Result<Vec<i64>, PolicyAdministrationError> {
        if !role_exists(&self.pool, role_id).await? {
            return Err(PolicyAdministrationError::RoleNotFound);
        }
        Ok(sqlx::query_scalar(
            r#"
            select split_part(v0, ':', 2)::bigint
            from casbin_rule
            where ptype = 'g' and v1 = $1 and v0 like 'user:%'
            order by split_part(v0, ':', 2)::bigint
            "#,
        )
        .bind(role_subject(role_id))
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn enforce(
        &self,
        user_id: i64,
        permission: &str,
    ) -> Result<bool, AuthorizationError> {
        let active_roles = self.active_user_role_ids(user_id).await?;
        self.enforce_with_active_roles(user_id, permission, &active_roles)
            .await
    }

    pub(crate) async fn enforce_with_active_roles(
        &self,
        user_id: i64,
        permission: &str,
        active_role_ids: &[i64],
    ) -> Result<bool, AuthorizationError> {
        self.ensure_loaded().await?;
        let subject = user_subject(user_id);
        let active_roles = active_role_ids
            .iter()
            .copied()
            .map(role_subject)
            .collect::<Vec<_>>();
        let state = self.state.read().await;
        let state = state.as_ref().ok_or(AuthorizationError::StateUnavailable)?;
        Ok(state
            .enforcer
            .enforce((subject, permission, active_roles))?)
    }

    async fn ensure_loaded(&self) -> Result<(), AuthorizationError> {
        if self.state.read().await.is_some() {
            return Ok(());
        }
        let _guard = self.reload_lock.lock().await;
        self.ensure_loaded_locked().await
    }

    async fn ensure_loaded_locked(&self) -> Result<(), AuthorizationError> {
        if self.state.read().await.is_some() {
            return Ok(());
        }
        let state = load_state(&self.pool).await?;
        *self.state.write().await = Some(state);
        Ok(())
    }

    pub(crate) async fn begin_mutation(
        &self,
    ) -> Result<AuthorizationMutation<'_>, AuthorizationError> {
        let reload_guard = self.reload_lock.lock().await;
        let transaction = self.pool.begin().await?;
        Ok(AuthorizationMutation {
            authorization: self,
            transaction: Some(transaction),
            _reload_guard: reload_guard,
        })
    }

    fn publish_change(&self) {
        if let Some(watcher) = self
            .watcher
            .lock()
            .expect("watcher mutex poisoned")
            .as_mut()
        {
            watcher.update(EventData::SavePolicy(Vec::new()));
        }
    }
}

async fn load_state(pool: &PgPool) -> Result<AuthorizationState, AuthorizationError> {
    validate_policy_rows(pool).await?;
    let model = DefaultModel::from_str(
            r#"
            [request_definition]
            r = sub, perm, active_roles

            [policy_definition]
            p = sub, perm

            [role_definition]
            g = _, _

            [policy_effect]
            e = some(where (p_eft == allow))

            [matchers]
            m = (r.sub == p.sub || (g(r.sub, p.sub) && r.active_roles.contains(p.sub))) && (p.perm == "*" || r.perm == p.perm)
            "#,
        )
        .await
        .map_err(|error| AuthorizationError::Configuration(error.to_string()))?;
    let adapter = SqlxAdapter::new_with_pool(pool.clone()).await?;
    let enforcer = Enforcer::new(model, adapter).await?;
    Ok(AuthorizationState { enforcer })
}

async fn validate_policy_rows(pool: &PgPool) -> Result<(), AuthorizationError> {
    let invalid = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
            select 1
            from casbin_rule
            where not (
                (
                    ptype = 'p'
                    and v0 ~ '^(user|role):[1-9][0-9]*$'
                    and v1 <> ''
                    and v2 = '' and v3 = '' and v4 = '' and v5 = ''
                )
                or (
                    ptype = 'g'
                    and v0 ~ '^user:[1-9][0-9]*$'
                    and v1 ~ '^role:[1-9][0-9]*$'
                    and v2 = '' and v3 = '' and v4 = '' and v5 = ''
                )
            )
        )
        "#,
    )
    .fetch_one(pool)
    .await?;
    if invalid {
        return Err(AuthorizationError::Configuration(
            "persisted policy shape is invalid".to_string(),
        ));
    }
    let system_roles =
        sqlx::query_scalar::<_, i64>("select count(*) from sys_roles where is_system")
            .fetch_one(pool)
            .await?;
    if system_roles != 1 {
        return Err(AuthorizationError::Configuration(
            "exactly one system role is required".to_string(),
        ));
    }
    let invalid_reference = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
            select 1
            from casbin_rule policy
            where
                (
                    policy.ptype = 'p'
                    and policy.v0 like 'role:%'
                    and not exists (
                        select 1 from sys_roles role
                        where role.id = split_part(policy.v0, ':', 2)::bigint
                    )
                )
                or (
                    policy.ptype = 'p'
                    and policy.v0 like 'user:%'
                    and not exists (
                        select 1 from sys_users account
                        where account.id = split_part(policy.v0, ':', 2)::bigint
                    )
                )
                or (
                    policy.ptype = 'p'
                    and policy.v1 = '*'
                    and (
                        policy.v0 not like 'role:%'
                        or not exists (
                            select 1 from sys_roles role
                            where role.id = split_part(policy.v0, ':', 2)::bigint
                              and role.is_system
                        )
                    )
                )
                or (
                    policy.ptype = 'p'
                    and policy.v1 <> '*'
                    and not exists (
                        select 1 from sys_menus menu
                        where menu.permission = policy.v1
                    )
                )
                or (
                    policy.ptype = 'g'
                    and (
                        not exists (
                            select 1 from sys_users account
                            where account.id = split_part(policy.v0, ':', 2)::bigint
                        )
                        or not exists (
                            select 1 from sys_roles role
                            where role.id = split_part(policy.v1, ':', 2)::bigint
                        )
                    )
                )
        )
        "#,
    )
    .fetch_one(pool)
    .await?;
    if invalid_reference {
        return Err(AuthorizationError::Configuration(
            "persisted policy references an invalid subject".to_string(),
        ));
    }
    let system_wildcards = sqlx::query_scalar::<_, i64>(
        r#"
        select count(*)
        from casbin_rule policy
        join sys_roles role
          on policy.v0 = 'role:' || role.id::text
         and role.is_system
        where policy.ptype = 'p' and policy.v1 = '*'
        "#,
    )
    .fetch_one(pool)
    .await?;
    if system_wildcards != 1 {
        return Err(AuthorizationError::Configuration(
            "the system role must have exactly one wildcard policy".to_string(),
        ));
    }
    Ok(())
}

async fn role_is_system_in(
    connection: &mut PgConnection,
    role_id: i64,
) -> Result<Option<bool>, sqlx::Error> {
    sqlx::query_scalar("select is_system from sys_roles where id = $1 for update")
        .bind(role_id)
        .fetch_optional(connection)
        .await
}

fn normalize_ids(ids: &[i64]) -> Vec<i64> {
    ids.iter()
        .copied()
        .filter(|id| *id > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

async fn known_permissions_in(
    connection: &mut PgConnection,
    permissions: &BTreeSet<String>,
) -> Result<bool, sqlx::Error> {
    let concrete = permissions.iter().cloned().collect::<Vec<_>>();
    let count =
        sqlx::query_scalar::<_, i64>("select count(*) from sys_menus where permission = any($1)")
            .bind(&concrete)
            .fetch_one(connection)
            .await?;
    Ok(count == concrete.len() as i64)
}

async fn known_permissions(
    pool: &PgPool,
    permissions: &BTreeSet<String>,
) -> Result<bool, sqlx::Error> {
    let concrete = permissions.iter().cloned().collect::<Vec<_>>();
    let count =
        sqlx::query_scalar::<_, i64>("select count(*) from sys_menus where permission = any($1)")
            .bind(&concrete)
            .fetch_one(pool)
            .await?;
    Ok(count == concrete.len() as i64)
}

async fn enabled_permissions(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        with recursive menu_tree as (
            select id, status = 'enabled' as enabled
            from sys_menus
            where parent_id is null

            union all

            select child.id, parent.enabled and child.status = 'enabled'
            from sys_menus child
            join menu_tree parent on child.parent_id = parent.id
        )
        select distinct menu.permission
        from menu_tree
        join sys_menus menu on menu.id = menu_tree.id
        where menu_tree.enabled and menu.permission is not null
        order by menu.permission
        "#,
    )
    .fetch_all(pool)
    .await
}

async fn user_exists(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists(select 1 from sys_users where id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await
}

async fn role_exists(pool: &PgPool, role_id: i64) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("select exists(select 1 from sys_roles where id = $1)")
        .bind(role_id)
        .fetch_one(pool)
        .await
}

async fn users_exist_in(
    connection: &mut PgConnection,
    user_ids: &[i64],
) -> Result<bool, sqlx::Error> {
    let count = sqlx::query_scalar::<_, i64>(
        "select count(*) from sys_users where id = any($1)",
    )
    .bind(user_ids)
    .fetch_one(connection)
    .await?;
    Ok(count == user_ids.len() as i64)
}

async fn ensure_user_in_scope(
    connection: &mut PgConnection,
    user_id: i64,
    data_scope: &ResolvedDataScope,
) -> Result<(), PolicyAdministrationError> {
    let visible = match data_scope {
        ResolvedDataScope::All => {
            sqlx::query_scalar::<_, bool>(
                "select exists(select 1 from sys_users where id = $1 for update)",
            )
            .bind(user_id)
            .fetch_one(&mut *connection)
            .await?
        }
        ResolvedDataScope::Owner(owner_id) if *owner_id == user_id => {
            sqlx::query_scalar::<_, bool>(
                "select exists(select 1 from sys_users where id = $1 for update)",
            )
            .bind(user_id)
            .fetch_one(&mut *connection)
            .await?
        }
        ResolvedDataScope::Owner(_) => false,
        ResolvedDataScope::DeptIds(dept_ids) if dept_ids.is_empty() => false,
        ResolvedDataScope::DeptIds(dept_ids) => {
            sqlx::query_scalar::<_, bool>(
                r#"
                select exists(
                    select 1 from sys_users
                    where id = $1 and dept_id = any($2)
                    for update
                )
                "#,
            )
            .bind(user_id)
            .bind(dept_ids)
            .fetch_one(&mut *connection)
            .await?
        }
    };
    if visible {
        Ok(())
    } else {
        Err(PolicyAdministrationError::UserNotFound)
    }
}

async fn ensure_assignable_roles(
    connection: &mut PgConnection,
    actor_user_id: i64,
    role_ids: &[i64],
) -> Result<(), PolicyAdministrationError> {
    let rows = sqlx::query_as::<_, (i64, bool)>(
        r#"
        select id, is_system
        from sys_roles
        where id = any($1) and status = 'enabled'
        for update
        "#,
    )
    .bind(role_ids)
    .fetch_all(&mut *connection)
    .await?;
    if rows.len() != role_ids.len() {
        return Err(PolicyAdministrationError::InvalidRoleAssignment);
    }
    if !rows.iter().any(|(_, is_system)| *is_system) {
        return Ok(());
    }
    let actor_is_system = sqlx::query_scalar::<_, bool>(
        r#"
        select exists(
            select 1
            from casbin_rule membership
            join sys_roles role
              on membership.v1 = 'role:' || role.id::text
             and role.status = 'enabled'
             and role.is_system
            where membership.ptype = 'g'
              and membership.v0 = $1
        )
        "#,
    )
    .bind(user_subject(actor_user_id))
    .fetch_one(connection)
    .await?;
    if actor_is_system {
        Ok(())
    } else {
        Err(PolicyAdministrationError::InvalidRoleAssignment)
    }
}

fn user_subject(user_id: i64) -> String {
    format!("user:{user_id}")
}

fn role_subject(role_id: i64) -> String {
    format!("role:{role_id}")
}

async fn replace_subject_policies_in(
    connection: &mut PgConnection,
    subject: &str,
    permissions: BTreeSet<String>,
) -> Result<(), sqlx::Error> {
    lock_policy_table(connection).await?;
    lock_subject(connection, subject).await?;
    sqlx::query("delete from casbin_rule where ptype = 'p' and v0 = $1")
        .bind(subject)
        .execute(&mut *connection)
        .await?;
    for permission in permissions {
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ('p', $1, $2, '', '', '', '')
            "#,
        )
        .bind(subject)
        .bind(permission)
        .execute(&mut *connection)
        .await?;
    }
    Ok(())
}

async fn replace_grouping_policies_in(
    connection: &mut PgConnection,
    field_index: usize,
    subject: &str,
    related_subjects: BTreeSet<String>,
) -> Result<Vec<String>, sqlx::Error> {
    let (select_sql, delete_sql, insert_sql) = match field_index {
        0 => (
            "select v1 from casbin_rule where ptype = 'g' and v0 = $1 order by v1",
            "delete from casbin_rule where ptype = 'g' and v0 = $1",
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', $1, $2, '', '', '', '')",
        ),
        1 => (
            "select v0 from casbin_rule where ptype = 'g' and v1 = $1 order by v0",
            "delete from casbin_rule where ptype = 'g' and v1 = $1",
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', $2, $1, '', '', '', '')",
        ),
        _ => unreachable!("grouping policies only have two subject fields"),
    };
    lock_policy_table(connection).await?;
    match field_index {
        0 => {
            let user_id = subject_id(subject);
            let role_ids = related_subjects
                .iter()
                .map(|role| subject_id(role))
                .collect();
            lock_roles(connection, &role_ids).await?;
            lock_user(connection, user_id).await?;
        }
        1 => {
            let role_id = subject_id(subject);
            lock_role(connection, role_id).await?;
            let user_ids = related_subjects
                .iter()
                .map(|user| subject_id(user))
                .collect();
            lock_users(connection, &user_ids).await?;
        }
        _ => unreachable!("grouping policies only have two subject fields"),
    }
    let previous = sqlx::query_scalar(select_sql)
        .bind(subject)
        .fetch_all(&mut *connection)
        .await?;
    sqlx::query(delete_sql)
        .bind(subject)
        .execute(&mut *connection)
        .await?;
    for related in related_subjects {
        sqlx::query(insert_sql)
            .bind(subject)
            .bind(related)
            .execute(&mut *connection)
            .await?;
    }
    Ok(previous)
}

async fn remove_subject_in(
    connection: &mut PgConnection,
    subject: &str,
) -> Result<(), sqlx::Error> {
    lock_policy_table(connection).await?;
    lock_subject(connection, subject).await?;
    sqlx::query(
        r#"
        delete from casbin_rule
        where (ptype = 'p' and v0 = $1)
           or (ptype = 'g' and (v0 = $1 or v1 = $1))
        "#,
    )
    .bind(subject)
    .execute(connection)
    .await?;
    Ok(())
}

async fn lock_policy_table(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query("lock table casbin_rule in share row exclusive mode")
        .execute(connection)
        .await?;
    Ok(())
}

fn subject_id(subject: &str) -> i64 {
    subject
        .split_once(':')
        .and_then(|(_, id)| id.parse().ok())
        .expect("internal subjects always use typed numeric IDs")
}

async fn lock_subject(connection: &mut PgConnection, subject: &str) -> Result<(), sqlx::Error> {
    if subject.starts_with("role:") {
        lock_role(connection, subject_id(subject)).await
    } else {
        lock_user(connection, subject_id(subject)).await
    }
}

async fn lock_role(connection: &mut PgConnection, role_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("select id from sys_roles where id = $1 for update")
        .bind(role_id)
        .fetch_one(connection)
        .await?;
    Ok(())
}

async fn lock_roles(
    connection: &mut PgConnection,
    role_ids: &BTreeSet<i64>,
) -> Result<(), sqlx::Error> {
    let role_ids = role_ids.iter().copied().collect::<Vec<_>>();
    let locked = sqlx::query_scalar::<_, i64>(
        "select id from sys_roles where id = any($1) order by id for update",
    )
    .bind(&role_ids)
    .fetch_all(connection)
    .await?;
    if locked.len() == role_ids.len() {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

async fn lock_user(connection: &mut PgConnection, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("select id from sys_users where id = $1 for update")
        .bind(user_id)
        .fetch_one(connection)
        .await?;
    Ok(())
}

async fn lock_users(
    connection: &mut PgConnection,
    user_ids: &BTreeSet<i64>,
) -> Result<(), sqlx::Error> {
    let user_ids = user_ids.iter().copied().collect::<Vec<_>>();
    let locked = sqlx::query_scalar::<_, i64>(
        "select id from sys_users where id = any($1) order by id for update",
    )
    .bind(&user_ids)
    .fetch_all(connection)
    .await?;
    if locked.len() == user_ids.len() {
        Ok(())
    } else {
        Err(sqlx::Error::RowNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn seed_user_and_role(pool: &PgPool) {
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'operator', 'Operator', 'enabled', 10, 'self', false)
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img
            )
            values (100, 'user-100', 'operator', 'hash', 'Operator', '')
            "#,
        )
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn persisted_membership_inherits_role_permission(pool: PgPool) {
        seed_user_and_role(&pool).await;

        let authorization = Authorization::load(pool.clone()).await.unwrap();
        authorization
            .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
            .await
            .unwrap();
        authorization
            .replace_user_roles(100, BTreeSet::from([2]))
            .await
            .unwrap();

        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );

        let grouping_rows: i64 = sqlx::query_scalar(
            r#"
            select count(*)
            from casbin_rule
            where ptype = 'g' and v0 = 'user:100' and v1 = 'role:2'
            "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(grouping_rows, 1);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn disabled_role_is_dormant_without_disabling_direct_permission(pool: PgPool) {
        seed_user_and_role(&pool).await;
        sqlx::query("update sys_roles set status = 'disabled' where id = 2")
            .execute(&pool)
            .await
            .unwrap();

        let authorization = Authorization::load(pool.clone()).await.unwrap();
        authorization
            .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
            .await
            .unwrap();
        authorization
            .replace_user_roles(100, BTreeSet::from([2]))
            .await
            .unwrap();

        assert!(
            !authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );

        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('p', 'user:100', 'system:user:list', '', '', '', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        authorization.reload().await.unwrap();
        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn replacement_is_atomic_and_survives_restart(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        authorization
            .replace_role_permissions(
                2,
                BTreeSet::from([
                    "system:user:list".to_string(),
                    "system:role:list".to_string(),
                ]),
            )
            .await
            .unwrap();
        authorization
            .replace_role_permissions(2, BTreeSet::from(["system:role:list".to_string()]))
            .await
            .unwrap();
        authorization
            .replace_user_roles(100, BTreeSet::from([2]))
            .await
            .unwrap();

        let restarted = Authorization::load(pool).await.unwrap();
        assert_eq!(
            restarted.role_permissions(2).await.unwrap(),
            vec!["system:role:list"]
        );
        assert!(restarted.enforce(100, "system:role:list").await.unwrap());
        assert!(!restarted.enforce(100, "system:user:list").await.unwrap());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn redis_watcher_reloads_another_authorization_instance(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379/".to_string());
        let first = Authorization::load(pool.clone()).await.unwrap();
        let second = Authorization::load(pool).await.unwrap();
        first.start_redis_watcher(&redis_url).unwrap();
        second.start_redis_watcher(&redis_url).unwrap();

        first
            .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
            .await
            .unwrap();
        first
            .replace_user_roles(100, BTreeSet::from([2]))
            .await
            .unwrap();

        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if second.enforce(100, "system:user:list").await.unwrap() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("watcher should converge the second instance");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn periodic_reload_repairs_a_missed_notification(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        authorization.start_periodic_reload(Duration::from_millis(20));

        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values
                ('p', 'role:2', 'system:user:list', '', '', '', ''),
                ('g', 'user:100', 'role:2', '', '', '', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if authorization
                    .enforce(100, "system:user:list")
                    .await
                    .unwrap()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("periodic reload should repair the missed notification");
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn failed_reload_discards_the_stale_enforcer(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        authorization
            .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
            .await
            .unwrap();
        authorization
            .replace_user_roles(100, BTreeSet::from([2]))
            .await
            .unwrap();
        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );

        pool.close().await;
        assert!(authorization.reload().await.is_err());
        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .is_err()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn committed_mutation_is_not_reported_as_failed_when_reload_fails(pool: PgPool) {
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let mut mutation = authorization.begin_mutation().await.unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('invalid', '', '', '', '', '', '')",
        )
        .execute(mutation.connection())
        .await
        .unwrap();

        mutation.commit().await.unwrap();

        let persisted: i64 =
            sqlx::query_scalar("select count(*) from casbin_rule where ptype = 'invalid'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(persisted, 1);
        assert!(authorization.enforce(1, "anything").await.is_err());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ordinary_role_cannot_receive_wildcard(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool).await.unwrap();

        let error = authorization
            .replace_role_permissions(2, BTreeSet::from(["*".to_string()]))
            .await
            .unwrap_err();
        assert!(matches!(error, AuthorizationError::Configuration(_)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn unknown_permission_is_rejected_before_persistence(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();

        let error = authorization
            .replace_role_permissions(2, BTreeSet::from(["unknown:permission".to_string()]))
            .await
            .unwrap_err();
        assert!(matches!(error, AuthorizationError::Configuration(_)));
        assert!(authorization.role_permissions(2).await.unwrap().is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn missing_system_wildcard_is_configuration_error(pool: PgPool) {
        sqlx::query("delete from casbin_rule where v0 = 'role:1' and v1 = '*'")
            .execute(&pool)
            .await
            .unwrap();

        let error = match Authorization::load(pool).await {
            Ok(_) => panic!("missing system wildcard should fail authorization load"),
            Err(error) => error,
        };
        assert!(matches!(error, AuthorizationError::Configuration(_)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn malformed_persisted_policy_is_configuration_error(pool: PgPool) {
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values ('p', 'broken-subject', 'system:user:list', '', '', '', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let error = match Authorization::load(pool).await {
            Ok(_) => panic!("malformed policy should fail authorization load"),
            Err(error) => error,
        };
        assert!(matches!(error, AuthorizationError::Configuration(_)));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn concurrent_final_set_replacements_do_not_merge(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let verify_pool = pool.clone();
        let first = Authorization::load(pool.clone()).await.unwrap();
        let second = Authorization::load(pool).await.unwrap();
        let first_write = tokio::spawn(async move {
            first
                .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
                .await
                .unwrap();
        });
        let second_write = tokio::spawn(async move {
            second
                .replace_role_permissions(2, BTreeSet::from(["system:role:list".to_string()]))
                .await
                .unwrap();
        });
        first_write.await.unwrap();
        second_write.await.unwrap();

        let actual = Authorization::load(verify_pool)
            .await
            .unwrap()
            .role_permissions(2)
            .await
            .unwrap();
        assert!(
            actual == vec!["system:user:list"] || actual == vec!["system:role:list"],
            "one complete final set should win, got {actual:?}"
        );
    }
}
