use gemini_client_api::gemini::utils::{GeminiSchema, gemini_schema};
use serde::{Deserialize, Serialize};

use super::transcript::TranscriptWord;

/// Input for the flash-lite word-fill guard.
/// Sent when a chunk transcription returns segments with empty `words` arrays.
#[derive(Debug, Serialize)]
#[gemini_schema]
pub struct WordFillInput {
    /// Segments that need word-level readings and romanizations.
    pub segments: Vec<WordFillSegment>,
}

#[derive(Debug, Serialize)]
#[gemini_schema]
pub struct WordFillSegment {
    /// Segment identifier used to map the result back to the original segment.
    pub id: u64,
    /// The raw Japanese text to be broken down into words.
    pub raw_text: String,
}

/// Output from the flash-lite word-fill guard.
#[derive(Debug, Deserialize)]
#[gemini_schema]
pub struct WordFillOutput {
    /// One entry per input segment, in the same order.
    pub segments: Vec<WordFillResultSegment>,
}

#[derive(Debug, Deserialize)]
#[gemini_schema]
pub struct WordFillResultSegment {
    pub id: u64,
    /// Populated word list. Must contain at least one element.
    pub words: Vec<TranscriptWord>,
}
