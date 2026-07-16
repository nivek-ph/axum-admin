use sqlx::PgPool;

use super::{
    DictionaryDetailInput, DictionaryError, DictionaryInput, DictionaryListQuery,
    DictionaryWithDetails, SysDictionary, SysDictionaryDetail, model::SysDictionaryDetailRow,
};

#[derive(Clone)]
pub struct DictionaryService {
    pool: PgPool,
}

impl DictionaryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list(
        &self,
        query: DictionaryListQuery,
    ) -> Result<Vec<SysDictionary>, DictionaryError> {
        Ok(list(&self.pool, query).await?)
    }

    pub async fn create(&self, payload: DictionaryInput) -> Result<(), DictionaryError> {
        Ok(create(&self.pool, payload).await?)
    }

    pub async fn update(&self, id: i64, payload: DictionaryInput) -> Result<(), DictionaryError> {
        Ok(update(&self.pool, id, payload).await?)
    }

    pub async fn find(
        &self,
        id: Option<i64>,
        kind: Option<String>,
    ) -> Result<Option<DictionaryWithDetails>, DictionaryError> {
        Ok(find_by_query(&self.pool, id, kind).await?)
    }

    pub async fn delete(&self, id: i64) -> Result<(), DictionaryError> {
        Ok(delete(&self.pool, id).await?)
    }

    pub async fn export(&self, id: i64) -> Result<Option<DictionaryWithDetails>, DictionaryError> {
        Ok(export_dictionary(&self.pool, id).await?)
    }

    pub async fn import(&self, payload: DictionaryInput) -> Result<(), DictionaryError> {
        Ok(create(&self.pool, payload).await?)
    }

    pub async fn create_detail(
        &self,
        dictionary_id: i64,
        payload: DictionaryDetailInput,
    ) -> Result<(), DictionaryError> {
        ensure_dictionary_exists(&self.pool, dictionary_id).await?;
        create_detail(&self.pool, dictionary_id, payload).await
    }

    pub async fn update_detail(
        &self,
        dictionary_id: i64,
        detail_id: i64,
        payload: DictionaryDetailInput,
    ) -> Result<(), DictionaryError> {
        update_detail(&self.pool, dictionary_id, detail_id, payload).await
    }

    pub async fn find_detail(
        &self,
        dictionary_id: i64,
        detail_id: i64,
    ) -> Result<SysDictionaryDetail, DictionaryError> {
        find_detail(&self.pool, dictionary_id, detail_id)
            .await?
            .ok_or(DictionaryError::DetailNotFound {
                dictionary_id,
                detail_id,
            })
    }

    pub async fn delete_detail(
        &self,
        dictionary_id: i64,
        detail_id: i64,
    ) -> Result<(), DictionaryError> {
        self.find_detail(dictionary_id, detail_id).await?;
        Ok(delete_detail(&self.pool, dictionary_id, detail_id).await?)
    }

    pub async fn tree_by_dictionary(
        &self,
        id: i64,
    ) -> Result<Vec<SysDictionaryDetail>, DictionaryError> {
        Ok(tree_by_dictionary(&self.pool, id).await?)
    }

    pub async fn tree_by_type(
        &self,
        kind: &str,
    ) -> Result<Vec<SysDictionaryDetail>, DictionaryError> {
        Ok(tree_by_type(&self.pool, kind).await?)
    }

    pub async fn details_by_parent(
        &self,
        dictionary_id: i64,
        parent_id: i64,
    ) -> Result<Vec<SysDictionaryDetail>, DictionaryError> {
        Ok(details_by_parent(&self.pool, dictionary_id, parent_id).await?)
    }

    pub async fn detail_path(
        &self,
        dictionary_id: i64,
        detail_id: i64,
    ) -> Result<Vec<SysDictionaryDetail>, DictionaryError> {
        self.find_detail(dictionary_id, detail_id).await?;
        Ok(detail_path(&self.pool, dictionary_id, detail_id).await?)
    }
}

async fn ensure_dictionary_exists(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
) -> Result<(), DictionaryError> {
    if find(pool, dictionary_id).await?.is_none() {
        return Err(DictionaryError::DictionaryNotFound { dictionary_id });
    }
    Ok(())
}

pub(crate) async fn list(
    pool: &sqlx::PgPool,
    query: DictionaryListQuery,
) -> Result<Vec<SysDictionary>, sqlx::Error> {
    let list = sqlx::query_as::<_, SysDictionary>(
        r#"
        select id, name, type as dict_type, status, "desc", parent_id
        from sys_dictionaries
        where ($1::text is null or name ilike '%' || $1 || '%')
        order by id desc
        "#,
    )
    .bind(query.name.as_deref())
    .fetch_all(pool)
    .await?;

    Ok(list)
}

pub(crate) async fn create(
    pool: &sqlx::PgPool,
    payload: DictionaryInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "insert into sys_dictionaries (name, type, status, \"desc\", parent_id) values ($1, $2, $3, $4, $5)",
    )
    .bind(payload.name)
    .bind(payload.dict_type)
    .bind(payload.status)
    .bind(payload.desc)
    .bind(payload.parent_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn update(
    pool: &sqlx::PgPool,
    id: i64,
    payload: DictionaryInput,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "update sys_dictionaries set name = $1, type = $2, status = $3, \"desc\" = $4, parent_id = $5 where id = $6",
    )
    .bind(payload.name)
    .bind(payload.dict_type)
    .bind(payload.status)
    .bind(payload.desc)
    .bind(payload.parent_id)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn find(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<Option<SysDictionary>, sqlx::Error> {
    sqlx::query_as::<_, SysDictionary>(
        "select id, name, type as dict_type, status, \"desc\", parent_id from sys_dictionaries where id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn find_by_query(
    pool: &sqlx::PgPool,
    id: Option<i64>,
    dict_type: Option<String>,
) -> Result<Option<DictionaryWithDetails>, sqlx::Error> {
    let dictionary = if let Some(id) = id {
        find(pool, id).await?
    } else if let Some(dict_type) = dict_type {
        find_by_type(pool, &dict_type).await?
    } else {
        None
    };

    if let Some(dictionary) = dictionary {
        let details = tree_by_dictionary(pool, dictionary.id).await?;
        return Ok(Some(DictionaryWithDetails {
            dictionary,
            details,
        }));
    }

    Ok(None)
}

pub(crate) async fn delete(pool: &sqlx::PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("delete from sys_dictionary_details where sys_dictionary_id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("delete from sys_dictionaries where id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub(crate) async fn find_by_type(
    pool: &sqlx::PgPool,
    dict_type: &str,
) -> Result<Option<SysDictionary>, sqlx::Error> {
    sqlx::query_as::<_, SysDictionary>(
        "select id, name, type as dict_type, status, \"desc\", parent_id from sys_dictionaries where type = $1",
    )
    .bind(dict_type)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn create_detail(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    payload: DictionaryDetailInput,
) -> Result<(), DictionaryError> {
    let (level, path) = match payload.parent_id {
        Some(parent_id) => detail_level_and_path(pool, Some(parent_id), dictionary_id)
            .await?
            .ok_or(DictionaryError::DetailNotFound {
                dictionary_id,
                detail_id: parent_id,
            })?,
        None => (0, String::new()),
    };
    sqlx::query(
        r#"
        insert into sys_dictionary_details
        (label, value, extend, status, sort, sys_dictionary_id, parent_id, level, path)
        values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(payload.label)
    .bind(payload.value)
    .bind(payload.extend)
    .bind(payload.status)
    .bind(payload.sort)
    .bind(dictionary_id)
    .bind(payload.parent_id)
    .bind(level)
    .bind(path)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn update_detail(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    detail_id: i64,
    payload: DictionaryDetailInput,
) -> Result<(), DictionaryError> {
    let mut transaction = pool.begin().await?;
    if let Some(parent_id) = payload.parent_id {
        let invalid_parent: bool = sqlx::query_scalar(
            r#"
            with recursive subtree as (
                select id from sys_dictionary_details
                where sys_dictionary_id = $1 and id = $2
                union all
                select child.id from sys_dictionary_details child
                join subtree parent on child.parent_id = parent.id
                where child.sys_dictionary_id = $1
            )
            select exists(select 1 from subtree where id = $3)
            "#,
        )
        .bind(dictionary_id)
        .bind(detail_id)
        .bind(parent_id)
        .fetch_one(&mut *transaction)
        .await?;
        if invalid_parent {
            return Err(DictionaryError::InvalidParent {
                dictionary_id,
                detail_id,
                parent_id,
            });
        }
    }
    let (level, path) = match payload.parent_id {
        Some(parent_id) => {
            let parent_info: Option<(i32, String)> = sqlx::query_as(
                "select level, path from sys_dictionary_details where sys_dictionary_id = $1 and id = $2",
            )
            .bind(dictionary_id)
            .bind(parent_id)
            .fetch_optional(&mut *transaction)
            .await?;

            match parent_info {
                Some((level, path)) => {
                    let new_path = if path.is_empty() {
                        parent_id.to_string()
                    } else {
                        format!("{path},{parent_id}")
                    };
                    Ok((level + 1, new_path))
                }
                None => Err(DictionaryError::DetailNotFound {
                    dictionary_id,
                    detail_id: parent_id,
                }),
            }
        }
        None => Ok((0, String::new())),
    }?;
    let result = sqlx::query(
        r#"
        update sys_dictionary_details
        set label = $1, value = $2, extend = $3, status = $4, sort = $5,
            sys_dictionary_id = $6, parent_id = $7, level = $8, path = $9
        where id = $10 and sys_dictionary_id = $11
        "#,
    )
    .bind(payload.label)
    .bind(payload.value)
    .bind(payload.extend)
    .bind(payload.status)
    .bind(payload.sort)
    .bind(dictionary_id)
    .bind(payload.parent_id)
    .bind(level)
    .bind(path)
    .bind(detail_id)
    .bind(dictionary_id)
    .execute(&mut *transaction)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DictionaryError::DetailNotFound {
            dictionary_id,
            detail_id,
        });
    }
    sqlx::query(
        r#"
        with recursive descendants as (
            select id, level, path
            from sys_dictionary_details
            where sys_dictionary_id = $1 and id = $2
            union all
            select child.id,
                   parent.level + 1,
                   case when parent.path = '' then parent.id::text
                        else parent.path || ',' || parent.id::text end
            from sys_dictionary_details child
            join descendants parent on child.parent_id = parent.id
            where child.sys_dictionary_id = $1
        )
        update sys_dictionary_details detail
        set level = descendants.level, path = descendants.path
        from descendants
        where detail.id = descendants.id
          and detail.sys_dictionary_id = $1
          and detail.id <> $2
        "#,
    )
    .bind(dictionary_id)
    .bind(detail_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub(crate) async fn find_detail(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    detail_id: i64,
) -> Result<Option<SysDictionaryDetail>, sqlx::Error> {
    sqlx::query_as::<_, SysDictionaryDetailRow>(
        r#"
        select id, label, value, extend, status, sort, sys_dictionary_id, parent_id, level, path
        from sys_dictionary_details where sys_dictionary_id = $1 and id = $2
        "#,
    )
    .bind(dictionary_id)
    .bind(detail_id)
    .fetch_optional(pool)
    .await
    .map(|opt| opt.map(detail_from_row))
}

pub(crate) async fn delete_detail(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    detail_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        with recursive subtree as (
            select id
            from sys_dictionary_details
            where sys_dictionary_id = $1 and id = $2
            union all
            select child.id
            from sys_dictionary_details child
            join subtree parent on child.parent_id = parent.id
            where child.sys_dictionary_id = $1
        )
        delete from sys_dictionary_details
        where sys_dictionary_id = $1 and id in (select id from subtree)
        "#,
    )
    .bind(dictionary_id)
    .bind(detail_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) async fn tree_by_dictionary(
    pool: &sqlx::PgPool,
    sys_dictionary_id: i64,
) -> Result<Vec<SysDictionaryDetail>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SysDictionaryDetailRow>(
        r#"
        select id, label, value, extend, status, sort, sys_dictionary_id, parent_id, level, path
        from sys_dictionary_details
        where sys_dictionary_id = $1
        order by sort asc, id asc
        "#,
    )
    .bind(sys_dictionary_id)
    .fetch_all(pool)
    .await?;

    Ok(build_detail_tree(&rows, None))
}

pub(crate) async fn tree_by_type(
    pool: &sqlx::PgPool,
    dict_type: &str,
) -> Result<Vec<SysDictionaryDetail>, sqlx::Error> {
    if let Some(dictionary) = find_by_type(pool, dict_type).await? {
        return tree_by_dictionary(pool, dictionary.id).await;
    }
    Ok(Vec::new())
}

pub(crate) async fn details_by_parent(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    parent_id: i64,
) -> Result<Vec<SysDictionaryDetail>, sqlx::Error> {
    let rows = sqlx::query_as::<_, SysDictionaryDetailRow>(
        r#"
        select id, label, value, extend, status, sort, sys_dictionary_id, parent_id, level, path
        from sys_dictionary_details
        where sys_dictionary_id = $1 and parent_id = $2
        order by sort asc, id asc
        "#,
    )
    .bind(dictionary_id)
    .bind(parent_id)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(detail_from_row).collect())
}

pub(crate) async fn detail_path(
    pool: &sqlx::PgPool,
    dictionary_id: i64,
    detail_id: i64,
) -> Result<Vec<SysDictionaryDetail>, sqlx::Error> {
    let Some(item) = find_detail(pool, dictionary_id, detail_id).await? else {
        return Ok(Vec::new());
    };
    if item.path.is_empty() {
        return Ok(vec![item]);
    }
    let mut ids = item
        .path
        .split(',')
        .filter_map(|part| part.parse::<i64>().ok())
        .collect::<Vec<_>>();
    ids.push(item.id);
    let rows = sqlx::query_as::<_, SysDictionaryDetailRow>(
        r#"
        select id, label, value, extend, status, sort, sys_dictionary_id, parent_id, level, path
        from sys_dictionary_details
        where sys_dictionary_id = $1 and id = any($2)
        order by level asc, id asc
        "#,
    )
    .bind(dictionary_id)
    .bind(&ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(detail_from_row).collect())
}

pub(crate) async fn export_dictionary(
    pool: &sqlx::PgPool,
    id: i64,
) -> Result<Option<DictionaryWithDetails>, sqlx::Error> {
    let Some(dictionary) = find(pool, id).await? else {
        return Ok(None);
    };
    let details = tree_by_dictionary(pool, id).await?;
    Ok(Some(DictionaryWithDetails {
        dictionary,
        details,
    }))
}

async fn detail_level_and_path(
    pool: &sqlx::PgPool,
    parent_id: Option<i64>,
    dictionary_id: i64,
) -> Result<Option<(i32, String)>, sqlx::Error> {
    if let Some(parent_id) = parent_id {
        let parent: Option<(i32, String)> =
            sqlx::query_as(
                "select level, path from sys_dictionary_details where sys_dictionary_id = $1 and id = $2",
            )
                .bind(dictionary_id)
                .bind(parent_id)
                .fetch_optional(pool)
                .await?;
        if let Some((level, path)) = parent {
            return Ok(Some((
                level + 1,
                if path.is_empty() {
                    parent_id.to_string()
                } else {
                    format!("{path},{parent_id}")
                },
            )));
        }
        return Ok(None);
    }
    Ok(Some((0, String::new())))
}

fn build_detail_tree(
    rows: &[SysDictionaryDetailRow],
    parent_id: Option<i64>,
) -> Vec<SysDictionaryDetail> {
    let mut list = rows
        .iter()
        .filter(|row| row.parent_id == parent_id)
        .map(|row| {
            let mut item = detail_from_row(row.clone());
            item.children = build_detail_tree(rows, Some(row.id));
            item
        })
        .collect::<Vec<_>>();
    list.sort_by_key(|item| (item.sort, item.id));
    list
}

fn detail_from_row(row: SysDictionaryDetailRow) -> SysDictionaryDetail {
    SysDictionaryDetail {
        id: row.id,
        label: row.label,
        value: row.value,
        extend: row.extend,
        status: row.status,
        sort: row.sort,
        sys_dictionary_id: row.sys_dictionary_id,
        parent_id: row.parent_id,
        level: row.level,
        path: row.path,
        children: Vec::new(),
    }
}
