use axum::response::{IntoResponse, Response, Sse, sse::Event};
use hex;
use serde_json::json;
use sha2::{Digest, Sha256};

pub(crate) fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Builds a minimal SSE response that immediately emits an `error` event and
/// closes. Used for errors that occur before the stream starts.
pub(crate) fn sse_error_response(message: impl Into<String>) -> Response {
    let payload = json!({ "message": message.into() }).to_string();
    let stream = tokio_stream::iter(vec![Ok::<Event, std::convert::Infallible>(
        Event::default().event("error").data(payload),
    )]);
    Sse::new(stream).into_response()
}
