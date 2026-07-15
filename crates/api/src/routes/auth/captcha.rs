use admin_httpz::{ApiResponse, AppResult};
use axum::{Json, extract::State};
use serde_json::Value;

use crate::{errors::auth::CAPTCHA_OPERATION_FAILED, state::AppState};

#[utoipa::path(
    post,
    path = "/api/auth/captcha",
    tag = "auth",
    responses(
        (status = 200, description = "Captcha config", body = crate::docs::CaptchaResponse)
    )
)]
pub async fn captcha(State(state): State<AppState>) -> AppResult<Json<ApiResponse<Value>>> {
    let challenge = state
        .captcha
        .create()
        .await
        .map_err(|error| CAPTCHA_OPERATION_FAILED.into_error().with_source(error))?;
    Ok(Json(ApiResponse::ok(serde_json::json!({
        "captchaLength": 4,
        "picPath": challenge.image,
        "captchaId": challenge.id,
        "openCaptcha": true
    }))))
}
