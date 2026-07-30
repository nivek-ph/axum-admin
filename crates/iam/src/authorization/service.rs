//! Casbin-backed authorization implementation.

use std::{
    collections::{BTreeSet, HashMap},
    time::Duration,
};

use audit::{
    AuditAction, AuditContext, AuditEvent, AuditReason, AuditResource, AuditResult, AuditService,
    AuditValue, FieldChange,
};
use sqlx::{PgConnection, PgPool, Postgres, Transaction};
use tokio::sync::MutexGuard;

use super::{
    engine::EnforcementEngine,
    store::{
        PolicyStore, ensure_user_in_scope, known_permissions_in, lock_policy_table, normalize_ids,
        remove_role_in, remove_user_in, replace_role_permissions_in, replace_role_users_in,
        replace_user_roles_in, role_is_system_for_update, role_subject, roles_are_assignable_in,
        user_subject, users_exist_in,
    },
};
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
    ) -> Result<Vec<i64>, AuthorizationError> {
        Ok(replace_user_roles_in(self.connection(), user_id, role_ids).await?)
    }

    pub(crate) async fn replace_initial_user_roles(
        &mut self,
        actor_user_id: i64,
        user_id: i64,
        role_ids: &[i64],
    ) -> Result<(), InitialMembershipError> {
        let role_ids = normalize_ids(role_ids);
        if role_ids.is_empty()
            || !roles_are_assignable_in(self.connection(), actor_user_id, &role_ids)
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
        // Persistence is already committed. Keep that success distinct from a
        // transient local reload failure; enforcement remains fail-closed and
        // the watcher/periodic reload will restore the in-memory state.
        let _ = authorization
            .engine
            .reload_locked(&authorization.store)
            .await;
        Ok(())
    }
}

impl Authorization {
    pub fn new(pool: PgPool) -> Self {
        Self {
            store: PolicyStore::new(pool),
            engine: EnforcementEngine::new(),
        }
    }

    pub async fn load(pool: PgPool) -> Result<Self, AuthorizationError> {
        let store = PolicyStore::new(pool);
        let engine = EnforcementEngine::load(&store).await?;
        Ok(Self { store, engine })
    }

    pub fn start_redis_watcher(&self, redis_url: &str) -> Result<(), AuthorizationError> {
        self.engine
            .start_redis_watcher(self.store.clone(), redis_url)
    }

    pub fn start_periodic_reload(&self, interval: Duration) {
        self.engine
            .start_periodic_reload(self.store.clone(), interval);
    }

    pub async fn replace_role_permissions(
        &self,
        role_id: i64,
        permissions: Vec<String>,
    ) -> Result<(), PolicyAdministrationError> {
        let permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        let mut mutation = self.begin_mutation().await?;
        let system_role = role_is_system_for_update(mutation.connection(), role_id)
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
        replace_role_permissions_in(mutation.connection(), role_id, permissions).await?;
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
        if !roles_are_assignable_in(mutation.connection(), request.actor_user_id, &role_ids).await?
        {
            return Err(PolicyAdministrationError::InvalidRoleAssignment);
        }
        ensure_user_in_scope(mutation.connection(), request.user_id, &request.data_scope).await?;
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
        AuditService::new(self.store.pool().clone())
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
        let system_role = role_is_system_for_update(mutation.connection(), role_id)
            .await?
            .ok_or(PolicyAdministrationError::RoleNotFound)?;
        if system_role {
            return Err(PolicyAdministrationError::RoleImmutable);
        }
        if !users_exist_in(mutation.connection(), &user_ids).await? {
            return Err(PolicyAdministrationError::InvalidUserAssignment);
        }
        replace_role_users_in(mutation.connection(), role_id, &user_ids).await?;
        Ok(mutation.commit().await?)
    }

    pub async fn role_permissions(
        &self,
        role_id: i64,
    ) -> Result<RolePermissionPolicy, PolicyAdministrationError> {
        let system_managed = self
            .store
            .role_is_system(role_id)
            .await?
            .ok_or(PolicyAdministrationError::RoleNotFound)?;
        let permissions = if system_managed {
            self.store.enabled_permissions().await?
        } else {
            let permissions = self.store.role_permissions(role_id).await?;
            let permission_set = permissions.iter().cloned().collect();
            if !self.store.known_permissions(&permission_set).await? {
                return Err(PolicyAdministrationError::Authorization(
                    AuthorizationError::Configuration(
                        "persisted role permission does not exist in the access catalog"
                            .to_string(),
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

    pub async fn user_role_ids(&self, user_id: i64) -> Result<Vec<i64>, PolicyAdministrationError> {
        if !self.store.user_exists(user_id).await? {
            return Err(PolicyAdministrationError::UserNotFound);
        }
        Ok(self.store.user_role_ids(user_id).await?)
    }

    pub async fn active_user_role_ids(&self, user_id: i64) -> Result<Vec<i64>, AuthorizationError> {
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

    pub async fn effective_permissions(
        &self,
        user_id: i64,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let active_role_ids = self.active_user_role_ids(user_id).await?;
        self.effective_permissions_for(user_id, &active_role_ids)
            .await
    }

    pub async fn role_user_ids(&self, role_id: i64) -> Result<Vec<i64>, PolicyAdministrationError> {
        if !self.store.role_exists(role_id).await? {
            return Err(PolicyAdministrationError::RoleNotFound);
        }
        Ok(self.store.role_user_ids(role_id).await?)
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
        let subject = user_subject(user_id);
        let active_roles = active_role_ids
            .iter()
            .copied()
            .map(role_subject)
            .collect::<Vec<_>>();
        self.engine
            .enforce(&self.store, subject, permission, active_roles)
            .await
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

#[cfg(test)]
mod tests {
    use super::*;

    async fn reload(authorization: &Authorization) -> Result<(), AuthorizationError> {
        authorization.engine.reload(&authorization.store).await
    }

    async fn replace_membership(
        authorization: &Authorization,
        user_id: i64,
        role_ids: impl IntoIterator<Item = i64>,
    ) {
        let mut mutation = authorization.begin_mutation().await.unwrap();
        mutation
            .replace_user_roles(user_id, role_ids.into_iter().collect())
            .await
            .unwrap();
        mutation.commit().await.unwrap();
    }

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
            .replace_role_permissions(2, vec!["system:user:list".to_string()])
            .await
            .unwrap();
        replace_membership(&authorization, 100, [2]).await;

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
            .replace_role_permissions(2, vec!["system:user:list".to_string()])
            .await
            .unwrap();
        replace_membership(&authorization, 100, [2]).await;

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
        reload(&authorization).await.unwrap();
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
                vec![
                    "system:user:list".to_string(),
                    "system:role:list".to_string(),
                ],
            )
            .await
            .unwrap();
        authorization
            .replace_role_permissions(2, vec!["system:role:list".to_string()])
            .await
            .unwrap();
        replace_membership(&authorization, 100, [2]).await;

        let restarted = Authorization::load(pool).await.unwrap();
        assert_eq!(
            restarted.role_permissions(2).await.unwrap().permissions,
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
        assert!(second.start_redis_watcher("not-a-redis-url").is_err());

        first
            .replace_role_permissions(2, vec!["system:user:list".to_string()])
            .await
            .unwrap();
        replace_membership(&first, 100, [2]).await;

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
            .replace_role_permissions(2, vec!["system:user:list".to_string()])
            .await
            .unwrap();
        replace_membership(&authorization, 100, [2]).await;
        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );

        pool.close().await;
        assert!(reload(&authorization).await.is_err());
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
        authorization.start_periodic_reload(Duration::from_millis(20));
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

        sqlx::query("delete from casbin_rule where ptype = 'invalid'")
            .execute(&pool)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if crate::authorization::engine::tests::is_available(&authorization.engine).await {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("periodic reload should restore state after post-commit reload failure");
        assert!(authorization.enforce(1, "anything").await.is_ok());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn ordinary_role_cannot_receive_wildcard(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool).await.unwrap();

        let error = authorization
            .replace_role_permissions(2, vec!["*".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            PolicyAdministrationError::InvalidPermissionAssignment
        ));
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn unknown_permission_is_rejected_before_persistence(pool: PgPool) {
        seed_user_and_role(&pool).await;
        let authorization = Authorization::load(pool.clone()).await.unwrap();

        let error = authorization
            .replace_role_permissions(2, vec!["unknown:permission".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            PolicyAdministrationError::InvalidPermissionAssignment
        ));
        assert!(
            authorization
                .role_permissions(2)
                .await
                .unwrap()
                .permissions
                .is_empty()
        );
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
                .replace_role_permissions(2, vec!["system:user:list".to_string()])
                .await
                .unwrap();
        });
        let second_write = tokio::spawn(async move {
            second
                .replace_role_permissions(2, vec!["system:role:list".to_string()])
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
            .unwrap()
            .permissions;
        assert!(
            actual == vec!["system:user:list"] || actual == vec!["system:role:list"],
            "one complete final set should win, got {actual:?}"
        );
    }
}
