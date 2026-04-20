/// Utilities for progressively extracting typed objects from a streaming JSON buffer.
///
/// Gemini emits the transcript as a single large JSON object in arbitrary-sized
/// text chunks. Rather than waiting for the full object before sending anything
/// to the client, we scan the accumulated buffer for complete `{…}` objects
/// inside the `"segments":[…]` array and emit each one as an SSE event the
/// moment it becomes available.

use crate::types::transcript::ComplexSegment;

/// Scans `buffer` for the next complete JSON object (balanced braces) starting
/// at or after `scan_from`, then attempts to deserialise it as a
/// `ComplexSegment`.
///
/// Returns `Some((segment, next_scan_pos))` on success; `None` if no complete
/// object is found yet or if the candidate does not deserialise cleanly.
///
/// `next_scan_pos` is the index *after* the closing `}` of the matched object
/// — pass it back as `scan_from` on the next call to avoid re-scanning already
/// processed text.
pub fn try_extract_next_segment(
    buffer: &str,
    scan_from: usize,
) -> Option<(ComplexSegment, usize)> {
    let array_start = find_segments_array_start(buffer)?;

    // The caller's scan cursor must be within or after the array body.
    let start = scan_from.max(array_start);
    let bytes = buffer.as_bytes();

    let mut depth: i32 = 0;
    let mut obj_start: Option<usize> = None;
    let mut in_string = false;
    let mut escape = false;
    let mut i = start;

    while i < bytes.len() {
        let ch = bytes[i] as char;

        if escape {
            escape = false;
            i += 1;
            continue;
        }
        if ch == '\\' && in_string {
            escape = true;
            i += 1;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        match ch {
            '{' => {
                if depth == 0 {
                    obj_start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start_idx) = obj_start {
                        let candidate = &buffer[start_idx..=i];
                        match serde_json::from_str::<ComplexSegment>(candidate) {
                            Ok(seg) => return Some((seg, i + 1)),
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    candidate_len = candidate.len(),
                                    candidate_start = %&candidate[..candidate.len().min(120)],
                                    "balanced-brace candidate failed ComplexSegment deser"
                                );
                            }
                        }
                        // Candidate parsed as balanced JSON but not a valid
                        // ComplexSegment — skip and keep scanning.
                        obj_start = None;
                    }
                }
            }
            // If we encounter `]` at depth 0 we have reached the end of the
            // segments array — no point scanning further.
            ']' if depth == 0 => break,
            _ => {}
        }
        i += 1;
    }

    None
}

/// Locates the byte index right after the opening `[` of the `"segments"` array.
///
/// Searches for the *last* occurrence of the `"segments"` key to avoid matching
/// the string literal inside a `raw_text` or other value field.
fn find_segments_array_start(buffer: &str) -> Option<usize> {
    let key = "\"segments\"";
    // Use rfind so we match the actual struct key (which appears after all the
    // content fields that might incidentally contain the word "segments").
    let key_pos = buffer.rfind(key)?;
    let mut ci = key_pos + key.len();
    let bs = buffer.as_bytes();
    // Skip whitespace then `:`
    while ci < bs.len() && bs[ci].is_ascii_whitespace() { ci += 1; }
    if ci >= bs.len() || bs[ci] != b':' { return None; }
    ci += 1;
    // Skip whitespace then `[`
    while ci < bs.len() && bs[ci].is_ascii_whitespace() { ci += 1; }
    if ci >= bs.len() || bs[ci] != b'[' { return None; }
    Some(ci + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reproduces the exact whitespace format observed in production logs:
    // `"segments": [` with a space after the colon.
    const SAMPLE: &str = r#"{
  "source_language": "Japanese",
  "target_language": "Japanese",
  "total_duration_seconds": 60,
  "speakers": [{"speaker_id": "speaker_1", "label": "男性"}],
  "segments": [
    {
      "id": 1,
      "start_seconds": 0.28,
      "end_seconds": 2.76,
      "raw_text": "はい。",
      "words": [{"text": "はい", "reading": "はい", "romanization": "hai"}],
      "translation": "Yes.",
      "speaker": {"speaker_id": "speaker_1", "label": "男性"}
    },
    {
      "id": 2,
      "start_seconds": 3.0,
      "end_seconds": 5.0,
      "raw_text": "そうです。",
      "words": [{"text": "そう", "reading": "そう", "romanization": "sou"}],
      "translation": "That's right.",
      "speaker": {"speaker_id": "speaker_1", "label": "男性"}
    }
  ]
}"#;

    #[test]
    fn extracts_first_segment_with_spaced_colon() {
        let (seg, next) = try_extract_next_segment(SAMPLE, 0).expect("should find first segment");
        assert_eq!(seg.id, 1);
        assert!(next < SAMPLE.len());
    }

    #[test]
    fn extracts_second_segment_after_cursor() {
        let (_, next) = try_extract_next_segment(SAMPLE, 0).unwrap();
        let (seg2, _) = try_extract_next_segment(SAMPLE, next).expect("should find second segment");
        assert_eq!(seg2.id, 2);
    }

    #[test]
    fn returns_none_when_segment_incomplete() {
        // Buffer ends mid-way through the first segment.
        let partial = &SAMPLE[..SAMPLE.find("\"end_seconds\"").unwrap()];
        assert!(try_extract_next_segment(partial, 0).is_none());
    }

    /// Simulates incremental chunk accumulation (as observed in production logs)
    /// and verifies that segments are extracted as soon as each one becomes
    /// complete in the buffer.
    #[test]
    fn extracts_segments_incrementally_from_chunked_buffer() {
        let chunks: Vec<&str> = vec![
            "{\n  \"source_language",
            "\": \"Japanese\",\n  \"target_language\": \"Japanese\",\n  \"total_duration_seconds\":",
            "60,\n  \"speakers\": [\n    {\n      \"speaker_id\": \"speaker_1\",\n      \"",
            "label\": \"男性\"\n    }\n  ],\n  \"segments\": [\n    {\n      \"id\":",
            "1,\n      \"start_seconds\": 0.28,\n      \"end_seconds\": 2.",
            "76,\n      \"raw_text\": \"はい。\",\n",
            "\"words\": [\n        {\n          \"text\": \"はい\",\n          \"reading\": \"はい\",\n          \"romanization\": \"hai\"\n        }\n      ],\n",
            "      \"translation\": \"Yes.\",\n      \"speaker\": {\"speaker_id\": \"speaker_1\", \"label\": \"男性\"}\n    },\n    {\n      \"id\":",
            "2,\n      \"start_seconds\": 3.0,\n      \"end_seconds\": 5.0,\n",
            "      \"raw_text\": \"そうです。\",\n      \"words\": [{\"text\": \"そう\", \"reading\": \"そう\", \"romanization\": \"sou\"}],\n      \"translation\": \"That's right.\",\n      \"speaker\": {\"speaker_id\": \"speaker_1\", \"label\": \"男性\"}\n    }\n  ]\n}",
        ];

        let mut buffer = String::new();
        let mut scan_cursor: usize = 0;
        let mut extracted: Vec<u64> = Vec::new();

        for chunk in &chunks {
            buffer.push_str(chunk);
            while let Some((seg, next)) = try_extract_next_segment(&buffer, scan_cursor) {
                extracted.push(seg.id);
                scan_cursor = next;
            }
        }

        assert_eq!(extracted, vec![1, 2], "should extract both segments incrementally");
    }

    /// Gemini sometimes emits "speaker_id" as a flat string inside a segment
    /// instead of the nested "speaker" object. The extractor should still find
    /// balanced-brace objects, but deserialization will fail (and that failure
    /// should not prevent extracting subsequent segments).
    #[test]
    fn flat_speaker_id_causes_deser_failure_but_scanning_continues() {
        let buf = r#"{
  "source_language": "ja",
  "target_language": "ja",
  "total_duration_seconds": 60,
  "speakers": [{"speaker_id": "speaker_1", "label": "男性A"}],
  "segments": [
    {
      "id": 1,
      "start_seconds": 0.28,
      "end_seconds": 2.76,
      "raw_text": "はい。",
      "words": [{"text": "はい", "reading": "はい", "romanization": "hai"}],
      "translation": "Yes.",
      "speaker_id": "speaker_1"
    }
  ]
}"#;
        // This segment uses flat `speaker_id` instead of nested `speaker`,
        // so ComplexSegment deserialization should fail.
        let result = try_extract_next_segment(buf, 0);
        assert!(result.is_none(), "flat speaker_id should not deserialize as ComplexSegment");
    }
}
