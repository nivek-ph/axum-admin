use std::{
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use casbin::{CoreApi, DefaultModel, Enforcer, EventData, Watcher};
use redis_watcher::{RedisWatcher, WatcherOptions};
use sqlx_adapter::SqlxAdapter;
use tokio::sync::{Mutex, MutexGuard, RwLock};

use super::{AuthorizationError, store::PolicyStore};

pub(crate) const REDIS_CHANNEL: &str = "/ava/casbin";

#[derive(Clone)]
pub(super) struct EnforcementEngine {
    state: Arc<RwLock<Option<Enforcer>>>,
    reload_lock: Arc<Mutex<()>>,
    watcher: Arc<StdMutex<Option<RedisWatcher>>>,
}

impl EnforcementEngine {
    pub(super) fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(None)),
            reload_lock: Arc::new(Mutex::new(())),
            watcher: Arc::new(StdMutex::new(None)),
        }
    }

    pub(super) async fn load(store: &PolicyStore) -> Result<Self, AuthorizationError> {
        let engine = Self::new();
        engine.reload(store).await?;
        Ok(engine)
    }

    pub(super) fn start_redis_watcher(
        &self,
        store: PolicyStore,
        redis_url: &str,
    ) -> Result<(), AuthorizationError> {
        let options = WatcherOptions::default()
            .with_channel(REDIS_CHANNEL.to_string())
            .with_ignore_self(true);
        let mut watcher = RedisWatcher::new(redis_url, options)?;
        let engine = self.clone();
        watcher.set_update_callback(Box::new(move |_| {
            let engine = engine.clone();
            let store = store.clone();
            tokio::spawn(async move {
                let _ = engine.reload(&store).await;
            });
        }));
        let mut installed = self
            .watcher
            .lock()
            .map_err(|_| AuthorizationError::WatcherInstallation)?;
        *installed = Some(watcher);
        Ok(())
    }

    pub(super) fn start_periodic_reload(&self, store: PolicyStore, interval: Duration) {
        let engine = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let _ = engine.reload(&store).await;
            }
        });
    }

    pub(super) async fn reload(&self, store: &PolicyStore) -> Result<(), AuthorizationError> {
        let _guard = self.reload_lock.lock().await;
        self.reload_locked(store).await
    }

    pub(super) async fn reload_locked(
        &self,
        store: &PolicyStore,
    ) -> Result<(), AuthorizationError> {
        // Withdraw the current enforcer before rebuilding it. Enforcement that
        // races a reload must wait for the new state instead of continuing with
        // a state that may already be stale.
        *self.state.write().await = None;
        let enforcer = load_enforcer(store).await?;
        *self.state.write().await = Some(enforcer);
        Ok(())
    }

    pub(super) async fn enforce(
        &self,
        subject: String,
        permission: &str,
        active_roles: Vec<String>,
    ) -> Result<bool, AuthorizationError> {
        let state = self.state.read().await;
        let enforcer = state.as_ref().ok_or(AuthorizationError::StateUnavailable)?;
        Ok(enforcer.enforce((subject, permission, active_roles))?)
    }

    pub(super) async fn lock_reload(&self) -> MutexGuard<'_, ()> {
        self.reload_lock.lock().await
    }

    pub(super) fn publish_change(&self) {
        let Ok(mut installed) = self.watcher.lock() else {
            return;
        };
        if let Some(watcher) = installed.as_mut() {
            watcher.update(EventData::SavePolicy(Vec::new()));
        }
    }
}

async fn load_enforcer(store: &PolicyStore) -> Result<Enforcer, AuthorizationError> {
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
