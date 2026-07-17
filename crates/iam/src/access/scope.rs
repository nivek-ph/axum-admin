#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    strum::EnumString,
    strum::Display,
)]
#[strum(serialize_all = "snake_case")]
pub enum DataScope {
    All,
    Dept,
    DeptAndChildren,
    CustomDepts,
    #[strum(serialize = "self")]
    SelfOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResolvedDataScope {
    All,
    DeptIds(Vec<i64>),
    Owner(i64),
}

pub fn merge_scopes(scopes: &[DataScope]) -> DataScope {
    if scopes.contains(&DataScope::All) {
        return DataScope::All;
    }
    if scopes.contains(&DataScope::DeptAndChildren) {
        return DataScope::DeptAndChildren;
    }
    if scopes.contains(&DataScope::CustomDepts) {
        return DataScope::CustomDepts;
    }
    if scopes.contains(&DataScope::Dept) {
        return DataScope::Dept;
    }
    DataScope::SelfOnly
}

pub(crate) async fn resolve_user_data_scope(
    pool: &sqlx::PgPool,
    user_id: i64,
    _resource: &str,
) -> Result<ResolvedDataScope, sqlx::Error> {
    let user_dept_id: Option<i64> =
        sqlx::query_scalar("select dept_id from sys_users where id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    let role_scopes: Vec<String> = sqlx::query_scalar(
        r#"
        select r.data_scope
        from sys_user_roles ur
        join sys_roles r on r.id = ur.role_id
        where ur.user_id = $1
          and r.status = 'enabled'
        order by r.id
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    let scopes = role_scopes
        .iter()
        .map(|scope| scope.parse().unwrap_or(DataScope::SelfOnly))
        .collect::<Vec<_>>();

    match merge_scopes(&scopes) {
        DataScope::All => Ok(ResolvedDataScope::All),
        DataScope::SelfOnly => Ok(ResolvedDataScope::Owner(user_id)),
        DataScope::Dept => Ok(ResolvedDataScope::DeptIds(
            user_dept_id.into_iter().collect(),
        )),
        DataScope::DeptAndChildren => {
            let Some(dept_id) = user_dept_id else {
                return Ok(ResolvedDataScope::DeptIds(Vec::new()));
            };
            Ok(ResolvedDataScope::DeptIds(
                dept_descendant_ids(pool, dept_id).await?,
            ))
        }
        DataScope::CustomDepts => {
            let dept_ids = sqlx::query_scalar(
                r#"
                select distinct rd.dept_id
                from sys_user_roles ur
                join sys_role_depts rd on rd.role_id = ur.role_id
                join sys_roles r on r.id = ur.role_id
                where ur.user_id = $1
                  and r.status = 'enabled'
                order by rd.dept_id
                "#,
            )
            .bind(user_id)
            .fetch_all(pool)
            .await?;
            Ok(ResolvedDataScope::DeptIds(dept_ids))
        }
    }
}

async fn dept_descendant_ids(pool: &sqlx::PgPool, dept_id: i64) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        with recursive descendants as (
            select id from sys_depts where id = $1
            union all
            select d.id
            from sys_depts d
            join descendants parent on d.parent_id = parent.id
        )
        select id from descendants order by id
        "#,
    )
    .bind(dept_id)
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_scopes_uses_broadest_scope() {
        let scopes = vec![DataScope::SelfOnly, DataScope::Dept, DataScope::All];
        assert_eq!(merge_scopes(&scopes), DataScope::All);
    }

    #[test]
    fn merge_scopes_prefers_dept_children_over_custom_and_dept() {
        let scopes = vec![
            DataScope::Dept,
            DataScope::CustomDepts,
            DataScope::DeptAndChildren,
        ];
        assert_eq!(merge_scopes(&scopes), DataScope::DeptAndChildren);
    }

    #[test]
    fn merge_scopes_defaults_to_self_for_empty_input() {
        assert_eq!(merge_scopes(&[]), DataScope::SelfOnly);
    }

    #[test]
    fn data_scope_parse_defaults_unknown_to_self() {
        assert_eq!("all".parse(), Ok(DataScope::All));
        assert_eq!("dept_and_children".parse(), Ok(DataScope::DeptAndChildren));
        assert_eq!("custom_depts".parse(), Ok(DataScope::CustomDepts));
        assert_eq!("self".parse(), Ok(DataScope::SelfOnly));
        assert_eq!(
            "unknown".parse().unwrap_or(DataScope::SelfOnly),
            DataScope::SelfOnly
        );
    }
}
