use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

use sqlx::PgPool;

use super::{
    PermissionCatalogItem, RoleError, RoleMenuAccess, RolePayload, RolePermissionView, RoleSummary,
};
use crate::{access::AccessCatalog, authorization::Authorization};

#[derive(Clone)]
pub struct RoleService {
    pool: PgPool,
    catalog: Arc<AccessCatalog>,
    authorization: Authorization,
}

impl RoleService {
    pub(crate) fn new(
        pool: PgPool,
        catalog: Arc<AccessCatalog>,
        authorization: Authorization,
    ) -> Self {
        Self {
            pool,
            catalog,
            authorization,
        }
    }

    pub async fn list(&self) -> Result<Vec<RoleSummary>, RoleError> {
        Ok(
            sqlx::query_as("select id, code, name, status, sort from sys_roles order by sort, id")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn create(&self, payload: RolePayload) -> Result<RoleSummary, RoleError> {
        Ok(sqlx::query_as(
            r#"
            insert into sys_roles (code, name, status, sort)
            values ($1, $2, $3, $4)
            returning id, code, name, status, sort
            "#,
        )
        .bind(payload.code)
        .bind(payload.name)
        .bind(payload.status.unwrap_or_else(|| "enabled".to_string()))
        .bind(payload.sort.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn update(&self, id: i64, payload: RolePayload) -> Result<RoleSummary, RoleError> {
        let current = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        if is_protected(&current)
            && (payload.code != current.code
                || payload
                    .status
                    .as_deref()
                    .is_some_and(|status| status != "enabled"))
        {
            return Err(RoleError::Immutable);
        }
        Ok(sqlx::query_as(
            r#"
            update sys_roles
            set name = $1,
                status = coalesce($2, status),
                sort = coalesce($3, sort),
                updated_at = now()
            where id = $4
            returning id, code, name, status, sort
            "#,
        )
        .bind(payload.name)
        .bind(payload.status)
        .bind(payload.sort)
        .bind(id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn delete(&self, id: i64) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let mut mutation = self.authorization.begin_mutation().await?;
        mutation.remove_role(id).await?;
        sqlx::query("delete from sys_roles where id = $1")
            .bind(id)
            .execute(mutation.connection())
            .await?;
        Ok(mutation.commit().await?)
    }

    pub async fn menu_access(&self, id: i64) -> Result<RoleMenuAccess, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let menu_ids = sqlx::query_scalar(
            "select menu_id from sys_role_menus where role_id = $1 order by menu_id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        let configured = menu_ids.iter().copied().collect::<HashSet<_>>();
        let effective_menu_ids = self
            .catalog
            .effective_page_access(&configured, role.status == "enabled")
            .into_iter()
            .collect();
        Ok(RoleMenuAccess {
            menu_ids,
            effective_menu_ids,
            protected: is_protected(&role),
        })
    }

    pub async fn set_menu_ids(&self, id: i64, values: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let values = normalize(values);
        self.catalog
            .validate_assignment(&values.iter().copied().collect())?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query("delete from sys_role_menus where role_id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "insert into sys_role_menus (role_id, menu_id) select $1, unnest($2::bigint[])",
        )
        .bind(id)
        .bind(values)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(())
    }

    pub async fn permission_catalog(
        &self,
        id: i64,
    ) -> Result<Vec<PermissionCatalogItem>, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let visible_pages = self
            .menu_access(id)
            .await?
            .effective_menu_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        Ok(self
            .catalog
            .permission_catalog(&visible_pages, role.status == "enabled")
            .into_iter()
            .map(|row| PermissionCatalogItem {
                permission: row.permission,
                title: row.title,
                menu_type: row.menu_type,
                status: row.status,
                effectively_enabled: row.effectively_enabled,
                owning_page_id: row.owning_page_id,
                owning_page_title: row.owning_page_title,
                page_visible: row.page_visible,
            })
            .collect())
    }

    pub async fn permissions(&self, id: i64) -> Result<RolePermissionView, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        Ok(RolePermissionView {
            permissions: self
                .authorization
                .role_permissions(id)
                .await
                .map_err(map_policy_error)?,
            protected: is_protected(&role),
        })
    }

    pub async fn set_permissions(
        &self,
        id: i64,
        permissions: Vec<String>,
    ) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        self.authorization
            .replace_role_permissions(id, permissions)
            .await
            .map_err(map_policy_error)?;
        Ok(())
    }
}

fn map_policy_error(error: crate::authorization::PolicyAdministrationError) -> RoleError {
    use crate::authorization::PolicyAdministrationError;
    match error {
        PolicyAdministrationError::RoleNotFound => RoleError::NotFound,
        PolicyAdministrationError::RoleImmutable => RoleError::Immutable,
        PolicyAdministrationError::InvalidPermissionAssignment => RoleError::InvalidPermissions,
        PolicyAdministrationError::Database(source) => RoleError::Database(source),
        PolicyAdministrationError::Authorization(source) => RoleError::Authorization(source),
        PolicyAdministrationError::UserNotFound
        | PolicyAdministrationError::AccessDenied
        | PolicyAdministrationError::LastSuperAdmin
        | PolicyAdministrationError::InvalidRoleAssignment
        | PolicyAdministrationError::Audit(_) => RoleError::Immutable,
    }
}

async fn find(pool: &PgPool, id: i64) -> Result<Option<RoleSummary>, sqlx::Error> {
    sqlx::query_as("select id, code, name, status, sort from sys_roles where id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

async fn ensure_mutable(pool: &PgPool, id: i64) -> Result<(), RoleError> {
    let role = find(pool, id).await?.ok_or(RoleError::NotFound)?;
    if is_protected(&role) {
        Err(RoleError::Immutable)
    } else {
        Ok(())
    }
}

fn is_protected(role: &RoleSummary) -> bool {
    role.code == "super_admin"
}

fn normalize(values: Vec<i64>) -> Vec<i64> {
    values
        .into_iter()
        .filter(|value| *value > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_set_ids_are_sorted_deduplicated_and_positive() {
        assert_eq!(normalize(vec![3, 1, 3, 0]), vec![1, 3]);
    }
}
