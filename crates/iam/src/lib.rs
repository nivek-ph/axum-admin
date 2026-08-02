use std::{sync::Arc, time::Duration};

use access::{AccessCatalog, AccessService};
use authorization::Authorization;
use menus::MenuService;
use sqlx::PgPool;

pub mod access;
pub mod accounts;
mod authorization;
pub mod departments;
pub mod menus;
pub mod roles;

pub use authorization::AuthorizationError;

#[derive(Clone)]
pub struct Iam {
    pub access: AccessService,
    pub accounts: accounts::Accounts,
    pub menus: MenuService,
    pub roles: roles::RoleService,
    authorization: Authorization,
}

impl Iam {
    pub async fn load(pool: PgPool) -> Result<Self, IamInitError> {
        let authorization = Authorization::load(pool.clone()).await?;
        let access_catalog = Arc::new(AccessCatalog::load(&pool).await?);
        let access = AccessService::from_catalog(
            pool.clone(),
            authorization.clone(),
            access_catalog.clone(),
        );
        let accounts =
            accounts::Accounts::new(pool.clone(), authorization.clone(), access_catalog.clone());
        let menus =
            MenuService::from_catalog(pool.clone(), authorization.clone(), access_catalog.clone());
        let roles = roles::RoleService::new(pool, authorization.clone(), access_catalog);
        Ok(Self {
            access,
            accounts,
            menus,
            roles,
            authorization,
        })
    }

    /// An error means only that the Redis watcher is unavailable; the periodic
    /// reload fallback is running either way.
    pub fn start_policy_sync(
        &self,
        redis_url: &str,
        reload_interval: Duration,
    ) -> Result<(), AuthorizationError> {
        self.authorization
            .start_policy_sync(redis_url, reload_interval)
    }
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IamInitError(#[from] IamInitErrorKind);

#[derive(Debug, thiserror::Error)]
enum IamInitErrorKind {
    #[error("IAM database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("IAM access catalog is invalid")]
    Catalog(#[from] access::CatalogError),
    #[error("IAM Authorization could not initialize")]
    Authorization(#[from] AuthorizationError),
}

impl From<sqlx::Error> for IamInitError {
    fn from(error: sqlx::Error) -> Self {
        IamInitErrorKind::Database(error).into()
    }
}

impl From<access::CatalogError> for IamInitError {
    fn from(error: access::CatalogError) -> Self {
        IamInitErrorKind::Catalog(error).into()
    }
}

impl From<AuthorizationError> for IamInitError {
    fn from(error: AuthorizationError) -> Self {
        IamInitErrorKind::Authorization(error).into()
    }
}
