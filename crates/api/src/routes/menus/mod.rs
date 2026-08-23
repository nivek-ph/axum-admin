pub(crate) mod dto;
mod handler;

use axum::{Router, routing::get};
pub use handler::*;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/current", get(get_menu))
        .route("/tree", get(get_base_menu_tree))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::extractors::current_user::AuthenticatedUser;

    async fn request_current_menu(pool: sqlx::PgPool, user_id: i64) -> Value {
        let state = crate::state::tests::test_state(pool).await;
        let response = routes()
            .with_state(state)
            .oneshot(
                Request::builder()
                    .uri("/current")
                    .extension(AuthenticatedUser { id: user_id })
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn collect_menu_ids(items: &[Value], ids: &mut BTreeSet<i64>) {
        for item in items {
            ids.insert(item["id"].as_i64().unwrap());
            collect_menu_ids(item["children"].as_array().unwrap(), ids);
        }
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn current_menu_route_uses_assigned_enabled_nodes_and_keeps_envelope(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id
            )
            values (9001, 'menu-user', 'menu-user', 'hash', 'Menu User', '', true, 1)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into sys_roles (id, name, code, status) values (9001, 'Menu Role', 'menu-role', 'enabled')",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            r#"
            insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5)
            values
                ('g', 'user:9001', 'role:9001', '', '', '', ''),
                ('p', 'role:9001', 'system:user:create', '', '', '', ''),
                ('p', 'role:9001', 'system:user:list', '', '', '', '')
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("update sys_menus set status = 'disabled' where id = 12")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            r#"
            delete from casbin_rule
            where ptype = 'p'
              and v0 = 'role:1'
              and v1 in (
                  select permission
                  from sys_menus
                  where (id = 12 or parent_id = 12) and permission is not null
              )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        let body = request_current_menu(pool, 9001).await;

        assert_eq!(body["code"], "OK");
        assert_eq!(body["message"], "ok");
        assert_eq!(
            body["data"]["permissions"],
            serde_json::json!(["system:user:create", "system:user:list"])
        );
        let menus = body["data"]["menus"].as_array().unwrap();
        assert_eq!(menus.len(), 1);
        assert_eq!(menus[0]["id"], 10);
        assert_eq!(menus[0]["children"].as_array().unwrap().len(), 1);
        assert_eq!(menus[0]["children"][0]["id"], 11);
    }

    #[sqlx::test(migrations = "../../migrations")]
    async fn current_menu_route_returns_every_enabled_menu_for_super_admin(pool: sqlx::PgPool) {
        sqlx::query(
            r#"
            insert into sys_users (
                id, uuid, username, password_hash, nick_name, header_img,
                enable, dept_id
            )
            values (9002, 'system-menu-user', 'system-menu-user', 'hash', 'System Menu User', '', true, 1)
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "insert into casbin_rule (ptype, v0, v1, v2, v3, v4, v5) values ('g', 'user:9002', 'role:1', '', '', '', '')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let expected_ids = sqlx::query_scalar::<_, i64>(
            "select id from sys_menus where status = 'enabled' and menu_type <> 'action' order by id",
        )
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .collect::<BTreeSet<_>>();
        let mut permissions = sqlx::query_scalar::<_, String>(
            "select permission from sys_menus where status = 'enabled' and permission is not null",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        permissions.sort();

        let body = request_current_menu(pool, 9002).await;

        let mut actual_ids = BTreeSet::new();
        collect_menu_ids(body["data"]["menus"].as_array().unwrap(), &mut actual_ids);
        assert_eq!(actual_ids, expected_ids);
        assert_eq!(body["data"]["permissions"], serde_json::json!(permissions));
    }
}
