use gemini_client_api::gemini::utils::{GeminiSchema, gemini_schema};
use serde::{Deserialize, Serialize};

/// Result of asking flash-lite to reconcile the overlap between the tail of
/// chunk N and the head of chunk N+1.
#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct OverlapResult {
    /// The seconds (relative to chunk N start) after which chunk N's tail
    /// should be discarded because it is covered by chunk N+1.
    pub overlap_end_seconds: f64,
    /// Segment IDs (from chunk N) that are duplicates or cut-off and should
    /// be replaced.
    pub replace_segment_ids: Vec<u64>,
    /// The corrected boundary segments that stitch the two chunks together.
    /// Timestamps are relative to chunk N and continue into chunk N+1.
    pub new_segments: Vec<StitchedSegment>,
}

/// A segment produced by the stitching model. It carries an extra flag so the
/// orchestrator knows whether this segment originated from the previous chunk,
/// the next chunk, or was reconstructed by the model.
#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct StitchedSegment {
    pub id: u64,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub raw_text: String,
    pub words: Vec<StitchedWord>,
    pub translation: String,
    pub speaker: StitchedSpeaker,
    /// True when the model reconstructed this segment from both chunks.
    pub is_reconstructed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct StitchedWord {
    pub text: String,
    pub reading: String,
    pub romanization: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[gemini_schema]
pub struct StitchedSpeaker {
    pub speaker_id: String,
    pub label: String,
}
