use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use casbin::{CoreApi, DefaultModel, Enforcer, EventData, MgmtApi, Watcher};
use redis_watcher::{RedisWatcher, WatcherOptions};
use sqlx_adapter::SqlxAdapter;
use tokio::sync::{Mutex, RwLock};

use super::{
    AuthorizationError,
    store::{PolicyStore, parse_role_subject, parse_user_subject, role_subject, user_subject},
};

const REDIS_CHANNEL: &str = "/ava/casbin";

/// Casbin owns both the in-memory policy and Adapter persistence. Application
/// code reaches policy only through these management operations.
pub(super) struct EnforcementEngine {
    store: Arc<PolicyStore>,
    snapshot: RwLock<AuthorizationSnapshot>,
    /// Serializes all snapshot mutations (reloads and local writes) so a
    /// completed reload can never overwrite a more recent local update.
    mutation_lock: Mutex<()>,
    watcher: StdMutex<Option<SharedWatcher>>,
}

struct AuthorizationSnapshot {
    policy: Enforcer,
    users: HashMap<i64, bool>,
    enabled_role_ids: HashSet<i64>,
    super_admin_role_id: i64,
}

#[derive(Clone)]
struct SharedWatcher(Arc<StdMutex<RedisWatcher>>);

impl Watcher for SharedWatcher {
    fn set_update_callback(&mut self, callback: Box<dyn FnMut(String) + Send + Sync>) {
        if let Ok(mut watcher) = self.0.lock() {
            watcher.set_update_callback(callback);
        }
    }

    fn update(&mut self, event: EventData) {
        if let Ok(mut watcher) = self.0.lock() {
            watcher.update(event);
        }
    }
}

impl EnforcementEngine {
    pub(super) async fn load(store: Arc<PolicyStore>) -> Result<Self, AuthorizationError> {
        let snapshot = build_snapshot(&store).await?;
        Ok(Self {
            store,
            snapshot: RwLock::new(snapshot),
            mutation_lock: Mutex::new(()),
            watcher: StdMutex::new(None),
        })
    }

    pub(super) async fn start_redis_watcher(
        self: &Arc<Self>,
        redis_url: &str,
    ) -> Result<(), AuthorizationError> {
        // Network I/O (Redis connection + PING) happens outside the mutation
        // critical section; only the snapshot/watcher installation is serialized.
        let options = WatcherOptions::default()
            .with_channel(REDIS_CHANNEL.to_string())
            .with_ignore_self(true);
        let mut watcher = RedisWatcher::new(redis_url, options)?;
        let weak_engine = Arc::downgrade(self);
        watcher.set_update_callback(Box::new(move |_| {
            let Some(engine) = weak_engine.upgrade() else {
                return;
            };
            tokio::spawn(async move {
                if let Err(error) = engine.reload().await {
                    tracing::error!(error = ?error, "Casbin watcher reload failed; retaining last policy");
                }
            });
        }));
        let shared = SharedWatcher(Arc::new(StdMutex::new(watcher)));
        let _guard = self.mutation_lock.lock().await;
        self.snapshot
            .write()
            .await
            .policy
            .set_watcher(Box::new(shared.clone()));
        let mut installed = self
            .watcher
            .lock()
            .map_err(|_| AuthorizationError::WatcherInstallation)?;
        *installed = Some(shared);
        Ok(())
    }

    pub(super) fn start_periodic_reload(self: &Arc<Self>, interval: Duration) {
        let weak_engine = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let Some(engine) = weak_engine.upgrade() else {
                    return;
                };
                if let Err(error) = engine.reload().await {
                    tracing::error!(error = ?error, "periodic Casbin reload failed; retaining last policy");
                }
            }
        });
    }

    pub(super) async fn reload(&self) -> Result<(), AuthorizationError> {
        let _guard = self.mutation_lock.lock().await;
        let mut next = build_snapshot(&self.store).await?;
        if let Some(watcher) = self
            .watcher
            .lock()
            .map_err(|_| AuthorizationError::WatcherInstallation)?
            .clone()
        {
            next.policy.set_watcher(Box::new(watcher));
        }
        *self.snapshot.write().await = next;
        Ok(())
    }

    pub(super) async fn user_status(&self, user_id: i64) -> Option<bool> {
        self.snapshot.read().await.users.get(&user_id).copied()
    }

    pub(super) async fn is_active_super_admin(&self, user_id: i64) -> bool {
        let snapshot = self.snapshot.read().await;
        if snapshot.users.get(&user_id) != Some(&true) {
            return false;
        }
        let super_admin_role_id = snapshot.super_admin_role_id;
        snapshot.enabled_role_ids.contains(&super_admin_role_id)
            && snapshot
                .policy
                .get_filtered_grouping_policy(0, vec![user_subject(user_id)])
                .into_iter()
                .any(|rule| {
                    rule.get(1).and_then(|value| parse_role_subject(value))
                        == Some(super_admin_role_id)
                })
    }

    pub(super) async fn authorize_permission(
        &self,
        user_id: i64,
        permission: &str,
    ) -> Result<bool, AuthorizationError> {
        let snapshot = self.snapshot.read().await;
        let subject = user_subject(user_id);
        let active_roles = snapshot
            .policy
            .get_filtered_grouping_policy(0, vec![subject.clone()])
            .into_iter()
            .filter_map(|rule| rule.get(1).and_then(|value| parse_role_subject(value)))
            .filter(|role_id| snapshot.enabled_role_ids.contains(role_id))
            .map(role_subject)
            .collect::<Vec<_>>();
        Ok(snapshot
            .policy
            .enforce((subject, permission, active_roles))?)
    }

    pub(super) async fn role_permissions(&self, role_id: i64) -> BTreeSet<String> {
        let subject = role_subject(role_id);
        self.snapshot
            .read()
            .await
            .policy
            .get_filtered_policy(0, vec![subject])
            .into_iter()
            .filter_map(|rule| rule.get(1).cloned())
            .collect()
    }

    pub(super) async fn user_role_ids(&self, user_id: i64) -> BTreeSet<i64> {
        let subject = user_subject(user_id);
        self.snapshot
            .read()
            .await
            .policy
            .get_filtered_grouping_policy(0, vec![subject])
            .into_iter()
            .filter_map(|rule| rule.get(1).and_then(|value| parse_role_subject(value)))
            .collect()
    }

    pub(super) async fn active_user_role_ids(&self, user_id: i64) -> Vec<i64> {
        let snapshot = self.snapshot.read().await;
        let subject = user_subject(user_id);
        snapshot
            .policy
            .get_filtered_grouping_policy(0, vec![subject])
            .into_iter()
            .filter_map(|rule| rule.get(1).and_then(|value| parse_role_subject(value)))
            .filter(|role_id| snapshot.enabled_role_ids.contains(role_id))
            .collect()
    }

    pub(super) async fn role_has_members(&self, role_id: i64) -> bool {
        !self
            .snapshot
            .read()
            .await
            .policy
            .get_filtered_grouping_policy(1, vec![role_subject(role_id)])
            .is_empty()
    }

    pub(super) async fn replace_role_permissions(
        &self,
        role_id: i64,
        permissions: BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let subject = role_subject(role_id);
        let _guard = self.mutation_lock.lock().await;
        let mut snapshot = self.snapshot.write().await;
        let policy = &mut snapshot.policy;
        let before = policy
            .get_filtered_policy(0, vec![subject.clone()])
            .into_iter()
            .filter_map(|rule| rule.get(1).cloned())
            .collect::<BTreeSet<_>>();
        let additions = permissions
            .difference(&before)
            .map(|permission| vec![subject.clone(), permission.clone()])
            .collect::<Vec<_>>();
        let removals = before
            .difference(&permissions)
            .map(|permission| vec![subject.clone(), permission.clone()])
            .collect::<Vec<_>>();
        if !additions.is_empty() {
            policy.add_policies(additions).await?;
        }
        if !removals.is_empty() {
            policy.remove_policies(removals).await?;
        }
        Ok(before)
    }

    pub(super) async fn replace_user_roles(
        &self,
        user_id: i64,
        role_ids: BTreeSet<i64>,
    ) -> Result<BTreeSet<i64>, AuthorizationError> {
        let user = user_subject(user_id);
        let _guard = self.mutation_lock.lock().await;
        let mut snapshot = self.snapshot.write().await;
        let policy = &mut snapshot.policy;
        let before = policy
            .get_filtered_grouping_policy(0, vec![user.clone()])
            .into_iter()
            .filter_map(|rule| rule.get(1).and_then(|value| parse_role_subject(value)))
            .collect::<BTreeSet<_>>();
        let additions = role_ids
            .difference(&before)
            .map(|role_id| vec![user.clone(), role_subject(*role_id)])
            .collect::<Vec<_>>();
        let removals = before
            .difference(&role_ids)
            .map(|role_id| vec![user.clone(), role_subject(*role_id)])
            .collect::<Vec<_>>();
        if !additions.is_empty() {
            policy.add_grouping_policies(additions).await?;
        }
        if !removals.is_empty() {
            policy.remove_grouping_policies(removals).await?;
        }
        Ok(before)
    }

    pub(super) async fn remove_user(&self, user_id: i64) -> Result<(), AuthorizationError> {
        let _guard = self.mutation_lock.lock().await;
        let mut snapshot = self.snapshot.write().await;
        snapshot
            .policy
            .remove_filtered_grouping_policy(0, vec![user_subject(user_id)])
            .await?;
        snapshot.users.remove(&user_id);
        drop(snapshot);
        drop(_guard);
        self.notify_reload();
        Ok(())
    }

    pub(super) async fn remove_role(&self, role_id: i64) -> Result<(), AuthorizationError> {
        let subject = role_subject(role_id);
        let _guard = self.mutation_lock.lock().await;
        let mut snapshot = self.snapshot.write().await;
        snapshot
            .policy
            .remove_filtered_policy(0, vec![subject.clone()])
            .await?;
        snapshot
            .policy
            .remove_filtered_grouping_policy(1, vec![subject])
            .await?;
        snapshot.enabled_role_ids.remove(&role_id);
        drop(snapshot);
        drop(_guard);
        self.notify_reload();
        Ok(())
    }

    pub(super) async fn set_user_status(&self, user_id: i64, enabled: bool) {
        let _guard = self.mutation_lock.lock().await;
        self.snapshot.write().await.users.insert(user_id, enabled);
        drop(_guard);
        self.notify_reload();
    }

    pub(super) async fn set_role_status(&self, role_id: i64, enabled: bool) {
        let guard = self.mutation_lock.lock().await;
        let mut snapshot = self.snapshot.write().await;
        if enabled {
            snapshot.enabled_role_ids.insert(role_id);
        } else {
            snapshot.enabled_role_ids.remove(&role_id);
        }
        drop(snapshot);
        drop(guard);
        self.notify_reload();
    }

    pub(super) fn notify_reload(&self) {
        let Ok(watcher) = self.watcher.lock() else {
            tracing::error!("Casbin watcher lock is poisoned; policy reload was not published");
            return;
        };
        if let Some(mut watcher) = watcher.clone() {
            watcher.update(EventData::ClearCache);
        }
    }
}

async fn build_snapshot(store: &PolicyStore) -> Result<AuthorizationSnapshot, AuthorizationError> {
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
        m = g(r.sub, p.sub) && r.active_roles.contains(p.sub) && r.perm == p.perm
        "#,
    )
    .await
    .map_err(|error| AuthorizationError::Configuration(error.to_string()))?;
    let adapter = SqlxAdapter::new_with_pool(store.pool().clone()).await?;
    let policy = Enforcer::new(model, adapter).await?;
    let facts = store.policy_facts().await?;
    validate_loaded_policy(&policy, &facts)?;
    let enabled_role_ids = facts
        .roles
        .iter()
        .filter_map(|(id, role)| (role.status == "enabled").then_some(*id))
        .collect();
    let super_admin_role_id = facts.super_admin_role_id;
    Ok(AuthorizationSnapshot {
        policy,
        users: facts.users,
        enabled_role_ids,
        super_admin_role_id,
    })
}

fn validate_loaded_policy(
    enforcer: &Enforcer,
    facts: &super::store::PolicyFacts,
) -> Result<(), AuthorizationError> {
    let mut super_permissions = HashSet::new();
    let mut permissions_by_role = HashMap::<i64, HashSet<String>>::new();
    for rule in enforcer.get_policy() {
        if rule.len() != 2 {
            return invalid("policy rule must contain exactly a Role subject and Permission");
        }
        let Some(role_id) = parse_role_subject(&rule[0]) else {
            return invalid("policy subjects must use role:<id>");
        };
        if !facts.roles.contains_key(&role_id) {
            return invalid("policy references an unknown Access Role");
        }
        if rule[1] == "*" || !facts.enabled_permissions.contains(&rule[1]) {
            return invalid("policy references an unknown or disabled concrete Permission");
        }
        if role_id == facts.super_admin_role_id {
            super_permissions.insert(rule[1].clone());
        }
        permissions_by_role
            .entry(role_id)
            .or_default()
            .insert(rule[1].clone());
    }
    for permissions in permissions_by_role.values() {
        for permission in permissions {
            if let Some(page) = facts.action_pages.get(permission)
                && !permissions.contains(page)
            {
                return invalid("action Permission requires its owning page Permission");
            }
        }
    }
    for rule in enforcer.get_grouping_policy() {
        if rule.len() != 2 {
            return invalid("membership rule must contain exactly a User and Role subject");
        }
        let Some(user_id) = parse_user_subject(&rule[0]) else {
            return invalid("membership subjects must use user:<id>");
        };
        let Some(role_id) = parse_role_subject(&rule[1]) else {
            return invalid("membership targets must use role:<id>");
        };
        if !facts.users.contains_key(&user_id) || !facts.roles.contains_key(&role_id) {
            return invalid("membership references an unknown User Account or Access Role");
        }
    }
    if !facts.enabled_permissions.is_subset(&super_permissions) {
        return invalid("super_admin is missing concrete Catalog Permissions");
    }
    Ok(())
}

fn invalid<T>(message: &str) -> Result<T, AuthorizationError> {
    Err(AuthorizationError::Configuration(message.to_string()))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, sync::Arc, time::Duration};

    use tokio::{sync::oneshot, time::timeout};

    use super::*;

    async fn insert_user(pool: &sqlx::PgPool, id: i64, name: &str) {
        sqlx::query(
            r#"
            insert into sys_users (id, uuid, username, password_hash, nick_name, header_img, dept_id)
            values ($1, $2, $2, 'hash', $2, '', 1)
            "#,
        )
        .bind(id)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_role(pool: &sqlx::PgPool, id: i64, code: &str, status: &str) {
        sqlx::query(
            "insert into sys_roles (id, code, name, status, sort) values ($1, $2, $2, $3, 10)",
        )
        .bind(id)
        .bind(code)
        .bind(status)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn insert_policy(pool: &sqlx::PgPool, ptype: &str, left: &str, right: &str) {
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ($1, $2, $3, '', '', '', '')",
        )
        .bind(ptype)
        .bind(left)
        .bind(right)
        .execute(pool)
        .await
        .unwrap();
    }

    /// `reload` must wait while another task holds `mutation_lock`.
    /// Removing `mutation_lock` from `reload` causes this test to fail.
    #[sqlx::test(migrations = "../../migrations")]
    async fn reload_waits_for_mutation_lock(pool: sqlx::PgPool) {
        let store = Arc::new(PolicyStore::new(pool));
        let engine = Arc::new(EnforcementEngine::load(store).await.unwrap());

        let guard = engine.mutation_lock.lock().await;
        let (started_tx, started_rx) = oneshot::channel();

        let mut reload = {
            let engine = Arc::clone(&engine);
            tokio::spawn(async move {
                started_tx.send(()).unwrap();
                engine.reload().await.unwrap();
            })
        };

        started_rx.await.unwrap();
        assert!(
            timeout(Duration::from_millis(50), &mut reload)
                .await
                .is_err(),
            "reload must wait while mutation_lock is held",
        );

        drop(guard);
        timeout(Duration::from_secs(2), &mut reload)
            .await
            .expect("reload did not resume after mutation_lock was released")
            .unwrap();
    }

    /// `replace_user_roles` must wait while another task holds `mutation_lock`,
    /// and its write must be visible in the snapshot after it completes.
    /// Removing `mutation_lock` from `replace_user_roles` causes this test to fail.
    #[sqlx::test(migrations = "../../migrations")]
    async fn replace_user_roles_waits_for_mutation_lock(pool: sqlx::PgPool) {
        insert_user(&pool, 117, "target-user").await;
        insert_role(&pool, 2, "reader", "enabled").await;
        insert_policy(&pool, "p", "role:2", "system:user:list").await;

        let store = Arc::new(PolicyStore::new(pool));
        let engine = Arc::new(EnforcementEngine::load(store).await.unwrap());

        let guard = engine.mutation_lock.lock().await;
        let (started_tx, started_rx) = oneshot::channel();

        let mut mutation = {
            let engine = Arc::clone(&engine);
            tokio::spawn(async move {
                started_tx.send(()).unwrap();
                engine
                    .replace_user_roles(117, BTreeSet::from([2]))
                    .await
                    .unwrap();
            })
        };

        started_rx.await.unwrap();
        assert!(
            timeout(Duration::from_millis(50), &mut mutation)
                .await
                .is_err(),
            "replace_user_roles must wait while mutation_lock is held",
        );

        drop(guard);
        timeout(Duration::from_secs(2), &mut mutation)
            .await
            .expect("replace_user_roles did not resume after mutation_lock was released")
            .unwrap();

        assert_eq!(engine.user_role_ids(117).await, BTreeSet::from([2]));
        assert!(
            engine
                .authorize_permission(117, "system:user:list")
                .await
                .unwrap(),
            "role assignment must be visible in the active snapshot",
        );
    }

    /// `replace_role_permissions` must wait while another task holds `mutation_lock`,
    /// and its write must be visible in the snapshot after it completes.
    /// Removing `mutation_lock` from `replace_role_permissions` causes this test to fail.
    #[sqlx::test(migrations = "../../migrations")]
    async fn replace_role_permissions_waits_for_mutation_lock(pool: sqlx::PgPool) {
        insert_user(&pool, 118, "target-user").await;
        insert_role(&pool, 2, "reader", "enabled").await;
        insert_policy(&pool, "g", "user:118", "role:2").await;

        let store = Arc::new(PolicyStore::new(pool));
        let engine = Arc::new(EnforcementEngine::load(store).await.unwrap());

        assert!(
            !engine
                .authorize_permission(118, "system:user:list")
                .await
                .unwrap(),
        );

        let guard = engine.mutation_lock.lock().await;
        let (started_tx, started_rx) = oneshot::channel();

        let mut mutation = {
            let engine = Arc::clone(&engine);
            tokio::spawn(async move {
                started_tx.send(()).unwrap();
                engine
                    .replace_role_permissions(2, BTreeSet::from(["system:user:list".to_string()]))
                    .await
                    .unwrap();
            })
        };

        started_rx.await.unwrap();
        assert!(
            timeout(Duration::from_millis(50), &mut mutation)
                .await
                .is_err(),
            "replace_role_permissions must wait while mutation_lock is held",
        );

        drop(guard);
        timeout(Duration::from_secs(2), &mut mutation)
            .await
            .expect("replace_role_permissions did not resume after mutation_lock was released")
            .unwrap();

        assert_eq!(
            engine.role_permissions(2).await,
            BTreeSet::from(["system:user:list".to_string()]),
        );
        assert!(
            engine
                .authorize_permission(118, "system:user:list")
                .await
                .unwrap(),
            "permission mutation must be visible in the active snapshot",
        );
    }
}
