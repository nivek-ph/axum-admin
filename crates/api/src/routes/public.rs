use axum::Router;

use super::{auth, health};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::public_routes())
}
