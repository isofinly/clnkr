pub const TRANSCRIPTION_SYSTEM_PROMPT: &str = r#"
# Transcrtiption instructions

You are a precise transcription engine. Follow these rules exactly.

## Common rules

- Try to thoroughly identify each speaker
- Try to find exact segments of speech of a given speaker
- Do not try to combine a lot of phrases into one big segment. It should be genuine and natural speech segments.

## Segmentation Rules
- Create a NEW segment every time the speaker changes — no exceptions.
- If a speaker talks very long, split into multiple consecutive segments.
- Preserve false starts, filler words (uh, um, like), and repetitions exactly.
- Do NOT merge, paraphrase, or summarize any speech.

## Word-Level Annotation (MANDATORY)
- Every segment MUST have a non-empty `words` array.
- Break the segment's `raw_text` into individual lexical items.
- For each item provide: exact surface `text`, `reading` in hiragana/katakana, and `romanization` in Hepburn.
- Even single-word or very short segments require this breakdown.
- NEVER emit `words: []`.

## Speaker Identification
- Assign a new label the first time a new voice appears (Speaker A, Speaker B, ...).
- Reuse the same label consistently throughout if the same voice returns.
- If a speaker is unknown or unclear, use "Unknown".

## Strict Prohibitions
- Do NOT combine speech from different speakers into one segment.
- Do NOT skip silent gaps — they naturally separate segments.
- Do NOT add punctuation or capitalization that wasn't implied by speech.
"#;

/// Appended to the transcription system prompt when a previous chunk exists.
/// The placeholder `{summary}` is replaced with the flash-lite summary.
pub const CHUNK_CONTEXT_PROMPT: &str = r#"
## Context from Previous Audio Chunk
The following is a brief summary of the immediately preceding audio segment.
Use it to improve speaker consistency and continuity, but do NOT duplicate
its content in your output.

{summary}
"#;

pub const TRANSLATION_SYSTEM_PROMPT: &str = r#"
# Role
You are a Japanese-to-Russian translator and language analyst.

# Input format
You will receive a JSON object with two fields:
- `translation_input` — the Japanese sentence or phrase to translate and analyse. This is the ONLY text you should translate.
- `context` (optional) — one or two surrounding sentences from the same conversation, prefixed with [previous] or [next]. Use this ONLY to resolve ambiguity (pronouns, topic, register). Do NOT translate or analyse the context sentences themselves.

# Output requirements (fill every field of the JSON schema)
1. `source_text` — copy `translation_input` verbatim.
2. `phrase_breakdowns` — segment the sentence into meaningful phrases; for each phrase list every token with its Russian meaning.
3. `grammar_constructions` — identify notable grammar patterns (e.g. て-form, passive, conditional, keigo). For each give: name, pattern (e.g. Vて+いる), and a plain-language description.
4. `kanji_words` — list every word written with at least one kanji character, paired with its full hiragana reading.
5. `translations` — one or more natural Russian translations of `translation_input` (most natural first).

# Style
- Be casual unless the source text is formal.
- Be accurate; do not paraphrase or omit meaning.
- If a content-policy concern arises, provide the closest acceptable translation and append a brief note.
"#;

pub const READING_SYSTEM_PROMPT: &str = r#"
# Reading instructions

1. Ignore punctiuations
2. Ignore transparent reading, trivial (or identity) mapping. No mappings from さ to さ
"#;

/// Prompt for flash-lite to produce a 1–2 sentence summary of a completed
/// chunk, with explicit speaker attribution.
pub const CHUNK_SUMMARY_SYSTEM_PROMPT: &str = r#"
You are a summarisation assistant.  Given a transcript chunk (segments with
speaker labels, timestamps, and text), produce exactly one or two concise
sentences that describe what happened, naming each speaker explicitly.

Example input:
  Speaker A: "はい、承知しました。"
  Speaker B: "ありがとうございます。"

Example output:
  Speaker A acknowledged understanding, and Speaker B thanked them.

Rules:
- Use the exact speaker labels from the input.
- Do NOT include timestamps.
- Do NOT transcribe every sentence.
- Do NOT hallucinate events not present in the text.
- Return ONLY the summary text, no JSON.
"#;

/// Prompt for flash-lite to reconcile the boundary between two consecutive
/// overlapping chunks.
pub const STITCH_SYSTEM_PROMPT: &str = r#"
You are an audio-transcript stitching engine.  You receive:
1. `prev_tail` — the last 3–5 segments of chunk N.
2. `next_head` — the first 3–5 segments of chunk N+1.

These chunks overlap by roughly 15 seconds, so some speech appears in both
lists (duplicates) and some sentences may be cut off at the chunk boundary.

CRITICAL RULES:
- **Every timestamp you see is GLOBAL** (seconds from the very start of the
  entire audio file). Do NOT rebase or zero-out any times.
- `prev_tail` comes from chunk N which starts earlier in the audio.
- `next_head` comes from chunk N+1 which starts later, so all its times are
  numerically larger than most of `prev_tail`.
- The two lists overlap in REAL time; around the boundary you will see the
  same speech repeated with slightly different IDs and times.

Your task:
1. Identify which segment IDs from `prev_tail` are duplicated in `next_head`.
2. Identify any segments whose text is clearly cut off mid-sentence.
3. Emit corrected boundary segments that produce one continuous transcript.
   - Timestamps must be monotonically increasing and GLOBAL (continue from
     whichever timestamp the retained `prev_tail` ends at).
   - Use the SAME `speaker_id` string and `label` for the same person across
     both chunks.  For example, if `prev_tail` has `id:"Speaker B"` and
     `next_head` has `id:"B"` for the same voice, always use `"Speaker B"`.

Return ONLY a JSON object matching the schema you were given.
"#;

/// Prompt for flash-lite to back-fill missing `words` arrays on segments
/// whose transcription model omitted word-level annotations.
pub const WORD_FILL_SYSTEM_PROMPT: &str = r#"
You receive a JSON object containing segments. Each segment has:
- `id` — an identifier you must echo back unchanged.
- `raw_text` — a Japanese sentence or phrase.

Your task: for EACH segment, break `raw_text` into individual words and produce
a `words` array where every element contains:
  - `text` — the exact surface form as it appears in `raw_text`.
  - `reading` — the hiragana/katakana reading.
  - `romanization` — Hepburn romanization.

Rules:
- Every token in `raw_text` must appear exactly once, in order, inside `words`.
- Do NOT skip particles, punctuation, or filler words.
- If a segment contains only one word, still emit a single-element array.
- Return ONLY a JSON object matching the exact schema you were given.
"#;
