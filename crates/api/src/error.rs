use std::{
    borrow::Cow,
    error::Error as StdError,
    fmt::{self, Display, Formatter},
};

use anyhow::Error as AnyError;
use axum::{
    Json,
    extract::rejection::JsonRejection,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing_error::SpanTrace;

use crate::response::ApiErrorResponse;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
enum ErrorKind {
    #[error("{2}")]
    Http(StatusCode, Cow<'static, str>, String),
    #[error("invalid request body")]
    JsonRejection(#[from] JsonRejection),
    #[error("invalid json: `{0}`")]
    InvalidJson(#[from] serde_json::Error),
    #[error("internal server error")]
    Any(#[from] anyhow::Error),
}

impl ErrorKind {
    fn status_code(status: StatusCode) -> Cow<'static, str> {
        status
            .canonical_reason()
            .map(|reason| reason.to_ascii_uppercase().replace(' ', "_").into())
            .unwrap_or_else(|| status.as_u16().to_string().into())
    }

    pub fn http(
        status: StatusCode,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> Self {
        Self::Http(status, code.into(), message.into())
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Http(status, _, _) => *status,
            Self::JsonRejection(_) | Self::InvalidJson(_) => StatusCode::BAD_REQUEST,
            Self::Any(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn code(&self) -> Cow<'static, str> {
        match self {
            Self::Http(_, code, _) => code.clone(),
            Self::Any(_) => Self::status_code(StatusCode::INTERNAL_SERVER_ERROR),
            Self::JsonRejection(_) | Self::InvalidJson(_) => {
                Self::status_code(StatusCode::BAD_REQUEST)
            }
        }
    }
}

#[derive(Debug)]
pub struct AppError {
    kind: ErrorKind,
    context: SpanTrace,
    source: Option<AnyError>,
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.kind)?;
        Display::fmt(&self.context, f)
    }
}

impl StdError for AppError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        if let Some(error) = self.source.as_ref() {
            return Some(error.as_ref());
        }
        self.kind.source()
    }
}

impl From<ErrorKind> for AppError {
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: SpanTrace::capture(),
            source: None,
        }
    }
}

impl From<JsonRejection> for AppError {
    fn from(error: JsonRejection) -> Self {
        ErrorKind::from(error).into()
    }
}

impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        ErrorKind::from(error).into()
    }
}

impl From<AnyError> for AppError {
    fn from(error: AnyError) -> Self {
        ErrorKind::from(error).into()
    }
}

impl AppError {
    pub fn new(
        status: StatusCode,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> Self {
        Self::custom(status, code, message)
    }

    pub fn custom(
        status: StatusCode,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> Self {
        ErrorKind::http(status, code, message).into()
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        let status = StatusCode::BAD_REQUEST;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::bad_request(message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        let status = StatusCode::UNAUTHORIZED;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn unauthenticated(message: impl Into<String>) -> Self {
        Self::unauthorized(message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        let status = StatusCode::FORBIDDEN;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        let status = StatusCode::NOT_FOUND;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        let status = StatusCode::CONFLICT;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn failed_precondition(message: impl Into<String>) -> Self {
        let status = StatusCode::PRECONDITION_FAILED;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn unavailable(message: impl Into<String>) -> Self {
        let status = StatusCode::SERVICE_UNAVAILABLE;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn internal(message: impl Into<String>) -> Self {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        ErrorKind::http(status, ErrorKind::status_code(status), message).into()
    }

    pub fn with_code(mut self, code: impl Into<Cow<'static, str>>) -> Self {
        let code = code.into();
        self.kind = match self.kind {
            ErrorKind::Http(status, _, message) => ErrorKind::Http(status, code, message),
            other => ErrorKind::Http(other.status(), code, other.to_string()),
        };
        self
    }

    pub fn with_source(mut self, error: impl Into<AnyError>) -> Self {
        self.source = Some(error.into());
        self
    }

    pub fn from_error(status: StatusCode, error: impl Into<AnyError>) -> Self {
        let error = error.into();
        let message = error.to_string();
        Self::custom(status, ErrorKind::status_code(status), message).with_source(error)
    }

    pub fn bad_request_error(error: impl Into<AnyError>) -> Self {
        Self::from_error(StatusCode::BAD_REQUEST, error)
    }

    pub fn validation_error(error: impl Into<AnyError>) -> Self {
        Self::bad_request_error(error)
    }

    pub fn unauthenticated_error(error: impl Into<AnyError>) -> Self {
        Self::from_error(StatusCode::UNAUTHORIZED, error)
    }

    pub fn internal_error(error: impl Into<AnyError>) -> Self {
        Self::from_error(StatusCode::INTERNAL_SERVER_ERROR, error)
    }

    pub fn status(&self) -> StatusCode {
        self.kind.status()
    }

    pub fn code(&self) -> Cow<'static, str> {
        self.kind.code()
    }

    pub fn message(&self) -> String {
        match &self.kind {
            ErrorKind::Http(_, _, message) => message.clone(),
            _ => self.kind.to_string(),
        }
    }

    pub fn span_trace(&self) -> &SpanTrace {
        &self.context
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorSpec {
    status: StatusCode,
    code: &'static str,
    message: &'static str,
}

impl ErrorSpec {
    pub(crate) const fn new(status: StatusCode, code: &'static str, message: &'static str) -> Self {
        Self {
            status,
            code,
            message,
        }
    }

    pub(crate) const fn bad_request(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub(crate) const fn validation(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub(crate) const fn unauthorized(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub(crate) const fn forbidden(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub(crate) const fn not_found(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub(crate) const fn conflict(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    pub(crate) const fn failed_precondition(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::PRECONDITION_FAILED, code, message)
    }

    pub(crate) const fn internal(code: &'static str, message: &'static str) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub(crate) fn into_error(self) -> AppError {
        self.into()
    }
}

impl From<ErrorSpec> for ErrorKind {
    fn from(spec: ErrorSpec) -> Self {
        Self::http(spec.status, spec.code, spec.message)
    }
}

impl From<ErrorSpec> for AppError {
    fn from(spec: ErrorSpec) -> Self {
        ErrorKind::from(spec).into()
    }
}

const INTERNAL_SERVER_ERROR: &str = "internal server error";

/// Generate a public response message for a given error kind and diagnostic message.
///
/// This function takes an `ErrorKind` and a diagnostic message, and returns a public response message.
/// The public response message is used to respond to the client with a message that is not sensitive to the internal implementation.
///
/// # Arguments
///
/// * `kind` - The error kind to generate a public response message for.
/// * `diagnostic` - The diagnostic message to generate a public response message for.
///
/// # Returns
///
/// A public response message.
///
/// # Examples
///
/// ```ignore
/// let error = AppError::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR", "internal server error");
/// let public_response_message = public_response_message(&error.kind, &error.message());
/// assert_eq!(public_response_message, "internal server error");
/// ```
///
/// ```ignore
/// let error = AppError::new(StatusCode::BAD_REQUEST, "BAD_REQUEST", "bad request");
/// let public_response_message = public_response_message(&error.kind, &error.message());
/// assert_eq!(public_response_message, "bad request");
/// ```
///
/// ```ignore
/// let error = AppError::new(StatusCode::UNAUTHORIZED, "UNAUTHORIZED", "unauthorized");
/// let public_response_message = public_response_message(&error.kind, &error.message());
/// assert_eq!(public_response_message, "unauthorized");
/// ```
///
/// ```ignore
/// let error = AppError::new(StatusCode::FORBIDDEN, "FORBIDDEN", "forbidden");
/// let public_response_message = public_response_message(&error.kind, &error.message());
/// assert_eq!(public_response_message, "forbidden");
/// ```
fn public_response_message(kind: &ErrorKind, diagnostic: &str) -> String {
    match kind {
        ErrorKind::Any(_) => INTERNAL_SERVER_ERROR.to_string(),
        ErrorKind::Http(status, _, _) => {
            if *status == StatusCode::REQUEST_TIMEOUT {
                return "request timed out".to_string();
            }
            if *status == StatusCode::SERVICE_UNAVAILABLE {
                return "service unavailable".to_string();
            }
            if status.is_server_error() {
                return INTERNAL_SERVER_ERROR.to_string();
            }
            diagnostic.to_string()
        }
        _ => diagnostic.to_string(),
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        let code = self.code().into_owned();
        let message = self.message();
        let public_message = public_response_message(&self.kind, &message);
        let span_trace = self.context;
        let kind = self.kind;
        let source = self.source;

        if status.is_server_error() {
            tracing::error!(
                status = status.as_u16(),
                code = %code,
                message = %message,
                error = %kind,
                source = ?source,
                span_trace = %span_trace,
                "request failed",
            );
        } else {
            tracing::warn!(
                status = status.as_u16(),
                code = %code,
                message = %message,
                error = %kind,
                source = ?source,
                span_trace = %span_trace,
                "request failed",
            );
        }

        let body = ApiErrorResponse::new(code, public_message, None);
        (status, Json(body)).into_response()
    }
}
