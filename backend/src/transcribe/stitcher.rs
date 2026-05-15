use crate::types::{
    overlap::{OverlapResult, StitchedSegment},
    transcript::{ComplexSegment, Speaker},
};

/// Applies an `OverlapResult` to merge two consecutive chunks into one
/// continuous transcript.
///
/// `prev`  – all segments accumulated from chunk N-1 (already shifted to
///           global timestamps).
/// `next`  – all segments from chunk N (already shifted to global timestamps).
/// `result`– the overlap model's output.
///
/// Returns the unified segment list and a deduplicated speaker list.
pub fn apply_stitch(
    mut prev: Vec<ComplexSegment>,
    next: Vec<ComplexSegment>,
    result: OverlapResult,
    global_speakers: &[Speaker],
) -> (Vec<ComplexSegment>, Vec<Speaker>) {
    let discard_count = prev
        .iter()
        .rev()
        .take_while(|s| result.replace_segment_ids.contains(&s.id))
        .count();

    let keep_up_to = prev.len().saturating_sub(discard_count);
    prev.truncate(keep_up_to);

    let mut merged = prev;
    for stitched in result.new_segments {
        merged.push(stitched.into());
    }

    for seg in next {
        if seg.start_seconds >= result.overlap_end_seconds {
            merged.push(seg);
        }
    }

    let mut speakers: Vec<Speaker> = global_speakers.to_vec();
    let mut seen: std::collections::HashSet<String> =
        speakers.iter().map(|s| s.speaker_id.clone()).collect();

    for seg in &merged {
        if !seen.contains(&seg.speaker.speaker_id) {
            seen.insert(seg.speaker.speaker_id.clone());
            speakers.push(seg.speaker.clone());
        }
    }

    (merged, speakers)
}

impl From<StitchedSegment> for ComplexSegment {
    fn from(s: StitchedSegment) -> Self {
        ComplexSegment {
            id: s.id,
            start_seconds: s.start_seconds,
            end_seconds: s.end_seconds,
            raw_text: s.raw_text,
            words: s
                .words
                .into_iter()
                .map(|w| crate::types::transcript::TranscriptWord {
                    text: w.text,
                    reading: w.reading,
                    romanization: w.romanization,
                })
                .collect(),
            translation: s.translation,
            speaker: crate::types::transcript::Speaker {
                speaker_id: s.speaker.speaker_id,
                label: s.speaker.label,
            },
        }
    }
}
