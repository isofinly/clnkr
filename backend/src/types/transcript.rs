use gemini_client_api::gemini::utils::{GeminiSchema, gemini_schema};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct TranscribeQuery {
    /// this flag is reserved for a future `SimpleTranscriptOutput` path.
    #[serde(default)]
    pub transcript_words: bool,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
#[allow(dead_code)]
pub struct SimpleTranscriptOutput {
    pub source_language: String,
    pub target_language: String,
    pub total_duration_seconds: f64,
    pub speakers: Vec<Speaker>,
    pub segments: Vec<SimpleSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
#[allow(dead_code)]
pub struct ComplexTranscriptOutput {
    pub source_language: String,
    pub target_language: String,
    pub total_duration_seconds: f64,
    pub speakers: Vec<Speaker>,
    pub segments: Vec<ComplexSegment>,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct Speaker {
    pub speaker_id: String,
    pub label: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct SimpleSegment {
    pub id: i64,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub raw_text: String,
    pub translation: String,
    pub speaker_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct ComplexSegment {
    pub id: u64,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub raw_text: String,
    pub words: Vec<TranscriptWord>,
    pub translation: String,
    pub speaker: Speaker,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct TranscriptWord {
    pub text: String,
    pub reading: String,
    pub romanization: String,
}
