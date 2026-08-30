use std::ops::Range;

use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{AppResult, state::AppState};

pub(super) fn routes() -> Router<AppState> {
    Router::new().route("/uploads/{*object}", get(serve_local_upload))
}

async fn serve_local_upload(
    State(state): State<AppState>,
    Path(object): Path<String>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let Some(file) = state.files.read_local_object(&object).await? else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };
    let total_size = file.size;
    let range = match requested_range(&headers, total_size) {
        Ok(range) => range,
        Err(()) => return Ok(range_not_satisfiable(total_size)),
    };
    let content_length = range
        .as_ref()
        .map_or(total_size, |range| range.end - range.start);
    let stream = file.into_stream(range.clone()).await?;
    let content_type = mime_guess::from_path(&object).first_or_octet_stream();
    let mut response = Body::from_stream(stream).into_response();
    if let Some(range) = range {
        *response.status_mut() = StatusCode::PARTIAL_CONTENT;
        response.headers_mut().insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!(
                "bytes {}-{}/{}",
                range.start,
                range.end - 1,
                total_size
            ))
            .expect("byte range should be a valid header"),
        );
    }
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(content_type.as_ref()).expect("MIME type should be a valid header"),
    );
    response
        .headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from(content_length));
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    Ok(response)
}

fn requested_range(headers: &HeaderMap, size: u64) -> Result<Option<Range<u64>>, ()> {
    let Some(value) = headers.get(header::RANGE) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| ())?;
    let value = value.strip_prefix("bytes=").ok_or(())?;
    if value.contains(',') {
        return Err(());
    }
    let (start, end) = value.split_once('-').ok_or(())?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().map_err(|_| ())?;
        if suffix == 0 || size == 0 {
            return Err(());
        }
        return Ok(Some(size.saturating_sub(suffix)..size));
    }
    let start = start.parse::<u64>().map_err(|_| ())?;
    if start >= size {
        return Err(());
    }
    let end = if end.is_empty() {
        size - 1
    } else {
        end.parse::<u64>().map_err(|_| ())?.min(size - 1)
    };
    if end < start {
        return Err(());
    }
    Ok(Some(start..end + 1))
}

fn range_not_satisfiable(size: u64) -> Response {
    let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
    response.headers_mut().insert(
        header::CONTENT_RANGE,
        HeaderValue::from_str(&format!("bytes */{size}"))
            .expect("file size should be a valid header"),
    );
    response
        .headers_mut()
        .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    response
}
