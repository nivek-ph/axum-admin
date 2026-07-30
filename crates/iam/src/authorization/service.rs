//! Casbin-backed authorization implementation.

use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

use audit::{
    AuditAction, AuditContext, AuditEvent, AuditResource, AuditResult, AuditService, AuditValue,
    FieldChange,
};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use tokio::sync::MutexGuard;

use super::{
    EffectivePermissionGrant,
    engine::EnforcementEngine,
    store::{
        PolicyStore, active_super_admin_count_in, is_active_super_admin_in, known_permissions_in,
        lock_policy_table, normalize_ids, protected_role_for_update, remove_role_in,
        remove_user_in, replace_role_permissions_in, replace_user_permissions_in,
        replace_user_roles_in, role_subject, roles_are_assignable_in, roles_exist_in,
        super_admin_role_id_in, user_exists_in, user_subject,
    },
};

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
    #[error("authorization watcher could not be installed")]
    WatcherInstallation,
    #[error("authorization state is unavailable")]
    StateUnavailable,
}

#[derive(Debug, thiserror::Error)]
pub enum PolicyAdministrationError {
    #[error("role not found")]
    RoleNotFound,
    #[error("user not found")]
    UserNotFound,
    #[error("protected role is immutable")]
    RoleImmutable,
    #[error("only an active super_admin may manage employee access")]
    AccessDenied,
    #[error("the final active super_admin cannot be removed")]
    LastSuperAdmin,
    #[error("selected permissions are invalid")]
    InvalidPermissionAssignment,
    #[error("selected roles are invalid")]
    InvalidRoleAssignment,
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
    pub audit_context: AuditContext,
}

#[derive(Debug, Clone)]
pub struct ReplaceUserPermissions {
    pub actor_user_id: i64,
    pub user_id: i64,
    pub permissions: Vec<String>,
    pub audit_context: AuditContext,
}

#[derive(Clone)]
pub struct Authorization {
    store: PolicyStore,
    engine: EnforcementEngine,
}

pub(crate) struct AuthorizationMutation<'a> {
    authorization: &'a Authorization,
    transaction: Transaction<'a, Postgres>,
    _reload_guard: MutexGuard<'a, ()>,
}

pub(crate) enum InitialMembershipError {
    AccessDenied,
    InvalidRoles,
    Database(sqlx::Error),
}

impl AuthorizationMutation<'_> {
    pub(crate) fn connection(&mut self) -> &mut PgConnection {
        self.transaction.as_mut()
    }

    pub(crate) async fn replace_user_roles(
        &mut self,
        user_id: i64,
        role_ids: BTreeSet<i64>,
    ) -> Result<Vec<i64>, sqlx::Error> {
        replace_user_roles_in(self.connection(), user_id, role_ids).await
    }

    pub(crate) async fn replace_initial_user_roles(
        &mut self,
        actor_user_id: i64,
        user_id: i64,
        role_ids: &[i64],
    ) -> Result<(), InitialMembershipError> {
        let role_ids = normalize_ids(role_ids);
        if role_ids.is_empty() {
            return Ok(());
        }
        if !is_active_super_admin_in(self.connection(), actor_user_id)
            .await
            .map_err(InitialMembershipError::Database)?
        {
            return Err(InitialMembershipError::AccessDenied);
        }
        if !roles_are_assignable_in(self.connection(), &role_ids)
            .await
            .map_err(InitialMembershipError::Database)?
        {
            return Err(InitialMembershipError::InvalidRoles);
        }
        replace_user_roles_in(self.connection(), user_id, role_ids.into_iter().collect())
            .await
            .map_err(InitialMembershipError::Database)?;
        Ok(())
    }

    pub(crate) async fn ensure_account_change(
        &mut self,
        actor_user_id: i64,
        target_user_id: i64,
        next_enabled: bool,
        removing_account: bool,
    ) -> Result<(), PolicyAdministrationError> {
        let Some((currently_enabled, holds_super_admin)) = sqlx::query_as::<_, (bool, bool)>(
            r#"
                select u.enable,
                       exists(
                           select 1
                           from casbin_rule g
                           join sys_roles r on g.v1 = concat('role:', r.id)
                           where g.ptype = 'g'
                             and g.v0 = concat('user:', u.id)
                             and r.code = 'super_admin'
                       )
                from sys_users u
                where u.id = $1
                "#,
        )
        .bind(target_user_id)
        .fetch_optional(self.connection())
        .await?
        else {
            return Err(PolicyAdministrationError::UserNotFound);
        };
        if !holds_super_admin || (!removing_account && next_enabled == currently_enabled) {
            return Ok(());
        }
        require_super_admin(self.connection(), actor_user_id).await?;
        if currently_enabled
            && (!next_enabled || removing_account)
            && active_super_admin_count_in(self.connection()).await? <= 1
        {
            return Err(PolicyAdministrationError::LastSuperAdmin);
        }
        Ok(())
    }

    pub(crate) async fn remove_user(&mut self, user_id: i64) -> Result<(), AuthorizationError> {
        remove_user_in(self.connection(), user_id).await?;
        Ok(())
    }

    pub(crate) async fn remove_role(&mut self, role_id: i64) -> Result<(), AuthorizationError> {
        remove_role_in(self.connection(), role_id).await?;
        Ok(())
    }

    pub(crate) async fn commit(self) -> Result<(), AuthorizationError> {
        let Self {
            authorization,
            transaction,
            _reload_guard,
        } = self;
        transaction.commit().await?;
        authorization.engine.publish_change();
        let _ = authorization
            .engine
            .reload_locked(&authorization.store)
            .await;
        Ok(())
    }
}

impl Authorization {
    pub(crate) async fn load(pool: PgPool) -> Result<Self, AuthorizationError> {
        let store = PolicyStore::new(pool);
        let engine = EnforcementEngine::load(&store).await?;
        Ok(Self { store, engine })
    }

    pub(crate) fn start_redis_watcher(&self, redis_url: &str) -> Result<(), AuthorizationError> {
        self.engine
            .start_redis_watcher(self.store.clone(), redis_url)
    }

    pub(crate) fn start_periodic_reload(&self, interval: Duration) {
        self.engine
            .start_periodic_reload(self.store.clone(), interval);
    }

    pub(crate) async fn is_active_super_admin(
        &self,
        user_id: i64,
    ) -> Result<bool, AuthorizationError> {
        let mut connection = self.store.pool().acquire().await?;
        Ok(is_active_super_admin_in(&mut connection, user_id).await?)
    }

    pub(crate) async fn ensure_access_manager(
        &self,
        actor_user_id: i64,
        target_user_id: i64,
    ) -> Result<(), PolicyAdministrationError> {
        let mut connection = self.store.pool().acquire().await?;
        require_super_admin(&mut connection, actor_user_id).await?;
        if !self.store.user_exists(target_user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        Ok(())
    }

    pub(crate) async fn replace_role_permissions(
        &self,
        role_id: i64,
        permissions: Vec<String>,
    ) -> Result<(), PolicyAdministrationError> {
        let permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        let mut mutation = self.begin_mutation().await?;
        let protected = protected_role_for_update(mutation.connection(), role_id)
            .await?
            .ok_or(PolicyAdministrationError::RoleNotFound)?;
        if protected {
            return Err(PolicyAdministrationError::RoleImmutable);
        }
        if !known_permissions_in(mutation.connection(), &permissions).await? {
            return Err(PolicyAdministrationError::InvalidPermissionAssignment);
        }
        replace_role_permissions_in(mutation.connection(), role_id, permissions).await?;
        Ok(mutation.commit().await?)
    }

    pub(crate) async fn role_permissions(
        &self,
        role_id: i64,
    ) -> Result<Vec<String>, PolicyAdministrationError> {
        let permissions = self.store.role_permissions(role_id).await?;
        if permissions.is_empty() {
            let exists = sqlx::query_scalar::<_, bool>(
                "select exists(select 1 from sys_roles where id = $1)",
            )
            .bind(role_id)
            .fetch_one(self.store.pool())
            .await?;
            if !exists {
                return Err(PolicyAdministrationError::RoleNotFound);
            }
        }
        let permission_set = permissions.iter().cloned().collect();
        if !self.store.known_permissions(&permission_set).await? {
            return Err(PolicyAdministrationError::Authorization(
                AuthorizationError::Configuration(
                    "persisted role permission does not exist in the access catalog".to_string(),
                ),
            ));
        }
        Ok(permissions)
    }

    pub(crate) async fn replace_user_roles(
        &self,
        request: ReplaceUserRoles,
    ) -> Result<(), PolicyAdministrationError> {
        let role_ids = normalize_ids(&request.role_ids);
        let mut mutation = self.begin_mutation().await?;
        require_super_admin(mutation.connection(), request.actor_user_id).await?;
        if !user_exists_in(mutation.connection(), request.user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        if !roles_exist_in(mutation.connection(), &role_ids).await? {
            return Err(PolicyAdministrationError::InvalidRoleAssignment);
        }
        let super_admin_role_id = super_admin_role_id_in(mutation.connection()).await?;
        let target_is_active_super =
            is_active_super_admin_in(mutation.connection(), request.user_id).await?;
        if target_is_active_super
            && !role_ids.contains(&super_admin_role_id)
            && active_super_admin_count_in(mutation.connection()).await? <= 1
        {
            return Err(PolicyAdministrationError::LastSuperAdmin);
        }
        let before = mutation
            .replace_user_roles(request.user_id, role_ids.iter().copied().collect())
            .await?;
        AuditService::record_in(
            mutation.connection(),
            AuditEvent {
                req_id: request.audit_context.req_id,
                actor: request.audit_context.actor,
                action: AuditAction::AssignUserRoles,
                resource: AuditResource::User(request.user_id),
                result: AuditResult::Succeeded,
                reason_code: None,
                source: request.audit_context.source,
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

    pub(crate) async fn replace_user_permissions(
        &self,
        request: ReplaceUserPermissions,
    ) -> Result<(), PolicyAdministrationError> {
        let permissions = request.permissions.into_iter().collect::<BTreeSet<_>>();
        let mut mutation = self.begin_mutation().await?;
        require_super_admin(mutation.connection(), request.actor_user_id).await?;
        if !user_exists_in(mutation.connection(), request.user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        if !known_permissions_in(mutation.connection(), &permissions).await? {
            return Err(PolicyAdministrationError::InvalidPermissionAssignment);
        }
        let before = replace_user_permissions_in(
            mutation.connection(),
            request.user_id,
            permissions.clone(),
        )
        .await?;
        AuditService::record_in(
            mutation.connection(),
            AuditEvent {
                req_id: request.audit_context.req_id,
                actor: request.audit_context.actor,
                action: AuditAction::AssignUserDirectPermissions,
                resource: AuditResource::User(request.user_id),
                result: AuditResult::Succeeded,
                reason_code: None,
                source: request.audit_context.source,
                changes: vec![FieldChange {
                    field: "direct_permissions".to_string(),
                    before: AuditValue::Texts(before),
                    after: AuditValue::Texts(permissions.into_iter().collect()),
                }],
            },
        )
        .await?;
        Ok(mutation.commit().await?)
    }

    pub(crate) async fn user_role_ids(
        &self,
        user_id: i64,
    ) -> Result<Vec<i64>, PolicyAdministrationError> {
        if !self.store.user_exists(user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        Ok(self.store.user_role_ids(user_id).await?)
    }

    pub(crate) async fn user_permissions(
        &self,
        user_id: i64,
    ) -> Result<Vec<String>, PolicyAdministrationError> {
        if !self.store.user_exists(user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        Ok(self.store.user_permissions(user_id).await?)
    }

    pub(crate) async fn active_user_role_ids(
        &self,
        user_id: i64,
    ) -> Result<Vec<i64>, AuthorizationError> {
        Ok(self
            .active_user_role_ids_for(&[user_id])
            .await?
            .remove(&user_id)
            .unwrap_or_default())
    }

    pub(crate) async fn active_user_role_ids_for(
        &self,
        user_ids: &[i64],
    ) -> Result<HashMap<i64, Vec<i64>>, AuthorizationError> {
        Ok(self.store.active_user_role_ids_for(user_ids).await?)
    }

    pub(crate) async fn effective_permissions_for(
        &self,
        user_id: i64,
        active_role_ids: &[i64],
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        Ok(self
            .store
            .effective_permissions_for(user_id, active_role_ids)
            .await?)
    }

    pub(crate) async fn effective_permission_grants(
        &self,
        user_id: i64,
        active_role_ids: &[i64],
    ) -> Result<Vec<EffectivePermissionGrant>, AuthorizationError> {
        Ok(self
            .store
            .effective_permission_grants(user_id, active_role_ids)
            .await?)
    }

    pub(crate) async fn enforce_with_active_roles(
        &self,
        user_id: i64,
        permission: &str,
        active_role_ids: &[i64],
    ) -> Result<bool, AuthorizationError> {
        let subject = user_subject(user_id);
        let active_roles = active_role_ids
            .iter()
            .copied()
            .map(role_subject)
            .collect::<Vec<_>>();
        self.engine.enforce(subject, permission, active_roles).await
    }

    pub(crate) async fn begin_mutation(
        &self,
    ) -> Result<AuthorizationMutation<'_>, AuthorizationError> {
        let reload_guard = self.engine.lock_reload().await;
        let mut transaction = self.store.pool().begin().await?;
        lock_policy_table(transaction.as_mut()).await?;
        Ok(AuthorizationMutation {
            authorization: self,
            transaction,
            _reload_guard: reload_guard,
        })
    }
}

async fn require_super_admin(
    connection: &mut PgConnection,
    actor_user_id: i64,
) -> Result<(), PolicyAdministrationError> {
    if is_active_super_admin_in(connection, actor_user_id).await? {
        Ok(())
    } else {
        Err(PolicyAdministrationError::AccessDenied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn enforce(
        authorization: &Authorization,
        user_id: i64,
        permission: &str,
    ) -> Result<bool, AuthorizationError> {
        let active_roles = authorization.active_user_role_ids(user_id).await?;
        authorization
            .enforce_with_active_roles(user_id, permission, &active_roles)
            .await
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn direct_permission_is_enforced_without_role_membership(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (id, uuid, username, password_hash, nick_name, header_img)
            values (100, 'direct-user', 'direct-user', 'hash', 'Direct User', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('p', 'user:100', 'system:user:list', '', '', '', '')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let authorization = Authorization::load(pool).await.unwrap();

        assert!(
            enforce(&authorization, 100, "system:user:list")
                .await
                .unwrap()
        );
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn watcher_install_failure_preserves_the_loaded_enforcer(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (id, uuid, username, password_hash, nick_name, header_img)
            values (101, 'watcher-user', 'watcher-user', 'hash', 'Watcher User', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:101', 'role:1', '', '', '', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let authorization = Authorization::load(pool).await.unwrap();

        assert!(
            authorization
                .start_redis_watcher("not a redis URL")
                .is_err()
        );
        assert!(
            enforce(&authorization, 101, "system:user:list")
                .await
                .unwrap()
        );
    }
}
