use gemini_client_api::gemini::utils::{GeminiSchema, gemini_schema};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[gemini_schema]
pub struct ReadingInput {
    pub source_language: String,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Deserialize)]
#[gemini_schema]
pub struct Segment {
    pub id: u64,
    pub raw_text: String,
}

pub type ReadingOutputResponse = ReadingOutput;

#[derive(Debug, Serialize)]
#[gemini_schema]
pub struct ReadingOutput {
    pub segments: Vec<OutputSegment>,
}

#[derive(Debug, Serialize)]
#[gemini_schema]
pub struct OutputSegment {
    pub id: u64,
    pub words: Vec<Word>,
}

#[derive(Debug, Serialize)]
#[gemini_schema]
pub struct Word {
    pub text: String,
    pub reading: String,
}
