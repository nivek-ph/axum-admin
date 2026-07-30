use std::sync::Arc;

use access::{AccessCatalog, AccessService};
use authorization::Authorization;
use menus::MenuService;
use sqlx::PgPool;

pub mod access;
pub mod accounts;
pub mod authorization;
pub mod departments;
pub mod menus;
pub mod roles;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct IamInitError(#[from] IamInitErrorKind);

#[derive(Debug, thiserror::Error)]
enum IamInitErrorKind {
    #[error("IAM database operation failed")]
    Database(#[from] sqlx::Error),
    #[error("IAM access catalog is invalid")]
    Catalog(#[from] access::CatalogError),
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

pub async fn load_access_and_menus(
    pool: PgPool,
    authorization: Authorization,
) -> Result<(AccessService, MenuService), IamInitError> {
    let catalog = Arc::new(AccessCatalog::load(&pool).await?);
    Ok((
        AccessService::from_catalog(pool.clone(), authorization.clone(), catalog.clone()),
        MenuService::from_catalog(pool, authorization, catalog),
    ))
}

#[doc(hidden)]
pub fn access_and_menus_without_catalog_for_tests(
    pool: PgPool,
    authorization: Authorization,
) -> (AccessService, MenuService) {
    let catalog = Arc::new(AccessCatalog::new(Vec::new()).expect("empty catalog is valid"));
    (
        AccessService::from_catalog(pool.clone(), authorization.clone(), catalog.clone()),
        MenuService::from_catalog(pool, authorization, catalog),
    )
}
