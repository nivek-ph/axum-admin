use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

use sqlx::PgPool;

use super::{
    OperationPermissionCatalogItem, RoleError, RoleMenuAccess, RoleOperationPermissionSelection,
    RoleOperationPermissionsWithCatalog, RolePayload, RoleSummary,
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

    pub async fn set_menu_ids(
        &self,
        actor_user_id: i64,
        id: i64,
        values: Vec<i64>,
    ) -> Result<(), RoleError> {
        let values = normalize(values);
        let mut mutation = self.authorization.begin_mutation().await?;
        mutation
            .ensure_role_access_change(actor_user_id, id)
            .await?;
        let configured_menu_ids = values.iter().copied().collect::<HashSet<_>>();
        self.catalog.validate_assignment(&configured_menu_ids)?;
        let mut permissions = self.catalog.page_entry_permissions(&configured_menu_ids);
        permissions.extend(
            mutation
                .role_permissions(id)
                .await?
                .into_iter()
                .filter(|permission| self.catalog.is_action_permission(permission)),
        );
        sqlx::query("delete from sys_role_menus where role_id = $1")
            .bind(id)
            .execute(mutation.connection())
            .await?;
        sqlx::query(
            "insert into sys_role_menus (role_id, menu_id) select $1, unnest($2::bigint[])",
        )
        .bind(id)
        .bind(values)
        .execute(mutation.connection())
        .await?;
        mutation.replace_role_permissions(id, permissions).await?;
        Ok(mutation.commit().await?)
    }

    pub async fn operation_permission_catalog(
        &self,
        id: i64,
    ) -> Result<Vec<OperationPermissionCatalogItem>, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        self.action_permission_catalog(id, role.status == "enabled")
            .await
    }

    pub async fn assigned_operation_permissions(
        &self,
        id: i64,
    ) -> Result<RoleOperationPermissionSelection, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        Ok(RoleOperationPermissionSelection {
            permissions: self.action_permissions(id).await?,
            protected: is_protected(&role),
        })
    }

    pub async fn operation_permissions_with_catalog(
        &self,
        id: i64,
    ) -> Result<RoleOperationPermissionsWithCatalog, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let role_enabled = role.status == "enabled";
        let catalog = self.action_permission_catalog(id, role_enabled).await?;
        let permissions = self.action_permissions(id).await?;
        Ok(RoleOperationPermissionsWithCatalog {
            permissions,
            catalog,
            protected: is_protected(&role),
        })
    }

    async fn action_permission_catalog(
        &self,
        id: i64,
        role_enabled: bool,
    ) -> Result<Vec<OperationPermissionCatalogItem>, RoleError> {
        let configured_menu_ids = sqlx::query_scalar(
            "select menu_id from sys_role_menus where role_id = $1 order by menu_id",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
        let visible_pages = self
            .catalog
            .effective_page_access(&configured_menu_ids, role_enabled);
        Ok(self
            .catalog
            .permission_catalog(&visible_pages, role_enabled)
            .into_iter()
            .filter(|row| row.menu_type == "action")
            .map(|row| OperationPermissionCatalogItem {
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

    async fn action_permissions(&self, id: i64) -> Result<Vec<String>, RoleError> {
        Ok(self
            .authorization
            .role_permissions(id)
            .await?
            .into_iter()
            .filter(|permission| self.catalog.is_action_permission(permission))
            .collect())
    }

    pub async fn set_permissions(
        &self,
        actor_user_id: i64,
        id: i64,
        permissions: Vec<String>,
    ) -> Result<(), RoleError> {
        let mut permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        let mut mutation = self.authorization.begin_mutation().await?;
        mutation
            .ensure_role_access_change(actor_user_id, id)
            .await?;
        if !permissions
            .iter()
            .all(|permission| self.catalog.is_action_permission(permission))
        {
            return Err(RoleError::InvalidPermissions);
        }
        let configured_menu_ids = sqlx::query_scalar(
            "select menu_id from sys_role_menus where role_id = $1 order by menu_id",
        )
        .bind(id)
        .fetch_all(mutation.connection())
        .await?
        .into_iter()
        .collect();
        permissions.extend(self.catalog.page_entry_permissions(&configured_menu_ids));
        mutation.replace_role_permissions(id, permissions).await?;
        Ok(mutation.commit().await?)
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
