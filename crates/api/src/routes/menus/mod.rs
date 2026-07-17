mod dto;
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
    use iam::access::{AccessSnapshot, ResolvedDataScope};
    use serde_json::Value;
    use tower::ServiceExt;

    use super::*;
    use crate::extractors::current_access::CurrentAccess;

    fn snapshot(
        menu_ids: impl IntoIterator<Item = i64>,
        permissions: impl IntoIterator<Item = impl Into<String>>,
        super_admin: bool,
    ) -> AccessSnapshot {
        AccessSnapshot {
            version: 0,
            user_id: 1,
            role_codes: if super_admin {
                BTreeSet::from(["super_admin".to_string()])
            } else {
                BTreeSet::new()
            },
            menu_ids: menu_ids.into_iter().collect(),
            permissions: permissions.into_iter().map(Into::into).collect(),
            data_scope: ResolvedDataScope::All,
        }
    }

    async fn request_current_menu(pool: sqlx::PgPool, snapshot: AccessSnapshot) -> Value {
        let response = routes()
            .with_state(crate::state::test_state(pool))
            .oneshot(
                Request::builder()
                    .uri("/current")
                    .extension(CurrentAccess(snapshot))
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
        sqlx::query("update sys_menus set status = 'disabled' where id = 12")
            .execute(&pool)
            .await
            .unwrap();

        let body = request_current_menu(
            pool,
            snapshot(
                [10, 11, 12, 1101],
                ["system:user:create", "system:user:list"],
                false,
            ),
        )
        .await;

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
        let enabled_ids = sqlx::query_scalar::<_, i64>(
            "select id from sys_menus where status = 'enabled' order by id",
        )
        .fetch_all(&pool)
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
        let permissions = sqlx::query_scalar::<_, String>(
            "select permission from sys_menus where status = 'enabled' and permission is not null order by permission",
        )
        .fetch_all(&pool)
        .await
        .unwrap();
        let permission_refs = permissions.iter().map(String::as_str).collect::<Vec<_>>();

        let body = request_current_menu(pool, snapshot(enabled_ids, permission_refs, true)).await;

        let mut actual_ids = BTreeSet::new();
        collect_menu_ids(body["data"]["menus"].as_array().unwrap(), &mut actual_ids);
        assert_eq!(actual_ids, expected_ids);
    }
}
