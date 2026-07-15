use axum::{Json, extract::State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{ApiResponse, AppResult, state::AppState};

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CaptchaData {
    #[serde(rename = "captchaLength")]
    pub captcha_length: i32,
    #[serde(rename = "picPath")]
    pub pic_path: String,
    #[serde(rename = "captchaId")]
    pub captcha_id: String,
    #[serde(rename = "openCaptcha")]
    pub open_captcha: bool,
}

#[utoipa::path(
    post,
    path = "/auth/captcha",
    tag = "auth",
    responses(
        (status = 200, description = "Captcha config", body = ApiResponse<CaptchaData>)
    )
)]
pub async fn captcha(State(state): State<AppState>) -> AppResult<Json<ApiResponse<CaptchaData>>> {
    let challenge = state.captcha.create().await?;
    Ok(Json(ApiResponse::ok(CaptchaData {
        captcha_length: 4,
        pic_path: challenge.image,
        captcha_id: challenge.id,
        open_captcha: true,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captcha_data_keeps_transport_field_names() {
        let value = serde_json::to_value(CaptchaData {
            captcha_length: 4,
            pic_path: "image".to_string(),
            captcha_id: "id".to_string(),
            open_captcha: true,
        })
        .expect("captcha data should serialize");

        assert_eq!(
            value,
            serde_json::json!({
                "captchaLength": 4,
                "picPath": "image",
                "captchaId": "id",
                "openCaptcha": true
            })
        );
    }
}
