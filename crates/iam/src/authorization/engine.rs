use std::{
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use casbin::{CoreApi, DefaultModel, Enforcer, EventData, Watcher};
use redis_watcher::{RedisWatcher, WatcherOptions};
use sqlx_adapter::SqlxAdapter;
use tokio::sync::{Mutex, MutexGuard, RwLock};

use super::{AuthorizationError, store::PolicyStore};

const REDIS_CHANNEL: &str = "/ava/casbin";

/// An enforcement engine for the authorization policy.
pub(super) struct EnforcementEngine {
    store: Arc<PolicyStore>,
    policy: RwLock<Enforcer>,
    policy_change_lock: Mutex<()>,
    watcher: StdMutex<Option<RedisWatcher>>,
}

impl EnforcementEngine {
    pub(super) async fn load(store: Arc<PolicyStore>) -> Result<Self, AuthorizationError> {
        let policy = build_enforcer(&store).await?;
        Ok(Self {
            store,
            policy: RwLock::new(policy),
            policy_change_lock: Mutex::new(()),
            watcher: StdMutex::new(None),
        })
    }

    pub(super) fn start_redis_watcher(
        self: &Arc<Self>,
        redis_url: &str,
    ) -> Result<(), AuthorizationError> {
        let options = WatcherOptions::default()
            .with_channel(REDIS_CHANNEL.to_string())
            .with_ignore_self(true);
        let mut watcher = RedisWatcher::new(redis_url, options)?;
        // The watcher is owned by the engine, so its callback must not hold a
        // strong engine reference and create an ownership cycle.
        let weak_engine = Arc::downgrade(self);
        watcher.set_update_callback(Box::new(move |_| {
            let Some(engine) = weak_engine.upgrade() else {
                return;
            };
            tokio::spawn(async move {
                if let Err(error) = engine.reload().await {
                    tracing::error!("Failed to reload enforcement engine from watcher: {error}");
                }
            });
        }));
        let mut installed = self
            .watcher
            .lock()
            .map_err(|_| AuthorizationError::WatcherInstallation)?;
        *installed = Some(watcher);
        Ok(())
    }

    pub(super) fn start_periodic_reload(self: &Arc<Self>, interval: Duration) {
        // The periodic task must not keep the engine alive after the owning
        // application state has been dropped.
        let weak_engine = Arc::downgrade(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;
                let Some(engine) = weak_engine.upgrade() else {
                    return;
                };
                if let Err(error) = engine.reload().await {
                    tracing::error!("Failed to reload enforcement engine periodically: {error}");
                }
            }
        });
    }

    async fn reload(&self) -> Result<(), AuthorizationError> {
        let _guard = self.policy_change_lock.lock().await;
        self.reload_locked().await
    }

    pub(super) async fn reload_locked(&self) -> Result<(), AuthorizationError> {
        let next = build_enforcer(&self.store).await?;
        *self.policy.write().await = next;
        Ok(())
    }

    pub(super) async fn enforce(
        &self,
        subject: String,
        permission: &str,
        active_roles: Vec<String>,
    ) -> Result<bool, AuthorizationError> {
        let policy = self.policy.read().await;
        Ok(policy.enforce((subject, permission, active_roles))?)
    }

    pub(super) async fn lock_policy_change(&self) -> MutexGuard<'_, ()> {
        self.policy_change_lock.lock().await
    }

    pub(super) fn publish_change(&self) {
        let Ok(mut watcher_guard) = self.watcher.lock() else {
            return;
        };
        if let Some(watcher) = watcher_guard.as_mut() {
            watcher.update(EventData::SavePolicy(Vec::new()));
        }
    }
}

/// Builds a fully loaded Casbin enforcement engine from the persisted policy.
///
/// Persisted invariants are validated before constructing the Casbin model and
/// SQLx adapter. Callers can therefore keep serving the current enforcer while
/// this function runs and replace it only after the new one is completely ready.
async fn build_enforcer(store: &PolicyStore) -> Result<Enforcer, AuthorizationError> {
    store.validate_policy_invariants().await?;
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
        m = (r.sub == p.sub || (g(r.sub, p.sub) && r.active_roles.contains(p.sub))) && r.perm == p.perm
        "#,
    )
    .await
    .map_err(|error| AuthorizationError::Configuration(error.to_string()))?;
    let adapter = SqlxAdapter::new_with_pool(store.pool().clone()).await?;
    Ok(Enforcer::new(model, adapter).await?)
}
