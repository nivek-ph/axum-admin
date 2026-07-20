use std::{convert::Infallible, ops::Deref};

use axum::{
    extract::FromRequestParts,
    http::{header::USER_AGENT, request::Parts},
};

/// `User-Agent` header value for audit/logging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserAgent(pub String);

impl Deref for UserAgent {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequestParts<S> for UserAgent
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let value = parts
            .headers
            .get(USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use axum::{
        extract::FromRequestParts,
        http::{HeaderValue, Request},
    };

    use super::*;

    #[tokio::test]
    async fn reads_user_agent_header() {
        let mut request = Request::new(());
        request
            .headers_mut()
            .insert(USER_AGENT, HeaderValue::from_static("ava-test/1.0"));
        let (mut parts, _) = request.into_parts();

        let UserAgent(agent) = UserAgent::from_request_parts(&mut parts, &())
            .await
            .expect("user agent should resolve");
        assert_eq!(agent, "ava-test/1.0");
    }

    #[tokio::test]
    async fn empty_when_header_missing() {
        let (mut parts, _) = Request::new(()).into_parts();

        let UserAgent(agent) = UserAgent::from_request_parts(&mut parts, &())
            .await
            .expect("user agent should resolve");
        assert_eq!(agent, "");
    }
}
