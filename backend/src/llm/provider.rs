use crate::{
    constants,
    llm::{common::Model, gemini, openrouter},
    types::{
        overlap::OverlapResult,
        reading::{WordFillInput, WordFillOutput, WordFillSegment},
        transcript::{ComplexSegment, ComplexTranscriptOutput, Speaker, TranscriptWord},
        translate::{TranslationInput, TranslationOutput},
    },
};
use anyhow::Result;
use gemini_client_api::gemini::utils::GeminiSchema;
use reqwest::Client as HttpClient;
use std::time::Duration;
use tokio::sync::mpsc;

/// Unified LLM client that tries both AIStudio Gemini keys before falling
/// back to OpenRouter. Applies to both streaming (transcription) and
/// request/response (translation, summary, stitching) operations.
#[derive(Clone)]
pub struct UnifiedModelClient {
    http: HttpClient,
    gemini_keys: (String, String),
    openrouter_key: Option<String>,
}

impl UnifiedModelClient {
    pub fn new(gemini_keys: (String, String), openrouter_key: Option<String>) -> Self {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build reqwest client");
        Self {
            http,
            gemini_keys,
            openrouter_key,
        }
    }

    /// Transcribe a single audio chunk non-streaming, collecting the full JSON
    /// response and retrying transparently across Gemini keys and OpenRouter.
    pub async fn transcribe_chunk(
        &self,
        bytes: Vec<u8>,
        mime_type: &str,
        system_prompt: &str,
        model: Model,
    ) -> Result<ComplexTranscriptOutput> {
        let model_str = model.as_str();
        let schema = Some(ComplexTranscriptOutput::gemini_schema());

        // Try Gemini key 1
        match self
            .try_transcribe_with_key(&self.gemini_keys.0, &bytes, mime_type, system_prompt, &schema, model_str)
            .await
        {
            Ok(t) => return Ok(t),
            Err(e) if is_retryable_gemini(&e) => {
                tracing::warn!(error = %e, "Gemini key 1 failed, trying key 2");
            }
            Err(e) => return Err(e),
        }

        // Try Gemini key 2
        match self
            .try_transcribe_with_key(&self.gemini_keys.1, &bytes, mime_type, system_prompt, &schema, model_str)
            .await
        {
            Ok(t) => return Ok(t),
            Err(e) if is_retryable_gemini(&e) => {
                tracing::warn!(error = %e, "Gemini key 2 failed, trying OpenRouter");
            }
            Err(e) => return Err(e),
        }

        // Fallback to OpenRouter
        if let Some(ref key) = self.openrouter_key {
            match self
                .try_transcribe_with_openrouter(key, &bytes, mime_type, system_prompt, &schema, model_str)
                .await
            {
                Ok(t) => {
                    tracing::info!("Falling back to OpenRouter for transcription chunk");
                    return Ok(t);
                }
                Err(e) => return Err(e),
            }
        }

        anyhow::bail!("All LLM providers failed for transcription chunk")
    }

    async fn try_transcribe_with_key(
        &self,
        api_key: &str,
        bytes: &[u8],
        mime_type: &str,
        system_prompt: &str,
        schema: &Option<serde_json::Value>,
        model: &str,
    ) -> Result<ComplexTranscriptOutput> {
        let rx = gemini::stream(
            &self.http,
            api_key,
            model,
            system_prompt,
            schema.clone(),
            bytes.to_vec(),
            mime_type,
        )
        .await?;
        let text = collect_stream(rx).await?;
        Ok(serde_json::from_str(&text)?)
    }

    async fn try_transcribe_with_openrouter(
        &self,
        api_key: &str,
        bytes: &[u8],
        mime_type: &str,
        system_prompt: &str,
        schema: &Option<serde_json::Value>,
        model: &str,
    ) -> Result<ComplexTranscriptOutput> {
        let rx = openrouter::stream(
            &self.http,
            api_key,
            model,
            system_prompt,
            schema.clone(),
            bytes,
            mime_type,
        )
        .await?;
        let text = collect_stream(rx).await?;
        Ok(serde_json::from_str(&text)?)
    }

    // -----------------------------------------------------------------------
    // Translation
    // -----------------------------------------------------------------------

    pub async fn translate(
        &self,
        input: TranslationInput,
        system_prompt: &str,
        model: Model,
    ) -> Result<TranslationOutput> {
        let model_str = model.as_str();
        let user_content = serde_json::to_string(&input)?;

        match gemini::ask_json(
            &self.gemini_keys.0,
            model_str,
            system_prompt,
            user_content.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {
                tracing::warn!(error = %e, "Gemini key 1 translation failed, trying key 2");
            }
            Err(e) => return Err(e),
        }

        match gemini::ask_json(
            &self.gemini_keys.1,
            model_str,
            system_prompt,
            user_content.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {
                tracing::warn!(error = %e, "Gemini key 2 translation failed, trying OpenRouter");
            }
            Err(e) => return Err(e),
        }

        if let Some(ref key) = self.openrouter_key {
            let schema = Some(TranslationOutput::gemini_schema());
            match openrouter::ask_json(
                &self.http,
                key,
                model_str,
                system_prompt,
                schema,
                user_content,
            )
            .await
            {
                Ok(r) => {
                    tracing::info!("Falling back to OpenRouter for translation");
                    return Ok(r);
                }
                Err(e) => return Err(e),
            }
        }

        anyhow::bail!("All LLM providers failed for translation")
    }

    // -----------------------------------------------------------------------
    // Summary generation (flash-lite)
    // -----------------------------------------------------------------------

    pub async fn generate_summary(
        &self,
        segments: &[ComplexSegment],
        _speakers: &[Speaker],
    ) -> Result<String> {
        let prompt = format_summary_prompt(segments);
        let model_str = Model::SummaryModel.as_str();

        match gemini::ask_text(
            &self.gemini_keys.0,
            model_str,
            constants::CHUNK_SUMMARY_SYSTEM_PROMPT,
            prompt.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {}
            Err(e) => return Err(e),
        }

        match gemini::ask_text(
            &self.gemini_keys.1,
            model_str,
            constants::CHUNK_SUMMARY_SYSTEM_PROMPT,
            prompt.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {}
            Err(e) => return Err(e),
        }

        if let Some(ref key) = self.openrouter_key {
            match openrouter::ask_text(
                &self.http,
                key,
                model_str,
                constants::CHUNK_SUMMARY_SYSTEM_PROMPT,
                prompt,
            )
            .await
            {
                Ok(r) => return Ok(r),
                Err(e) => return Err(e),
            }
        }

        anyhow::bail!("All LLM providers failed for summary generation")
    }

    // -----------------------------------------------------------------------
    // Stitching (flash-lite)
    // -----------------------------------------------------------------------

    pub async fn stitch_overlap(
        &self,
        prev_segments: &[ComplexSegment],
        next_segments: &[ComplexSegment],
    ) -> Result<OverlapResult> {
        let prompt = format_stitch_prompt(prev_segments, next_segments);
        let model_str = Model::StitchModel.as_str();

        match gemini::ask_json(
            &self.gemini_keys.0,
            model_str,
            constants::STITCH_SYSTEM_PROMPT,
            prompt.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {}
            Err(e) => return Err(e),
        }

        match gemini::ask_json(
            &self.gemini_keys.1,
            model_str,
            constants::STITCH_SYSTEM_PROMPT,
            prompt.clone(),
        )
        .await
        {
            Ok(r) => return Ok(r),
            Err(e) if is_retryable_gemini(&e) => {}
            Err(e) => return Err(e),
        }

        if let Some(ref key) = self.openrouter_key {
            let schema = Some(OverlapResult::gemini_schema());
            match openrouter::ask_json(
                &self.http,
                key,
                model_str,
                constants::STITCH_SYSTEM_PROMPT,
                schema,
                prompt,
            )
            .await
            {
                Ok(r) => return Ok(r),
                Err(e) => return Err(e),
            }
        }

        anyhow::bail!("All LLM providers failed for stitching")
    }

    // -----------------------------------------------------------------------
    // Word-fill guard (flash-lite)
    // -----------------------------------------------------------------------

    /// Fills empty `words` arrays for segments whose transcription model
    /// skipped the word-level breakdown.
    ///
    /// Returns a map `segment_id -> words` for every segment that needed a
    /// back-fill. If all segments already have words, returns an empty map.
    pub async fn fill_words(
        &self,
        segments: &[ComplexSegment],
    ) -> Result<std::collections::HashMap<u64, Vec<TranscriptWord>>> {
        let missing: Vec<&ComplexSegment> = segments
            .iter()
            .filter(|s| s.words.is_empty())
            .collect();
        if missing.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let payload = WordFillInput {
            segments: missing
                .iter()
                .map(|s| WordFillSegment {
                    id: s.id,
                    raw_text: s.raw_text.clone(),
                })
                .collect(),
        };
        let prompt = serde_json::to_string(&payload)?;
        let model_str = Model::SummaryModel.as_str(); // flash-lite

        let output: WordFillOutput = match gemini::ask_json(
            &self.gemini_keys.0,
            model_str,
            constants::WORD_FILL_SYSTEM_PROMPT,
            prompt.clone(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) if is_retryable_gemini(&e) => {
                tracing::warn!(error = %e, "word-fill key 1 failed, trying key 2");
                gemini::ask_json(
                    &self.gemini_keys.1,
                    model_str,
                    constants::WORD_FILL_SYSTEM_PROMPT,
                    prompt.clone(),
                )
                .await
                .map_err(|e2| {
                    tracing::warn!(error = %e2, "word-fill key 2 failed");
                    e2
                })?
            }
            Err(e) => return Err(e),
        };

        let mut result = std::collections::HashMap::new();
        for seg in output.segments {
            if !seg.words.is_empty() {
                result.insert(seg.id, seg.words);
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn is_retryable_gemini(err: &anyhow::Error) -> bool {
    use gemini_client_api::gemini::error::{GeminiResponseError, Status};

    // String-based fallback for transient errors where we are unsure
    // of the exact Status variant name (e.g. 503 wrapped in a stream body).
    let msg = err.to_string().to_lowercase();
    if msg.contains("429")
        || msg.contains("rate limit")
        || msg.contains("resource_exhausted")
        || msg.contains("503")
        || msg.contains("unavailable")
        || msg.contains("high demand")
    {
        return true;
    }

    if let Some(ge) = err.downcast_ref::<GeminiResponseError>() {
        match ge {
            GeminiResponseError::StatusNotOk(s) => {
                matches!(s.error.status, Status::Unavailable)
            }
            _ => false,
        }
    } else {
        false
    }
}

fn format_summary_prompt(segments: &[ComplexSegment]) -> String {
    let mut lines = vec!["Transcript segments:".to_string()];
    for seg in segments {
        lines.push(format!("  {}: \"{}\"", seg.speaker.label, seg.raw_text));
    }
    lines.join("\n")
}

fn format_stitch_prompt(prev: &[ComplexSegment], next: &[ComplexSegment]) -> String {
    let mut lines = vec![
        "prev_tail (chunk N — GLOBAL timestamps):".to_string(),
    ];
    for seg in prev {
        lines.push(format!(
            "  {{ \"id\": {}, \"start\": {:.2}, \"end\": {:.2}, \"speaker_id\": \"{}\", \"label\": \"{}\", \"text\": \"{}\" }}",
            seg.id, seg.start_seconds, seg.end_seconds, seg.speaker.speaker_id, seg.speaker.label, seg.raw_text
        ));
    }
    lines.push("next_head (chunk N+1 — GLOBAL timestamps):".to_string());
    for seg in next {
        lines.push(format!(
            "  {{ \"id\": {}, \"start\": {:.2}, \"end\": {:.2}, \"speaker_id\": \"{}\", \"label\": \"{}\", \"text\": \"{}\" }}",
            seg.id, seg.start_seconds, seg.end_seconds, seg.speaker.speaker_id, seg.speaker.label, seg.raw_text
        ));
    }
    lines.join("\n")
}

/// Accumulate every chunk from an LLM text stream into a single string.
/// If the stream delivers an error, that error is propagated immediately
/// so the caller can decide whether to retry.
async fn collect_stream(mut rx: mpsc::Receiver<Result<String>>) -> Result<String> {
    let mut buffer = String::new();
    while let Some(result) = rx.recv().await {
        match result {
            Ok(text) => buffer.push_str(&text),
            Err(e) => return Err(e),
        }
    }
    Ok(buffer)
}
