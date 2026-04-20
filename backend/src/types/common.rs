use axum::Json;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CachedTranslation {
    pub response_json: serde_json::Value,
    pub input_hash: String,
}

#[derive(Serialize, Deserialize)]
pub struct CachedTranscription {
    pub response_json: serde_json::Value,
    pub audio_signature: String,
    pub transcript_type: String,
    pub file_name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CachedTranscriptionResponse {
    /// Up to 25 transcriptions
    pub transcriptions: Vec<CachedTranscription>,
    pub total_translations: i64,
}

impl IntoResponse for CachedTranscriptionResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[derive(Serialize, Deserialize)]
pub struct CachedTranslationResponse {
    /// Up to 100 translations
    pub translations: Vec<CachedTranslation>,
    pub total_transcriptions: i64,
}

impl IntoResponse for CachedTranslationResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
