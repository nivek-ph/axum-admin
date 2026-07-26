use std::collections::BTreeSet;

use sqlx::PgPool;

use super::{
    PermissionCatalogItem, RoleError, RoleMenuAccess, RolePayload, RolePermissions, RoleSummary,
};
use crate::{access::AccessService, authorization::Authorization};

#[derive(Clone)]
pub struct RoleService {
    pool: PgPool,
    access: AccessService,
    authorization: Authorization,
}

impl RoleService {
    pub fn new(pool: PgPool, access: AccessService, authorization: Authorization) -> Self {
        Self {
            pool,
            access,
            authorization,
        }
    }

    pub async fn list(&self) -> Result<Vec<RoleSummary>, RoleError> {
        Ok(list(&self.pool).await?)
    }

    pub async fn create(&self, p: RolePayload) -> Result<RoleSummary, RoleError> {
        let role = create(&self.pool, p).await?;
        self.bump_access_version().await?;
        Ok(role)
    }

    pub async fn update(&self, id: i64, p: RolePayload) -> Result<RoleSummary, RoleError> {
        let current = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        if current.is_system {
            return Err(RoleError::Immutable);
        }
        let role = sqlx::query_as(
            r#"
            update sys_roles
            set name = $1,
                status = coalesce($2, status),
                sort = coalesce($3, sort),
                data_scope = coalesce($4, data_scope),
                updated_at = now()
            where id = $5
            returning id, code, name, status, sort, data_scope, is_system
            "#,
        )
        .bind(p.name)
        .bind(p.status)
        .bind(p.sort)
        .bind(p.data_scope)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        self.authorization.refresh().await?;
        self.bump_access_version().await?;
        Ok(role)
    }

    pub async fn delete(&self, id: i64) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let mut transaction = self.pool.begin().await?;
        self.authorization
            .remove_role_in(&mut transaction, id)
            .await?;
        sqlx::query("delete from sys_roles where id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        self.authorization.refresh().await?;
        self.bump_access_version().await
    }

    pub async fn menu_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        ids(&self.pool, id, "sys_role_menus", "menu_id").await
    }

    pub async fn menu_access(&self, id: i64) -> Result<RoleMenuAccess, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let menu_ids = if role.is_system {
            Vec::new()
        } else {
            ids(&self.pool, id, "sys_role_menus", "menu_id").await?
        };
        let configured = menu_ids.iter().copied().collect::<BTreeSet<_>>();
        let effective_menu_ids = self
            .access
            .effective_role_menu_ids(&configured, role.status == "enabled", role.is_system)
            .into_iter()
            .collect();
        Ok(RoleMenuAccess {
            menu_ids,
            effective_menu_ids,
            system_managed: role.is_system,
        })
    }

    pub async fn set_menu_ids(&self, id: i64, values: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let values = normalize(values);
        self.access
            .validate_menu_assignment(&values.iter().copied().collect())?;
        replace(&self.pool, id, "sys_role_menus", "menu_id", values).await?;
        self.bump_access_version().await
    }

    pub async fn permissions(&self, id: i64) -> Result<RolePermissions, RoleError> {
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let permissions = if role.is_system {
            self.access.enabled_permissions().into_iter().collect()
        } else {
            let permissions = self.authorization.role_permissions(id).await?;
            let permission_set = permissions.iter().cloned().collect();
            self.access
                .validate_permission_assignment(&permission_set)
                .map_err(|_| RoleError::AuthorizationConfig)?;
            permissions
        };
        let visible_pages = self
            .menu_access(id)
            .await?
            .effective_menu_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        let catalog = self
            .access
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
            .collect();
        Ok(RolePermissions {
            permissions,
            catalog,
            system_managed: role.is_system,
        })
    }

    pub async fn set_permissions(
        &self,
        id: i64,
        permissions: Vec<String>,
    ) -> Result<(), RoleError> {
        let permissions = permissions.into_iter().collect::<BTreeSet<_>>();
        ensure_mutable(&self.pool, id).await?;
        self.access
            .validate_permission_assignment(&permissions)
            .map_err(|_| RoleError::InvalidPermissionAssignment)?;
        self.authorization
            .replace_role_permissions(id, permissions)
            .await?;
        self.bump_access_version().await
    }

    pub async fn dept_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        ids(&self.pool, id, "sys_role_depts", "dept_id").await
    }

    pub async fn set_dept_ids(&self, id: i64, v: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        replace(&self.pool, id, "sys_role_depts", "dept_id", normalize(v)).await?;
        self.bump_access_version().await
    }

    pub async fn user_ids(&self, id: i64) -> Result<Vec<i64>, RoleError> {
        find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        Ok(self.authorization.role_user_ids(id).await?)
    }

    pub async fn set_user_ids(&self, id: i64, v: Vec<i64>) -> Result<(), RoleError> {
        ensure_mutable(&self.pool, id).await?;
        let user_ids = normalize(v);
        let existing =
            sqlx::query_scalar::<_, i64>("select count(*) from sys_users where id = any($1)")
                .bind(&user_ids)
                .fetch_one(&self.pool)
                .await?;
        if existing != user_ids.len() as i64 {
            return Err(RoleError::InvalidUserAssignment);
        }
        self.authorization
            .replace_role_users(id, user_ids.into_iter().collect())
            .await?;
        self.bump_access_version().await
    }

    async fn bump_access_version(&self) -> Result<(), RoleError> {
        self.access.bump_version().await?;
        Ok(())
    }
}

pub(crate) async fn list(pool: &PgPool) -> Result<Vec<RoleSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,code,name,status,sort,data_scope,is_system FROM sys_roles ORDER BY sort,id",
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn find(pool: &PgPool, id: i64) -> Result<Option<RoleSummary>, sqlx::Error> {
    sqlx::query_as(
        "SELECT id,code,name,status,sort,data_scope,is_system FROM sys_roles WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

async fn create(pool: &PgPool, p: RolePayload) -> Result<RoleSummary, RoleError> {
    Ok(sqlx::query_as("INSERT INTO sys_roles(code,name,status,sort,data_scope) VALUES($1,$2,$3,$4,$5) RETURNING id,code,name,status,sort,data_scope,is_system").bind(p.code).bind(p.name).bind(p.status.unwrap_or_else(||"enabled".into())).bind(p.sort.unwrap_or(0)).bind(p.data_scope.unwrap_or_else(||"self".into())).fetch_one(pool).await?)
}

async fn ensure_mutable(pool: &PgPool, id: i64) -> Result<(), RoleError> {
    let r = find(pool, id).await?.ok_or(RoleError::NotFound)?;
    if r.is_system {
        Err(RoleError::Immutable)
    } else {
        Ok(())
    }
}

async fn ids(
    pool: &PgPool,
    role_id: i64,
    table: &str,
    column: &str,
) -> Result<Vec<i64>, RoleError> {
    if find(pool, role_id).await?.is_none() {
        return Err(RoleError::NotFound);
    }
    let sql = format!("SELECT {column} FROM {table} WHERE role_id=$1 ORDER BY {column}");
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
        .bind(role_id)
        .fetch_all(pool)
        .await?)
}

async fn replace(
    pool: &PgPool,
    role_id: i64,
    table: &str,
    column: &str,
    values: Vec<i64>,
) -> Result<(), RoleError> {
    let mut tx = pool.begin().await?;
    let del = format!("DELETE FROM {table} WHERE role_id=$1");
    sqlx::query(sqlx::AssertSqlSafe(del))
        .bind(role_id)
        .execute(&mut *tx)
        .await?;
    if !values.is_empty() {
        let insert = if column == "user_id" {
            format!(
                "INSERT INTO {table}(user_id,role_id) SELECT unnest($2::bigint[]),$1 ON CONFLICT DO NOTHING"
            )
        } else {
            format!(
                "INSERT INTO {table}(role_id,{column}) SELECT $1,unnest($2::bigint[]) ON CONFLICT DO NOTHING"
            )
        };
        sqlx::query(sqlx::AssertSqlSafe(insert))
            .bind(role_id)
            .bind(&values)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn normalize(v: Vec<i64>) -> Vec<i64> {
    v.into_iter()
        .filter(|v| *v > 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{AccessCatalog, AccessNode};

    #[test]
    fn normalizes_ids() {
        assert_eq!(normalize(vec![3, 1, 3, 0]), vec![1, 3]);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_assignments_replace_normalize_and_clear(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'batch-role', 'Batch Role', 'enabled', 0, 'self', false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_depts (id, name, code, sort, status)
            values
                (2, 'Batch Department A', 'batch-dept-a', 0, 'enabled'),
                (3, 'Batch Department B', 'batch-dept-b', 0, 'enabled')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            ) values
                (100, 'batch-user-a-uuid', 'batch-user-a', 'hash', 'Batch User A', '', 'dashboard', true, 1, false),
                (101, 'batch-user-b-uuid', 'batch-user-b', 'hash', 'Batch User B', '', 'dashboard', true, 1, false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("insert into sys_role_menus (role_id, menu_id) values (2, 1)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into sys_role_depts (role_id, dept_id) values (2, 2)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:100', 'role:2', '', '', '', '')",
        )
            .execute(&pool)
            .await
            .unwrap();

        let catalog = AccessCatalog::from_parts(
            vec![
                AccessNode {
                    id: 10,
                    parent_id: None,
                    title: "Organization".to_string(),
                    menu_type: "directory".to_string(),
                    status: "enabled".to_string(),
                    permission: None,
                },
                AccessNode {
                    id: 11,
                    parent_id: Some(10),
                    title: "Users".to_string(),
                    menu_type: "page".to_string(),
                    status: "enabled".to_string(),
                    permission: Some("system:user:list".to_string()),
                },
            ],
            Vec::new(),
        )
        .unwrap();
        let access = AccessService::with_catalog(pool.clone(), catalog);
        let authorization = Authorization::new(pool.clone());
        let service = RoleService::new(pool, access, authorization);
        service.set_menu_ids(2, vec![11, 10, 11]).await.unwrap();
        service.set_dept_ids(2, vec![3, 3, 0]).await.unwrap();
        service.set_user_ids(2, vec![101, 101, 0]).await.unwrap();

        assert_eq!(service.menu_ids(2).await.unwrap(), vec![10, 11]);
        assert_eq!(service.dept_ids(2).await.unwrap(), vec![3]);
        assert_eq!(service.user_ids(2).await.unwrap(), vec![101]);

        service.set_menu_ids(2, Vec::new()).await.unwrap();
        service.set_dept_ids(2, Vec::new()).await.unwrap();
        service.set_user_ids(2, Vec::new()).await.unwrap();
        assert!(service.menu_ids(2).await.unwrap().is_empty());
        assert!(service.dept_ids(2).await.unwrap().is_empty());
        assert!(service.user_ids(2).await.unwrap().is_empty());

        let error = service.set_user_ids(2, vec![999]).await.unwrap_err();
        assert!(matches!(error, RoleError::InvalidUserAssignment));
        assert!(service.user_ids(2).await.unwrap().is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn role_permissions_remain_enforceable_when_page_access_is_removed(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'operator', 'Operator', 'enabled', 10, 'self', false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img, home_route,
                enable, dept_id, is_system
            )
            values (
                100, 'operator-uuid', 'operator', 'hash', 'Operator', '', 'dashboard',
                true, 1, false
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:100', 'role:2', '', '', '', '')",
        )
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("insert into sys_role_menus (role_id, menu_id) values (2, 10), (2, 11)")
            .execute(&pool)
            .await
            .unwrap();
        let access = AccessService::load_without_cache(pool.clone())
            .await
            .unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let roles = RoleService::new(pool.clone(), access.clone(), authorization.clone());

        roles
            .set_permissions(2, vec!["system:user:list".to_string()])
            .await
            .unwrap();
        roles.set_menu_ids(2, Vec::new()).await.unwrap();

        let page_access = roles.menu_access(2).await.unwrap();
        assert!(page_access.menu_ids.is_empty());
        assert!(page_access.effective_menu_ids.is_empty());
        let permission_view = roles.permissions(2).await.unwrap();
        assert_eq!(
            permission_view.permissions,
            vec!["system:user:list".to_string()]
        );
        assert!(
            !permission_view
                .catalog
                .iter()
                .find(|item| item.permission == "system:user:list")
                .unwrap()
                .page_visible
        );
        assert!(
            authorization
                .enforce(100, "system:user:list")
                .await
                .unwrap()
        );
        let snapshot = access.snapshot(100).await.unwrap();
        assert!(snapshot.menu_ids.is_empty());
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn deleting_role_removes_its_persisted_permissions(pool: PgPool) {
        sqlx::query(
            r#"
            insert into sys_roles (id, code, name, status, sort, data_scope, is_system)
            values (2, 'disposable', 'Disposable', 'enabled', 10, 'self', false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id, is_system
            )
            values (100, 'role-delete-user', 'role-delete-user', 'hash', 'Role User', '', true, 1, false)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
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
        let access = AccessService::load_without_cache(pool.clone())
            .await
            .unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let roles = RoleService::new(pool.clone(), access, authorization);

        roles.delete(2).await.unwrap();

        let remaining = sqlx::query_scalar::<_, i64>(
            "select count(*) from casbin_rule where v0 = 'role:2' or v1 = 'role:2'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(remaining, 0);
        let user_exists =
            sqlx::query_scalar::<_, bool>("select exists(select 1 from sys_users where id = 100)")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(user_exists);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn system_role_is_dynamic_and_never_exposes_wildcard(pool: PgPool) {
        let access = AccessService::load_without_cache(pool.clone())
            .await
            .unwrap();
        let authorization = Authorization::load(pool.clone()).await.unwrap();
        let roles = RoleService::new(pool, access, authorization);

        let page_access = roles.menu_access(1).await.unwrap();
        assert!(page_access.system_managed);
        assert!(page_access.menu_ids.is_empty());
        assert!(!page_access.effective_menu_ids.is_empty());

        let permissions = roles.permissions(1).await.unwrap();
        assert!(permissions.system_managed);
        assert!(!permissions.permissions.is_empty());
        assert!(!permissions.permissions.iter().any(|value| value == "*"));
        assert!(matches!(
            roles
                .set_permissions(1, vec!["system:user:list".to_string()])
                .await,
            Err(RoleError::Immutable)
        ));
    }
}
