use std::sync::Arc;

use axum::{
    Json,
    extract::{Extension, Multipart, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    api::{app::AppState, utils},
    audio,
    db::core,
    llm::common::Model,
    transcribe,
    types::{
        common::{CachedTranscriptionResponse, CachedTranslationResponse},
        transcript::{ComplexSegment, ComplexTranscriptOutput, Speaker},
        translate::{
            TranslateQuery, TranslationErrorResponse, TranslationInput, TranslationInputRequest,
            TranslationOutputResponse,
        },
    },
};

pub(super) async fn health_check() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

/// `POST /api/v1/transcriptions/stream`
pub(super) async fn transcribe_stream(
    State(state): State<Arc<AppState>>,
    Extension(_user_id): Extension<String>,
    Query(params): Query<crate::types::transcript::TranscribeQuery>,
    mut multipart: Multipart,
) -> Response {
    let (audio_bytes, filename, mime_type) = match extract_audio_field(&mut multipart).await {
        Ok(v) => v,
        Err(msg) => return utils::sse_error_response(msg),
    };

    let audio_signature = utils::hex_sha256(&audio_bytes);
    let transcript_type = if params.transcript_words {
        "complex"
    } else {
        "complex" // TODO: wire SimpleTranscriptOutput path when needed
    };

    if !params.force {
        if let Ok(Some(cached)) =
            core::get_cached_transcription(&state.pool, &audio_signature, transcript_type).await
        {
            let stream = tokio_stream::iter(vec![
                Ok::<Event, std::convert::Infallible>(
                    Event::default()
                        .event("status")
                        .data("{\"message\":\"served from cache\"}"),
                ),
                Ok(Event::default()
                    .event("complete")
                    .json_data(&cached)
                    .unwrap()),
            ]);
            return Sse::new(stream)
                .keep_alive(axum::response::sse::KeepAlive::default())
                .into_response();
        }
    }

    // Chunk the audio via ffmpeg.
    let chunks = match audio::chunker::chunk_audio(&audio_bytes, &mime_type).await {
        Ok(c) => c,
        Err(e) => {
            return utils::sse_error_response(format!("Audio chunking failed: {e}"));
        }
    };

    tracing::info!(
        chunks = chunks.len(),
        total_bytes = audio_bytes.len(),
        "audio chunked for transcription"
    );

    let pool = state.pool.clone();
    let llm = state.llm.clone();
    let stored_filename = filename.clone();

    let sse_stream = async_stream::stream! {
        let total = chunks.len();
        yield Ok::<Event, std::convert::Infallible>(
            Event::default()
                .event("status")
                .data(format!(
                    "{{\"message\":\"processing {} chunks\",\"total_chunks\":{}}}",
                    total, total
                )),
        );

        let mut all_segments: Vec<ComplexSegment> = Vec::new();
        let mut all_speakers: Vec<Speaker> = Vec::new();
        let mut chunk_summaries: Vec<String> = Vec::new();
        let mut next_id_offset: u64 = 0;
        let mut total_duration: f64 = 0.0;

        for (chunk_idx, chunk) in chunks.iter().enumerate() {
            tracing::info!(
                chunk = chunk_idx,
                start = chunk.start_seconds,
                end = chunk.end_seconds,
                bytes = chunk.bytes.len(),
                "starting transcription for chunk"
            );

            let system_prompt = transcribe::chunked::build_system_prompt(chunk_idx, &chunk_summaries);
            let model = Model::TranscriptionModel;

            yield Ok(Event::default()
                .event("status")
                .data(format!(
                    "{{\"message\":\"transcribing chunk {}/{}\",\"chunk_index\":{}}}",
                    chunk_idx + 1,
                    total,
                    chunk_idx
                )));

            let transcript_result = match llm.transcribe_chunk(
                chunk.bytes.clone(),
                "audio/mpeg", // ffmpeg converts every chunk to MP3
                &system_prompt,
                model,
            ).await {
                Ok(t) => t,
                Err(e) => {
                    let payload = json!({ "message": format!("Chunk {chunk_idx} transcription failed: {e}") }).to_string();
                    yield Ok(Event::default().event("error").data(payload));
                    return;
                }
            };

            // Shift timestamps and IDs to global space.
            let mut chunk_segments = transcript_result.segments;
            for seg in &mut chunk_segments {
                seg.start_seconds += chunk.start_seconds;
                seg.end_seconds += chunk.start_seconds;
                seg.id += next_id_offset;
            }
            if let Some(max_id) = chunk_segments.iter().map(|s| s.id).max() {
                next_id_offset = max_id + 1;
            }

            tracing::info!(
                chunk = chunk_idx,
                extracted = chunk_segments.len(),
                "chunk transcription complete"
            );

            // -----------------------------------------------------------------
            // Word-fill guard: back-fill empty `words` arrays via flash-lite.
            // -----------------------------------------------------------------
            let any_empty = chunk_segments.iter().any(|s| s.words.is_empty());
            if any_empty {
                yield Ok(Event::default()
                    .event("status")
                    .data(format!(
                        "{{\"message\":\"filling word annotations {}/{}\",\"chunk_index\":{}}}",
                        chunk_idx + 1,
                        total,
                        chunk_idx
                    )));

                match llm.fill_words(&chunk_segments).await {
                    Ok(filled) => {
                        let filled_count = filled.len();
                        if filled_count > 0 {
                            for seg in &mut chunk_segments {
                                if let Some(words) = filled.get(&seg.id) {
                                    seg.words.clone_from(words);
                                }
                            }
                            tracing::info!(
                                chunk = chunk_idx,
                                filled = filled_count,
                                "word-fill guard applied"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, chunk = chunk_idx, "word-fill guard failed");
                    }
                }
            }

            // Generate summary of this chunk (flash-lite).
            yield Ok(Event::default()
                .event("status")
                .data(format!(
                    "{{\"message\":\"summarizing chunk {}/{}\",\"chunk_index\":{}}}",
                    chunk_idx + 1,
                    total,
                    chunk_idx
                )));

            let summary = match llm.generate_summary(&chunk_segments, &all_speakers).await {
                Ok(s) => {
                    tracing::info!(chunk = chunk_idx, summary_len = s.len(), "summary generated");
                    s
                }
                Err(e) => {
                    tracing::warn!(error = %e, chunk = chunk_idx, "summary generation failed");
                    String::new()
                }
            };
            chunk_summaries.push(summary);

            // Stitch with previous chunk if applicable.
            if chunk_idx > 0 && !chunk_segments.is_empty() {
                yield Ok(Event::default()
                    .event("status")
                    .data(format!(
                        "{{\"message\":\"stitching boundary {}/{}\",\"chunk_index\":{}}}",
                        chunk_idx,
                        total,
                        chunk_idx
                    )));

                let prev_tail = transcribe::chunked::extract_tail(&all_segments, 5);
                let next_head = transcribe::chunked::extract_head(&chunk_segments, 5);

                if !prev_tail.is_empty() && !next_head.is_empty() {
                    match llm.stitch_overlap(&prev_tail, &next_head).await {
                        Ok(overlap) => {
                            let discard_count = all_segments
                                .iter()
                                .rev()
                                .take_while(|s| overlap.replace_segment_ids.contains(&s.id))
                                .count();
                            let cut = all_segments.len().saturating_sub(discard_count);
                            tracing::info!(
                                chunk = chunk_idx,
                                replace_ids = ?overlap.replace_segment_ids,
                                new_boundary = overlap.new_segments.len(),
                                truncated = discard_count,
                                "stitch applied"
                            );
                            all_segments.truncate(cut);

                            // The model was already given global timestamps,
                            // so its returned boundary segments are global too.
                            for stitched in overlap.new_segments {
                                let seg: ComplexSegment = stitched.into();
                                all_segments.push(seg);
                            }

                            // Append the remainder of chunk N that lies *after*
                            // the known 15-second overlap window.  Using the
                            // fixed overlap bound is more reliable than whatever
                            // the model hallucinates for overlap_end_seconds.
                            let overlap_end = chunk.start_seconds
                                + crate::audio::chunker::CHUNK_OVERLAP_SECONDS;
                            for seg in &chunk_segments {
                                if seg.start_seconds >= overlap_end {
                                    all_segments.push(seg.clone());
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, chunk = chunk_idx, "stitching failed, appending raw");
                            all_segments.extend(chunk_segments);
                        }
                    }
                } else {
                    all_segments.extend(chunk_segments);
                }
            } else {
                all_segments.extend(chunk_segments);
            }

            // Renumber globally, reconcile speaker IDs by label, and rebuild
            // the canonical speaker list.
            transcribe::chunked::renumber_segments(&mut all_segments);
            all_speakers = transcribe::chunked::reconcile_speakers(&mut all_segments);
            total_duration = chunk.end_seconds;

            tracing::info!(
                chunk = chunk_idx,
                total_segments = all_segments.len(),
                total_duration = total_duration,
                speakers = all_speakers.len(),
                "chunk complete"
            );

            // Persist chunk to DB (best-effort).
            let segs_val = serde_json::to_value(&all_segments).unwrap_or(Value::Null);
            let spk_val = serde_json::to_value(&all_speakers).unwrap_or(Value::Null);
            let _ = core::upsert_chunk(
                &pool,
                &audio_signature,
                chunk_idx as i32,
                chunk.start_seconds,
                chunk.end_seconds,
                &segs_val,
                &spk_val,
                chunk_summaries.last().map(|s| s.as_str()),
                chunk_idx > 0,
            ).await;
        }

        yield Ok(Event::default()
            .event("status")
            .data("{\"message\":\"finalizing transcript\"}"));

        tracing::info!(
            total_segments = all_segments.len(),
            total_duration = total_duration,
            speakers = all_speakers.len(),
            "transcription complete, emitting final transcript"
        );

        let transcript = ComplexTranscriptOutput {
            source_language: "ja".to_string(),
            target_language: "en".to_string(),
            total_duration_seconds: total_duration,
            speakers: all_speakers.clone(),
            segments: all_segments,
        };

        let response_val = serde_json::to_value(&transcript).unwrap_or(Value::Null);
        let _ = core::upsert_transcription(
            &pool,
            &audio_signature,
            transcript_type,
            &response_val,
            stored_filename.as_deref(),
        )
        .await;

        if let Ok(event) = Event::default().event("complete").json_data(&transcript) {
            yield Ok(event);
        }
    };

    Sse::new(sse_stream)
        .keep_alive(axum::response::sse::KeepAlive::default())
        .into_response()
}

/// `POST /api/v1/translations`
pub(super) async fn translate(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    Query(params): Query<TranslateQuery>,
    Json(req): Json<TranslationInputRequest>,
) -> Result<TranslationOutputResponse, (StatusCode, TranslationErrorResponse)> {
    let input = TranslationInput {
        translation_input: req.translation_input.clone(),
        context: req.context.clone(),
    };

    let input_hash =
        utils::hex_sha256(serde_json::to_string(&input).unwrap_or_default().as_bytes());

    if !params.force {
        if let Ok(Some(cached)) =
            core::get_cached_translation(&state.pool, &user_id, &input_hash).await
        {
            return Ok(TranslationOutputResponse {
                served_from_cache: true,
                input_hash: input_hash.clone(),
                translation: cached,
            });
        }
    }

    let output = state
        .llm
        .translate(
            input.clone(),
            crate::constants::TRANSLATION_SYSTEM_PROMPT,
            Model::TranslationModel,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                TranslationErrorResponse {
                    message: format!("LLM error: {e}"),
                },
            )
        })?;

    let _ = core::upsert_translation(
        &state.pool,
        &user_id,
        &input_hash,
        &input,
        &output,
        false,
        None,
        Some(req.segment_id as i64),
    )
    .await;

    Ok(TranslationOutputResponse {
        served_from_cache: false,
        input_hash,
        translation: output,
    })
}

#[derive(Deserialize)]
pub(super) struct NoteBody {
    note_text: String,
}

#[derive(Deserialize)]
pub(super) struct RenameBody {
    file_name: String,
}

/// `GET /api/v1/notes/:input_hash`
pub(super) async fn get_note(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(input_hash): axum::extract::Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let note = core::get_note(&state.pool, &user_id, &input_hash)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;

    match note {
        Some(text) => Ok(Json(json!({ "note_text": text }))),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "message": "not found" })),
        )),
    }
}

/// `PUT /api/v1/notes/:input_hash`
pub(super) async fn upsert_note(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(input_hash): axum::extract::Path<String>,
    Json(body): Json<NoteBody>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    core::upsert_note(&state.pool, &user_id, &input_hash, &body.note_text)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/v1/notes/:input_hash`
pub(super) async fn delete_note(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    axum::extract::Path(input_hash): axum::extract::Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    core::delete_note(&state.pool, &user_id, &input_hash)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub(super) struct OffsetQuery {
    #[serde(default)]
    pub offset: i64,
}

/// `GET /api/v1/transcriptions?offset=`
pub(super) async fn all_transcriptions(
    State(state): State<Arc<AppState>>,
    Extension(_user_id): Extension<String>,
    Query(params): Query<OffsetQuery>,
) -> Result<CachedTranscriptionResponse, (StatusCode, Json<Value>)> {
    let data = core::get_cached_transcriptions(&state.pool, params.offset)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;

    let total_transcriptions_count =
        core::get_transcriptions_count(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": format!("{e}") })),
                )
            })?;
    Ok(CachedTranscriptionResponse {
        transcriptions: data,
        total_translations: total_transcriptions_count,
    })
}

/// `PATCH /api/v1/transcriptions/:audio_signature/rename`
pub(super) async fn rename_transcription(
    State(state): State<Arc<AppState>>,
    Extension(_user_id): Extension<String>,
    axum::extract::Path(audio_signature): axum::extract::Path<String>,
    Json(body): Json<RenameBody>,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    core::rename_transcription(&state.pool, &audio_signature, "complex", &body.file_name)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/user/translations?offset=`
pub(super) async fn user_translations(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    Query(params): Query<OffsetQuery>,
) -> Result<CachedTranslationResponse, (StatusCode, Json<Value>)> {
    let data = core::get_cached_translations(&state.pool, &user_id, params.offset)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "message": format!("{e}") })),
            )
        })?;

    let total_transcriptions_count =
        core::get_translations_count(&state.pool)
            .await
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "message": format!("{e}") })),
                )
            })?;

    Ok(CachedTranslationResponse {
        translations: data,
        total_transcriptions: total_transcriptions_count,
    })
}

type Content = Vec<u8>;
type Filename = Option<String>;
type MimeType = String;

async fn extract_audio_field(
    input: &mut Multipart,
) -> Result<(Content, Filename, MimeType), String> {
    while let Some(field) = input.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("audio") {
            let filename = field.file_name().map(str::to_owned);
            let mime_type = field
                .content_type()
                .map(|m| m.to_string())
                .unwrap_or_else(|| "audio/mpeg".to_string());
            let bytes = field.bytes().await.map_err(|e| e.to_string())?;
            return Ok((bytes.to_vec(), filename, mime_type));
        }
    }
    Err("missing `audio` field in multipart body".into())
}
