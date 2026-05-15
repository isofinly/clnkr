use crate::constants;
use crate::types::transcript::{ComplexSegment, ComplexTranscriptOutput, Speaker};

/// Builds the system prompt for chunk N.
///
/// Chunk 0 uses the base transcription prompt unchanged.
/// Every subsequent chunk appends a summary of the previous chunk so the
/// model knows what came before (speaker labels, topic, register, …).
pub fn build_system_prompt(chunk_index: usize, summaries: &[String]) -> String {
    let base = constants::TRANSCRIPTION_SYSTEM_PROMPT;
    if chunk_index == 0 || summaries.is_empty() {
        base.to_string()
    } else {
        let summary = summaries.last().unwrap();
        let ctx = constants::CHUNK_CONTEXT_PROMPT.replace("{summary}", summary);
        format!("{base}\n\n{ctx}")
    }
}

/// Adds `offset_seconds` to every segment timestamp and `id_offset` to every
/// segment id so that the chunk's local numbers become globally unique.
pub fn shift_segments(
    mut segments: Vec<ComplexSegment>,
    offset_seconds: f64,
    id_offset: u64,
) -> Vec<ComplexSegment> {
    for seg in &mut segments {
        seg.start_seconds += offset_seconds;
        seg.end_seconds += offset_seconds;
        seg.id += id_offset;
    }
    segments
}

/// Returns the last `n` segments (or fewer if the slice is short).
pub fn extract_tail(segments: &[ComplexSegment], n: usize) -> Vec<ComplexSegment> {
    segments.iter().rev().take(n).rev().cloned().collect()
}

/// Returns the first `n` segments.
pub fn extract_head(segments: &[ComplexSegment], n: usize) -> Vec<ComplexSegment> {
    segments.iter().take(n).cloned().collect()
}

/// Extracts a deduplicated, order-preserving speaker list from a segment slice.
pub fn dedup_speakers(segments: &[ComplexSegment]) -> Vec<Speaker> {
    let mut speakers = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for seg in segments {
        if !seen.contains(&seg.speaker.speaker_id) {
            seen.insert(seg.speaker.speaker_id.clone());
            speakers.push(seg.speaker.clone());
        }
    }
    speakers
}

/// Re-assigns sequential ids starting at 1.  Call after stitching so the
/// frontend receives a clean, gap-free numbering.
pub fn renumber_segments(segments: &mut [ComplexSegment]) {
    for (i, seg) in segments.iter_mut().enumerate() {
        seg.id = (i + 1) as u64;
    }
}

/// Unifies speaker IDs across chunks by label.  If two different `speaker_id`s
/// share the same `label`, all segments are rewritten to use the first-seen
/// `speaker_id` so the speaker list stays deduplicated.
pub fn reconcile_speakers(segments: &mut [ComplexSegment]) -> Vec<Speaker> {
    let mut canonical: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut speakers: Vec<Speaker> = Vec::new();

    for seg in segments.iter() {
        if let Some(existing_id) = canonical.get(&seg.speaker.label) {
            if existing_id != &seg.speaker.speaker_id {
                // Different ID for same label — will rewrite below.
            }
        } else {
            canonical.insert(seg.speaker.label.clone(), seg.speaker.speaker_id.clone());
            speakers.push(seg.speaker.clone());
        }
    }

    for seg in segments.iter_mut() {
        if let Some(canon_id) = canonical.get(&seg.speaker.label) {
            seg.speaker.speaker_id = canon_id.clone();
        }
    }

    speakers
}

/// Converts a `ComplexTranscriptOutput` into its raw JSON value.  Convenience
/// wrapper so the handler does not need to import `serde_json` just for this.
pub fn transcript_to_value(t: &ComplexTranscriptOutput) -> serde_json::Value {
    serde_json::to_value(t).unwrap_or(serde_json::Value::Null)
}
