pub(crate) mod events;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().nest("/audit/events", events::routes())
}
