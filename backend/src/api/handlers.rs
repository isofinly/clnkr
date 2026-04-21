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
    db::core,
    streaming::try_extract_next_segment,
    types::{
        common::{CachedTranscriptionResponse, CachedTranslationResponse},
        transcript::{ComplexTranscriptOutput, TranscribeQuery},
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
///
/// Accepts a multipart upload with a single `audio` field, saves the file to
/// disk, derives an audio signature (SHA-256 of raw bytes), checks the cache,
/// and either returns the cached transcript or streams a live Gemini
/// transcription over SSE.
///
/// SSE event sequence:
/// - `event: status`  — emitted immediately to confirm the connection is alive.
/// - `event: chunk`   — zero or more raw JSON text chunks from Gemini.
/// - `event: complete`— the fully assembled and deserialized transcript JSON.
/// - `event: error`   — emitted instead of `complete` if something goes wrong.
pub(super) async fn transcribe_stream(
    State(state): State<Arc<AppState>>,
    Extension(user_id): Extension<String>,
    Query(params): Query<TranscribeQuery>,
    mut multipart: Multipart,
) -> Response {
    let (audio_bytes, filename) = match extract_audio_field(&mut multipart).await {
        Ok(v) => v,
        Err(msg) => return utils::sse_error_response(msg),
    };

    let audio_signature = utils::hex_sha256(&audio_bytes);
    let transcript_type = if params.transcript_words {
        "complex"
    } else {
        // TODO: wire SimpleTranscriptOutput path when needed
        "complex"
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

    tracing::info!(bytes = audio_bytes.len(), "sending audio inline to Gemini");

    let mut chunk_rx = match state
        .gemini
        .transcribe_audio_as_stream(audio_bytes, "audio/mpeg")
        .await
    {
        Ok(rx) => rx,
        Err(e) => {
            return utils::sse_error_response(format!("Failed to start transcription: {e}"));
        }
    };

    let pool = state.pool.clone();
    let stored_filename = filename.clone();

    let sse_stream = async_stream::stream! {
        yield Ok::<Event, std::convert::Infallible>(
            Event::default()
                .event("status")
                .data("{\"message\":\"processing\"}"),
        );

        let mut buffer = String::new();
        let mut stream_error: Option<String> = None;
        // Cursor into `buffer` past which all complete segments have already
        // been extracted and emitted, so we never re-scan processed text.
        let mut scan_cursor: usize = 0;
        let mut segments_emitted: usize = 0;

        while let Some(result) = chunk_rx.recv().await {
            match result {
                Ok(text) => {
                    // The sentinel is injected by the Gemini client actor when
                    // it detects a 503 and activates the OpenRouter fallback.
                    // We surface it as a dedicated SSE event so the frontend can
                    // show an informational banner without treating it as a JSON
                    // fragment or an error.
                    if text == "__FALLBACK_OPENROUTER__" {
                        yield Ok(Event::default()
                            .event("fallback")
                            .data("{\"provider\":\"openrouter\"}"));
                        continue;
                    }

                    buffer.push_str(&text);

                    // Extract every newly completable segment from the
                    // accumulated buffer and emit each one immediately so the
                    // frontend can render it without waiting for the full JSON.
                    while let Some((seg, next)) =
                        try_extract_next_segment(&buffer, scan_cursor)
                    {
                        scan_cursor = next;
                        segments_emitted += 1;
                        tracing::debug!(id = seg.id, segments_emitted, "emitting streamed segment");
                        if let Ok(event) =
                            Event::default().event("segment").json_data(&seg)
                        {
                            yield Ok(event);
                        }
                    }
                }
                Err(e) => {
                    stream_error = Some(format!("{e}"));
                    break;
                }
            }
        }

        tracing::info!(
            segments_emitted,
            buffer_len = buffer.len(),
            scan_cursor,
            "Gemini stream consumed"
        );

        // Diagnostic: dump a sample around the "segments" key so we can see
        // exactly what Gemini produced when no segments were extracted.
        if segments_emitted == 0 {
            if let Some(pos) = buffer.find("\"segments\"") {
                let start = pos.saturating_sub(20);
                let end = (pos + 500).min(buffer.len());
                tracing::warn!(
                    segments_region = %&buffer[start..end],
                    "zero segments extracted — buffer around \"segments\" key"
                );
            } else {
                tracing::warn!(
                    buffer_head = %&buffer[..buffer.len().min(300)],
                    "zero segments extracted — no \"segments\" key found in buffer"
                );
            }
        }

        if let Some(err) = stream_error {
            let payload = json!({ "message": err }).to_string();
            yield Ok(Event::default().event("error").data(payload));
            return;
        }


        let transcript: ComplexTranscriptOutput = match serde_json::from_str(&buffer) {
            Ok(t) => t,
            Err(e) => {
                let payload = json!({ "message": format!("parse error: {e}") }).to_string();
                yield Ok(Event::default().event("error").data(payload));
                return;
            }
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

    // Derive a stable cache key from the serialized input (excludes segment_id
    // which is UI metadata, not part of the translation content).
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

    let output = state.gemini.translate(input.clone()).await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            TranslationErrorResponse {
                message: format!("Gemini error: {e}"),
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

type TranscriptRequest = axum::extract::Multipart;

async fn extract_audio_field(input: &mut TranscriptRequest) -> Result<(Content, Filename), String> {
    while let Some(field) = input.next_field().await.map_err(|e| e.to_string())? {
        if field.name() == Some("audio") {
            let filename = field.file_name().map(str::to_owned);
            let bytes = field.bytes().await.map_err(|e| e.to_string())?;
            return Ok((bytes.to_vec(), filename));
        }
    }
    Err("missing `audio` field in multipart body".into())
}
