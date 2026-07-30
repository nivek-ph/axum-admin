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
        let catalog = Arc::new(AccessCatalog::load(&pool).await?);
        let access =
            AccessService::from_catalog(pool.clone(), authorization.clone(), catalog.clone());
        let accounts =
            accounts::Accounts::new(pool.clone(), authorization.clone(), catalog.clone());
        let menus = MenuService::from_catalog(pool.clone(), authorization.clone(), catalog.clone());
        let roles = roles::RoleService::new(pool, catalog, authorization.clone());
        Ok(Self {
            access,
            accounts,
            menus,
            roles,
            authorization,
        })
    }

    pub fn start_redis_watcher(&self, redis_url: &str) -> Result<(), AuthorizationError> {
        self.authorization.start_redis_watcher(redis_url)
    }

    pub fn start_periodic_reload(&self, interval: Duration) {
        self.authorization.start_periodic_reload(interval);
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
