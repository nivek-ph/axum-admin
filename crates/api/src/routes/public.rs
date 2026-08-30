use axum::Router;

use super::{auth, files, health};

pub fn router() -> Router<crate::state::AppState> {
    Router::new()
        .merge(health::routes())
        .merge(auth::public_routes())
}

pub fn root_router() -> Router<crate::state::AppState> {
    files::public_routes()
}
