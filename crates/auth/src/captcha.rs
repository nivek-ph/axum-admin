use captcha_rs::CaptchaBuilder;
use redis::aio::MultiplexedConnection;
use uuid::Uuid;

const CAPTCHA_KEY_PREFIX: &str = "auth:captcha:";
const CAPTCHA_TTL_SECONDS: u64 = 300;

pub struct CaptchaChallenge {
    pub id: String,
    pub image: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CaptchaError {
    #[error("captcha store is unavailable")]
    StoreUnavailable,
    #[error("captcha store operation failed")]
    Redis(#[from] redis::RedisError),
    #[error("captcha image rendering failed")]
    RenderFailed,
}

#[derive(Clone)]
pub struct CaptchaService {
    redis_connection: Option<MultiplexedConnection>,
}

impl CaptchaService {
    pub fn new(redis_connection: MultiplexedConnection) -> Self {
        Self {
            redis_connection: Some(redis_connection),
        }
    }

    pub fn without_store() -> Self {
        Self {
            redis_connection: None,
        }
    }

    pub async fn create(&self) -> Result<CaptchaChallenge, CaptchaError> {
        let id = Uuid::new_v4().to_string();
        let captcha = CaptchaBuilder::new()
            .length(4)
            .width(220)
            .height(64)
            .dark_mode(false)
            .complexity(2)
            .compression(85)
            .drop_shadow(true)
            .interference_lines(1)
            .interference_ellipses(0)
            .distortion(2)
            .build();
        let image = captcha.to_base64();
        let code = captcha.text;

        if image == "data:image/jpeg;base64," {
            return Err(CaptchaError::RenderFailed);
        }

        let key = format!("{CAPTCHA_KEY_PREFIX}{id}");
        let mut redis = self.redis_connection()?;
        redis::cmd("SETEX")
            .arg(key)
            .arg(CAPTCHA_TTL_SECONDS)
            .arg(code)
            .query_async::<()>(&mut redis)
            .await?;
        Ok(CaptchaChallenge { id, image })
    }

    pub async fn verify(&self, id: &str, answer: &str) -> Result<bool, CaptchaError> {
        let key = format!("{CAPTCHA_KEY_PREFIX}{id}");
        let mut redis = self.redis_connection()?;
        let expected: Option<String> = redis::cmd("GETDEL")
            .arg(key)
            .query_async(&mut redis)
            .await?;
        Ok(expected.is_some_and(|value| value.eq_ignore_ascii_case(answer.trim())))
    }

    fn redis_connection(&self) -> Result<MultiplexedConnection, CaptchaError> {
        self.redis_connection
            .clone()
            .ok_or(CaptchaError::StoreUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use redis::ErrorKind;

    use super::CaptchaError;

    #[test]
    fn redis_failure_keeps_a_stable_capability_message_and_source() {
        let source = redis::RedisError::from((ErrorKind::Io, "redis detail"));
        let error = CaptchaError::from(source);

        assert_eq!(error.to_string(), "captcha store operation failed");
        let source = error
            .source()
            .expect("store error should keep Redis source");
        assert!(source.downcast_ref::<redis::RedisError>().is_some());
    }
}
