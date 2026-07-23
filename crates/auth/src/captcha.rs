use captcha_rs::{Captcha, CaptchaBuilder};
use redis::aio::MultiplexedConnection;
use uuid::Uuid;

const CAPTCHA_KEY_PREFIX: &str = "auth:captcha:";
const CAPTCHA_TTL_SECONDS: u64 = 300;
const CAPTCHA_CHARACTERS: [char; 31] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'M',
    'N', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

// get a captcha with the given text or a random text
fn get_captcha(text: Option<&str>) -> Captcha {
    let mut builder = CaptchaBuilder::new()
        .length(4)
        .chars(CAPTCHA_CHARACTERS.to_vec())
        .width(148)
        .height(44)
        .dark_mode(false)
        .complexity(1)
        .compression(92)
        .drop_shadow(false)
        .interference_lines(1)
        .interference_ellipses(0)
        .distortion(0);

    // use the provided text if it exists
    if let Some(text) = text {
        builder = builder.text(text.to_string());
    }

    builder.build()
}

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
        let captcha = get_captcha(None);
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

    use super::{CAPTCHA_CHARACTERS, CaptchaError, get_captcha};

    #[test]
    fn captcha_characters_do_not_use_a_dark_drop_shadow() {
        let captcha = get_captcha(Some("spRk"));
        let has_dark_shadow = captcha
            .image
            .to_rgb8()
            .pixels()
            .any(|pixel| pixel.0 == [20, 20, 20]);

        assert!(!has_dark_shadow);
    }

    #[test]
    fn captcha_alphabet_uses_readable_uppercase_characters() {
        assert!(
            CAPTCHA_CHARACTERS
                .iter()
                .all(|character| { character.is_ascii_digit() || character.is_ascii_uppercase() })
        );
        assert!(
            CAPTCHA_CHARACTERS
                .iter()
                .all(|character| !matches!(character, '0' | '1' | 'I' | 'L' | 'O'))
        );
    }

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
