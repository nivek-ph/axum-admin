use std::sync::Arc;

use audit::{
    AuditAction, AuditContext, AuditEvent, AuditResource, AuditResult, AuditService, AuditValue,
    FieldChange,
};
use sqlx::PgPool;

use super::{RoleAccessView, RoleError, RolePayload, RoleSummary};
use crate::{
    access::AccessCatalog,
    authorization::{Authorization, ReplaceRoleAccess},
    menus::MenuService,
};

#[derive(Clone)]
pub struct RoleService {
    pool: PgPool,
    authorization: Authorization,
    access_catalog: Arc<AccessCatalog>,
    audits: AuditService,
}

impl RoleService {
    pub(crate) fn new(
        pool: PgPool,
        authorization: Authorization,
        access_catalog: Arc<AccessCatalog>,
    ) -> Self {
        Self {
            audits: AuditService::new(pool.clone()),
            pool,
            authorization,
            access_catalog,
        }
    }

    pub async fn list(&self, actor_user_id: i64) -> Result<Vec<RoleSummary>, RoleError> {
        self.authorization
            .require_role_manager(actor_user_id)
            .await?;
        Ok(
            sqlx::query_as("select id, code, name, status, sort from sys_roles order by sort, id")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    pub async fn create(
        &self,
        actor_user_id: i64,
        payload: RolePayload,
        audit_context: AuditContext,
    ) -> Result<RoleSummary, RoleError> {
        self.authorization
            .require_role_manager(actor_user_id)
            .await?;
        let role: RoleSummary = sqlx::query_as(
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
        .await?;
        self.authorization
            .set_role_status(role.id, role.status == "enabled")
            .await;
        self.record_role_audit(
            audit_context,
            AuditAction::CreateRole,
            role.id,
            vec![FieldChange {
                field: "role".to_string(),
                before: AuditValue::Texts(Vec::new()),
                after: AuditValue::Texts(vec![
                    role.code.clone(),
                    role.name.clone(),
                    role.status.clone(),
                ]),
            }],
        )
        .await;
        Ok(role)
    }

    pub async fn update(
        &self,
        actor_user_id: i64,
        id: i64,
        payload: RolePayload,
        audit_context: AuditContext,
    ) -> Result<RoleSummary, RoleError> {
        self.authorization
            .require_mutable_role(actor_user_id, id)
            .await?;
        let before = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        if payload.code != before.code {
            return Err(RoleError::Immutable);
        }
        let role: RoleSummary = sqlx::query_as(
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
        .await?;
        self.authorization
            .set_role_status(role.id, role.status == "enabled")
            .await;
        self.record_role_audit(
            audit_context,
            AuditAction::UpdateRole,
            id,
            vec![FieldChange {
                field: "metadata".to_string(),
                before: AuditValue::Texts(vec![
                    before.name,
                    before.status,
                    before.sort.to_string(),
                ]),
                after: AuditValue::Texts(vec![
                    role.name.clone(),
                    role.status.clone(),
                    role.sort.to_string(),
                ]),
            }],
        )
        .await;
        Ok(role)
    }

    pub async fn delete(
        &self,
        actor_user_id: i64,
        id: i64,
        audit_context: AuditContext,
    ) -> Result<(), RoleError> {
        self.authorization
            .require_mutable_role(actor_user_id, id)
            .await?;
        let before = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let before_permissions = self.authorization.role_permissions(id).await?;
        if self.authorization.role_has_members(id).await {
            return Err(RoleError::HasMembers);
        }
        self.authorization.remove_role(id).await?;
        let deleted = sqlx::query("delete from sys_roles where id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if deleted == 0 {
            return Err(RoleError::NotFound);
        }
        self.authorization.notify_policy_changed();
        self.record_role_audit(
            audit_context,
            AuditAction::DeleteRole,
            id,
            vec![
                FieldChange {
                    field: "metadata".to_string(),
                    before: AuditValue::Texts(vec![
                        before.code,
                        before.name,
                        before.status,
                        before.sort.to_string(),
                    ]),
                    after: AuditValue::Texts(Vec::new()),
                },
                FieldChange {
                    field: "permissions".to_string(),
                    before: AuditValue::Texts(before_permissions),
                    after: AuditValue::Texts(Vec::new()),
                },
            ],
        )
        .await;
        Ok(())
    }

    pub async fn access(&self, actor_user_id: i64, id: i64) -> Result<RoleAccessView, RoleError> {
        self.authorization
            .require_role_manager(actor_user_id)
            .await?;
        let role = find(&self.pool, id).await?.ok_or(RoleError::NotFound)?;
        let tree = MenuService::from_catalog(
            self.pool.clone(),
            self.authorization.clone(),
            Arc::clone(&self.access_catalog),
        )
        .tree()
        .await?;
        Ok(RoleAccessView {
            permissions: self.authorization.role_permissions(id).await?,
            tree,
            protected: role.code == "super_admin",
        })
    }

    pub async fn replace_access(
        &self,
        actor_user_id: i64,
        id: i64,
        permissions: Vec<String>,
        audit_context: AuditContext,
    ) -> Result<(), RoleError> {
        let permissions = self.access_catalog.normalize_role_access(permissions)?;
        self.authorization
            .replace_role_access(ReplaceRoleAccess {
                actor_user_id,
                role_id: id,
                permissions,
                audit_context,
            })
            .await?;
        Ok(())
    }

    async fn record_role_audit(
        &self,
        context: AuditContext,
        action: AuditAction,
        role_id: i64,
        changes: Vec<FieldChange>,
    ) {
        self.audits
            .record_best_effort(AuditEvent {
                req_id: context.req_id,
                actor: context.actor,
                action,
                resource: AuditResource::Role(role_id),
                result: AuditResult::Succeeded,
                reason_code: None,
                source: context.source,
                changes,
            })
            .await;
    }
}

async fn find(pool: &PgPool, id: i64) -> Result<Option<RoleSummary>, sqlx::Error> {
    sqlx::query_as("select id, code, name, status, sort from sys_roles where id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}
