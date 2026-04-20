use axum::Json;
use axum::response::{IntoResponse, Response};
use gemini_client_api::gemini::utils::{GeminiSchema, gemini_schema};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TranslateQuery {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationInputRequest {
    pub translation_input: String,
    pub context: Option<String>,
    pub segment_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[gemini_schema]
pub struct TranslationInput {
    pub translation_input: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TranslationOutputResponse {
    pub served_from_cache: bool,
    pub input_hash: String,
    pub translation: TranslationOutput,
}

impl IntoResponse for TranslationOutputResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationErrorResponse {
    pub message: String,
}

impl IntoResponse for TranslationErrorResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct TranslationOutput {
    // TODO: We can use id or something else to identify the source text
    pub source_text: String,
    pub phrase_breakdowns: Vec<PhraseBreakdown>,
    pub grammar_constructions: Vec<GrammarConstruction>,
    pub kanji_words: Vec<KanjiWord>,
    pub translations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct PhraseBreakdown {
    pub phrase: String,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct Token {
    pub token: String,
    pub meaning: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct GrammarConstruction {
    pub name: String,
    pub pattern: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct KanjiWord {
    pub kanji: String,
    pub reading: String,
}
