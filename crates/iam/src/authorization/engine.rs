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
    policy: RwLock<Enforcer>,
    reload_lock: Mutex<()>,
    watcher: StdMutex<Option<SharedWatcher>>,
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
        let policy = build_enforcer(&store).await?;
        Ok(Self {
            store,
            policy: RwLock::new(policy),
            reload_lock: Mutex::new(()),
            watcher: StdMutex::new(None),
        })
    }

    pub(super) async fn start_redis_watcher(
        self: &Arc<Self>,
        redis_url: &str,
    ) -> Result<(), AuthorizationError> {
        let _reload_guard = self.reload_lock.lock().await;
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
        self.policy
            .write()
            .await
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
        let _guard = self.reload_lock.lock().await;
        let mut next = build_enforcer(&self.store).await?;
        if let Some(watcher) = self
            .watcher
            .lock()
            .map_err(|_| AuthorizationError::WatcherInstallation)?
            .clone()
        {
            next.set_watcher(Box::new(watcher));
        }
        *self.policy.write().await = next;
        Ok(())
    }

    pub(super) async fn enforce(
        &self,
        subject: String,
        permission: &str,
        active_roles: Vec<String>,
    ) -> Result<bool, AuthorizationError> {
        Ok(self
            .policy
            .read()
            .await
            .enforce((subject, permission, active_roles))?)
    }

    pub(super) async fn role_permissions(&self, role_id: i64) -> BTreeSet<String> {
        let subject = role_subject(role_id);
        self.policy
            .read()
            .await
            .get_filtered_policy(0, vec![subject])
            .into_iter()
            .filter_map(|rule| rule.get(1).cloned())
            .collect()
    }

    pub(super) async fn user_role_ids(&self, user_id: i64) -> BTreeSet<i64> {
        let subject = user_subject(user_id);
        self.policy
            .read()
            .await
            .get_filtered_grouping_policy(0, vec![subject])
            .into_iter()
            .filter_map(|rule| rule.get(1).and_then(|value| parse_role_subject(value)))
            .collect()
    }

    pub(super) async fn role_has_members(&self, role_id: i64) -> bool {
        !self
            .policy
            .read()
            .await
            .get_filtered_grouping_policy(1, vec![role_subject(role_id)])
            .is_empty()
    }

    pub(super) async fn replace_role_permissions(
        &self,
        role_id: i64,
        permissions: BTreeSet<String>,
    ) -> Result<BTreeSet<String>, AuthorizationError> {
        let subject = role_subject(role_id);
        let mut policy = self.policy.write().await;
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
        let mut policy = self.policy.write().await;
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
        self.policy
            .write()
            .await
            .remove_filtered_grouping_policy(0, vec![user_subject(user_id)])
            .await?;
        Ok(())
    }

    pub(super) async fn remove_role(&self, role_id: i64) -> Result<(), AuthorizationError> {
        let subject = role_subject(role_id);
        let mut policy = self.policy.write().await;
        policy
            .remove_filtered_policy(0, vec![subject.clone()])
            .await?;
        policy
            .remove_filtered_grouping_policy(1, vec![subject])
            .await?;
        Ok(())
    }
}

async fn build_enforcer(store: &PolicyStore) -> Result<Enforcer, AuthorizationError> {
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
    let enforcer = Enforcer::new(model, adapter).await?;
    validate_loaded_policy(&enforcer, store).await?;
    Ok(enforcer)
}

async fn validate_loaded_policy(
    enforcer: &Enforcer,
    store: &PolicyStore,
) -> Result<(), AuthorizationError> {
    let facts = store.policy_facts().await?;
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
        if !facts.users.contains(&user_id) || !facts.roles.contains_key(&role_id) {
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
