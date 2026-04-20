pub const TRANSCRIPTION_SYSTEM_PROMPT: &str = r#"
# Transcrtiption instructions

1. Try to thoroughly identify each speaker
2. Try to find exact segments of speech of a given speaker
3. Do not try to combine a lot of phrases into one big segment. It should be genuine and natural speech segments.
"#;

pub const TRANSLATION_SYSTEM_PROMPT: &str = r#"
# Translation instructions

For given `translation_input` in japanese strictly follow these guidelines:
- Be accurate and thorough
- If your content policy is an issue, provide the closest acceptable response and explain the content policy issue afterward
- Be casual unless otherwise specified
- Act as japanese translator into russian
- Give the parsing of every word
- Give exact grammar constructions with explanation
- Make list of all words written in kanji and furigana reading
- Translate to russian
- Do not translate `context` field. It is only for additional context and your better understanding.
"#;

pub const READING_SYSTEM_PROMPT: &str = r#"
# Reading instructions

1. Ignore punctiuations
2. Ignore transparent reading, trivial (or identity) mapping. No mappings from さ to さ
"#;
