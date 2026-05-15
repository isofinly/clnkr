use crate::types::{
    common::{CachedTranscription, CachedTranslation},
    transcript::{ComplexTranscriptOutput, SimpleTranscriptOutput},
    translate::{TranslationInput, TranslationOutput},
};
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_cached_transcription(
    pool: &PgPool,
    audio_signature: &str,
    transcript_type: &str,
) -> Result<Option<serde_json::Value>> {
    let row = sqlx::query_scalar!(
        "SELECT response_json FROM transcriptions WHERE audio_signature = $1 AND transcript_type = $2",
        audio_signature,
        transcript_type
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn get_cached_complex_transcription(
    pool: &PgPool,
    audio_signature: &str,
) -> Result<Option<ComplexTranscriptOutput>> {
    let row = get_cached_transcription(pool, audio_signature, "complex").await?;
    match row {
        Some(v) => Ok(Some(serde_json::from_value(v)?)),
        None => Ok(None),
    }
}

pub async fn get_cached_simple_transcription(
    pool: &PgPool,
    audio_signature: &str,
) -> Result<Option<SimpleTranscriptOutput>> {
    let row = get_cached_transcription(pool, audio_signature, "simple").await?;
    match row {
        Some(v) => Ok(Some(serde_json::from_value(v)?)),
        None => Ok(None),
    }
}

pub async fn upsert_transcription(
    pool: &PgPool,
    audio_signature: &str,
    transcript_type: &str,
    response_json: &serde_json::Value,
    file_name: Option<&str>,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO transcriptions (audio_signature, transcript_type, response_json, file_name)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (audio_signature, transcript_type)
        DO UPDATE SET response_json = EXCLUDED.response_json, updated_at = NOW(),
            file_name = COALESCE(EXCLUDED.file_name, transcriptions.file_name)
        "#,
        audio_signature,
        transcript_type,
        response_json,
        file_name
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_transcription(
    pool: &PgPool,
    audio_signature: &str,
    transcript_type: &str,
) -> Result<()> {
    sqlx::query!(
        "DELETE FROM transcriptions WHERE audio_signature = $1 AND transcript_type = $2",
        audio_signature,
        transcript_type
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn rename_transcription(
    pool: &PgPool,
    audio_signature: &str,
    transcript_type: &str,
    file_name: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE transcriptions SET file_name = $3 WHERE audio_signature = $1 AND transcript_type = $2",
        audio_signature,
        transcript_type,
        file_name
    )
    .execute(pool)
    .await?;
    Ok(())
}


pub async fn get_cached_translation(
    pool: &PgPool,
    user_id: &str,
    input_hash: &str,
) -> Result<Option<TranslationOutput>> {
    let row = sqlx::query_scalar!(
        "SELECT response_json FROM translations WHERE user_id = $1 AND input_hash = $2",
        user_id,
        input_hash
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(v) => Ok(Some(serde_json::from_value(v)?)),
        None => Ok(None),
    }
}

pub async fn upsert_translation(
    pool: &PgPool,
    user_id: &str,
    input_hash: &str,
    input_json: &TranslationInput,
    response_json: &TranslationOutput,
    served_from_cache: bool,
    origin_audio_sig: Option<&str>,
    origin_segment_id: Option<i64>,
) -> Result<Uuid> {
    let input_val = serde_json::to_value(input_json)?;
    let response_val = serde_json::to_value(response_json)?;

    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO translations
            (user_id, input_hash, input_json, response_json, served_from_cache, origin_audio_sig, origin_segment_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (user_id, input_hash)
        DO UPDATE SET
            response_json     = EXCLUDED.response_json,
            served_from_cache = EXCLUDED.served_from_cache,
            origin_audio_sig  = EXCLUDED.origin_audio_sig,
            origin_segment_id = EXCLUDED.origin_segment_id
        RETURNING id
        "#,
        user_id,
        input_hash,
        input_val,
        response_val,
        served_from_cache,
        origin_audio_sig,
        origin_segment_id
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}

pub async fn get_note(pool: &PgPool, user_id: &str, input_hash: &str) -> Result<Option<String>> {
    let row = sqlx::query_scalar!(
        "SELECT note_text FROM notes WHERE user_id = $1 AND input_hash = $2",
        user_id,
        input_hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(row)
}

pub async fn upsert_note(
    pool: &PgPool,
    user_id: &str,
    input_hash: &str,
    note_text: &str,
) -> Result<Uuid> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO notes (user_id, input_hash, note_text)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, input_hash)
        DO UPDATE SET note_text = EXCLUDED.note_text, updated_at = NOW()
        RETURNING id
        "#,
        user_id,
        input_hash,
        note_text
    )
    .fetch_one(pool)
    .await?;

    Ok(id)
}

pub async fn delete_note(pool: &PgPool, user_id: &str, input_hash: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM notes WHERE user_id = $1 AND input_hash = $2",
        user_id,
        input_hash
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn log_request(
    pool: &PgPool,
    user_id: &str,
    request_type: &str,
    request_meta: Option<&serde_json::Value>,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO request_log (user_id, request_type, request_meta) VALUES ($1, $2, $3)",
        user_id,
        request_type,
        request_meta
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Returns 50 entries of cached translations starting from the given offset.
pub async fn get_cached_translations(
    pool: &PgPool,
    user_id: &str,
    offset: i64,
) -> Result<Vec<CachedTranslation>> {
    let rows = sqlx::query!(
        "SELECT response_json, input_hash FROM translations WHERE user_id = $1 LIMIT 50 OFFSET $2",
        user_id,
        offset
    )
    .fetch_all(pool)
    .await?;

    let result: Vec<CachedTranslation> = rows
        .into_iter()
        .map(|r| CachedTranslation {
            response_json: r.response_json,
            input_hash: r.input_hash,
        })
        .collect();

    Ok(result)
}

/// Returns 25 entries of cached transcriptions starting from the given offset.
pub async fn get_cached_transcriptions(
    pool: &PgPool,
    offset: i64,
) -> Result<Vec<CachedTranscription>> {
    let rows = sqlx::query!(
        "SELECT response_json, audio_signature, transcript_type, file_name FROM transcriptions LIMIT 25 OFFSET $1",
        offset
    )
    .fetch_all(pool)
    .await?;

    let result: Vec<CachedTranscription> = rows
        .into_iter()
        .map(|r| CachedTranscription {
            response_json: r.response_json,
            audio_signature: r.audio_signature,
            transcript_type: r.transcript_type,
            file_name: r.file_name,
        })
        .collect();

    Ok(result)
}

pub async fn get_transcriptions_count(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query!("SELECT COUNT(1) FROM transcriptions")
        .fetch_one(pool)
        .await?;
    Ok(count.count.unwrap_or(0))
}

pub async fn get_translations_count(pool: &PgPool) -> Result<i64> {
    let count = sqlx::query!("SELECT COUNT(1) FROM translations")
        .fetch_one(pool)
        .await?;
    Ok(count.count.unwrap_or(0))
}

// ---------------------------------------------------------------------------
// Chunk-level storage
// ---------------------------------------------------------------------------

pub async fn upsert_chunk(
    pool: &PgPool,
    audio_signature: &str,
    chunk_index: i32,
    start_seconds: f64,
    end_seconds: f64,
    segments_json: &serde_json::Value,
    speakers_json: &serde_json::Value,
    summary: Option<&str>,
    stitched: bool,
) -> Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO transcription_chunks
            (audio_signature, chunk_index, start_seconds, end_seconds,
             segments_json, speakers_json, summary, stitched)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (audio_signature, chunk_index)
        DO UPDATE SET
            start_seconds = EXCLUDED.start_seconds,
            end_seconds   = EXCLUDED.end_seconds,
            segments_json = EXCLUDED.segments_json,
            speakers_json = EXCLUDED.speakers_json,
            summary       = EXCLUDED.summary,
            stitched      = EXCLUDED.stitched,
            updated_at    = NOW()
        "#,
        audio_signature,
        chunk_index,
        start_seconds,
        end_seconds,
        segments_json,
        speakers_json,
        summary,
        stitched
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_chunks(
    pool: &PgPool,
    audio_signature: &str,
) -> Result<Vec<serde_json::Value>> {
    let rows = sqlx::query_scalar!(
        "SELECT segments_json FROM transcription_chunks WHERE audio_signature = $1 ORDER BY chunk_index",
        audio_signature
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn delete_chunks(pool: &PgPool, audio_signature: &str) -> Result<()> {
    sqlx::query!(
        "DELETE FROM transcription_chunks WHERE audio_signature = $1",
        audio_signature
    )
    .execute(pool)
    .await?;
    Ok(())
}
