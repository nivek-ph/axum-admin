use std::collections::{HashMap, HashSet};

use super::{CreateDeptPayload, Dept, DeptError, DeptNode, UpdateDeptPayload};
use crate::access::AccessService;

#[derive(Clone)]
pub struct DepartmentService {
    pool: sqlx::PgPool,
    access: AccessService,
}

impl DepartmentService {
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self {
            access: AccessService::new(pool.clone()),
            pool,
        }
    }

    pub fn with_access(pool: sqlx::PgPool, access: AccessService) -> Self {
        Self { pool, access }
    }

    pub async fn tree(&self) -> Result<Vec<DeptNode>, DeptError> {
        Ok(tree(&self.pool).await?)
    }

    pub async fn find(&self, id: i64) -> Result<Option<Dept>, DeptError> {
        Ok(find(&self.pool, id).await?)
    }

    pub async fn create(&self, payload: CreateDeptPayload) -> Result<(), DeptError> {
        create(&self.pool, payload).await?;
        self.bump_access_version().await
    }

    pub async fn update(&self, id: i64, payload: UpdateDeptPayload) -> Result<(), DeptError> {
        update(&self.pool, id, payload).await?;
        self.bump_access_version().await
    }

    pub async fn delete(&self, id: i64) -> Result<(), DeptError> {
        delete(&self.pool, id).await?;
        self.bump_access_version().await
    }

    async fn bump_access_version(&self) -> Result<(), DeptError> {
        self.access.bump_version().await?;
        Ok(())
    }
}

pub fn build_dept_tree(rows: Vec<Dept>) -> Vec<DeptNode> {
    let ids = rows.iter().map(|dept| dept.id).collect::<HashSet<_>>();
    let mut children_by_parent = HashMap::<i64, Vec<DeptNode>>::new();
    let mut roots = Vec::new();

    for row in rows {
        let node = DeptNode {
            id: row.id,
            parent_id: row.parent_id,
            name: row.name,
            code: row.code,
            sort: row.sort,
            status: row.status,
            children: Vec::new(),
        };

        match node.parent_id {
            Some(parent_id) if ids.contains(&parent_id) => {
                children_by_parent.entry(parent_id).or_default().push(node);
            }
            Some(_) | None => roots.push(node),
        }
    }

    sort_nodes(&mut roots);
    attach_children(&mut roots, &mut children_by_parent);
    roots
}

pub(crate) async fn list(pool: &sqlx::PgPool) -> Result<Vec<Dept>, sqlx::Error> {
    sqlx::query_as::<_, Dept>(
        r#"
        select id, parent_id, name, code, sort, status
        from sys_depts
        order by parent_id nulls first, sort, id
        "#,
    )
    .fetch_all(pool)
    .await
}

pub(crate) async fn tree(pool: &sqlx::PgPool) -> Result<Vec<DeptNode>, sqlx::Error> {
    let rows = list(pool).await?;
    Ok(build_dept_tree(rows))
}

pub(crate) async fn find(pool: &sqlx::PgPool, id: i64) -> Result<Option<Dept>, sqlx::Error> {
    sqlx::query_as::<_, Dept>(
        r#"
        select id, parent_id, name, code, sort, status
        from sys_depts
        where id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn create(
    pool: &sqlx::PgPool,
    payload: CreateDeptPayload,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        insert into sys_depts (parent_id, name, code, sort, status)
        values ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(payload.parent_id)
    .bind(payload.name)
    .bind(payload.code)
    .bind(payload.sort.unwrap_or(0))
    .bind(payload.status.unwrap_or_else(|| "enabled".to_string()))
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn update(
    pool: &sqlx::PgPool,
    id: i64,
    payload: UpdateDeptPayload,
) -> Result<(), DeptError> {
    if parent_is_self(id, payload.parent_id) {
        return Err(DeptError::InvalidParent);
    }

    if let Some(parent_id) = payload.parent_id
        && sqlx::query_scalar(
            r#"
            with recursive ancestors as (
                select id, parent_id from sys_depts where id = $1
                union all
                select d.id, d.parent_id
                from sys_depts d
                join ancestors a on d.id = a.parent_id
            )
            select exists(select 1 from ancestors where id = $2)
            "#,
        )
        .bind(parent_id)
        .bind(id)
        .fetch_one(pool)
        .await?
    {
        return Err(DeptError::InvalidParent);
    }

    sqlx::query(
        r#"
        update sys_depts
        set parent_id = $1, name = $2, code = $3, sort = coalesce($4, sort), status = coalesce($5, status)
        where id = $6
        "#,
    )
    .bind(payload.parent_id)
    .bind(payload.name)
    .bind(payload.code)
    .bind(payload.sort)
    .bind(payload.status)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn delete(pool: &sqlx::PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("delete from sys_depts where id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

fn attach_children(nodes: &mut [DeptNode], children_by_parent: &mut HashMap<i64, Vec<DeptNode>>) {
    for node in nodes {
        if let Some(mut children) = children_by_parent.remove(&node.id) {
            sort_nodes(&mut children);
            attach_children(&mut children, children_by_parent);
            node.children = children;
        }
    }
}

fn sort_nodes(nodes: &mut [DeptNode]) {
    nodes.sort_by_key(|node| (node.sort, node.id));
}

fn parent_is_self(id: i64, parent_id: Option<i64>) -> bool {
    parent_id == Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dept(id: i64, parent_id: Option<i64>, sort: i32) -> Dept {
        Dept {
            id,
            parent_id,
            name: format!("Dept {id}"),
            code: format!("dept_{id}"),
            sort,
            status: "enabled".to_string(),
        }
    }

    #[test]
    fn build_dept_tree_returns_root_with_child_when_parent_exists() {
        let tree = build_dept_tree(vec![dept(2, Some(1), 0), dept(1, None, 0)]);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 1);
        assert_eq!(tree[0].children.len(), 1);
        assert_eq!(tree[0].children[0].id, 2);
    }

    #[test]
    fn build_dept_tree_preserves_orphan_as_root() {
        let tree = build_dept_tree(vec![dept(2, Some(99), 0)]);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].id, 2);
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn parent_is_self_detects_same_id_parent() {
        assert!(parent_is_self(7, Some(7)));
        assert!(!parent_is_self(7, Some(8)));
        assert!(!parent_is_self(7, None));
    }
}
