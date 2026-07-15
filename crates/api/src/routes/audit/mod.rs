pub(crate) mod login_logs;
pub(crate) mod operation_logs;

use axum::Router;

use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/login-logs", login_logs::routes())
        .nest("/operation-logs", operation_logs::routes())
}
