use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use gemini_client_api::gemini::{
    ask::Gemini,
    types::{
        request::{FileData, InlineData, PartType, SystemInstruction},
        sessions::Session,
    },
    utils::GeminiSchema,
};
use mime::Mime;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};

use crate::{
    constants,
    gemini::common::Model,
    types::{
        transcript::ComplexTranscriptOutput,
        translate::{TranslationInput, TranslationOutput},
    },
};

// Files smaller than this threshold are sent as inline base64 data.
// Files at or above this threshold are uploaded via the resumable File API,
// which avoids the ~20 MiB base64 inline payload limit and prevents
// mid-stream "error decoding response body" failures on larger audio.
const FILE_API_THRESHOLD_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

const FILE_API_UPLOAD_URL: &str =
    "https://generativelanguage.googleapis.com/upload/v1beta/files";

enum GeminiCmd {
    TranscribeStream {
        bytes: Vec<u8>,
        mime_type: String,
        chunk_tx: mpsc::Sender<Result<String>>,
    },
    Translate {
        input: TranslationInput,
        reply: oneshot::Sender<Result<TranslationOutput>>,
    },
}

#[derive(Clone)]
pub struct GeminiClient {
    tx: mpsc::Sender<GeminiCmd>,
}

impl GeminiClient {
    pub fn new(api_keys: (String, String)) -> Self {
        let (tx, rx) = mpsc::channel(8);
        tokio::spawn(gemini_actor(api_keys, rx));
        Self { tx }
    }

    /// Returns an `mpsc::Receiver` that yields text chunks as they arrive from
    /// Gemini. The receiver closes when streaming is complete or on error
    /// (the error itself is delivered as the final `Err` item).
    ///
    /// The caller is responsible for accumulating all `Ok` chunks into a
    /// single buffer and deserializing the result into `ComplexTranscriptOutput`.
    pub async fn transcribe_audio_stream(
        &self,
        bytes: Vec<u8>,
        mime_type: impl Into<String>,
    ) -> Result<mpsc::Receiver<Result<String>>> {
        let (chunk_tx, chunk_rx) = mpsc::channel(64);
        self.tx
            .send(GeminiCmd::TranscribeStream {
                bytes,
                mime_type: mime_type.into(),
                chunk_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("GeminiClient actor is gone"))?;
        Ok(chunk_rx)
    }

    pub async fn translate(&self, input: TranslationInput) -> Result<TranslationOutput> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(GeminiCmd::Translate { input, reply })
            .await
            .map_err(|_| anyhow::anyhow!("GeminiClient actor is gone"))?;

        rx.await
            .map_err(|_| anyhow::anyhow!("GeminiClient actor dropped reply sender"))?
    }
}

async fn gemini_actor(api_keys: (String, String), mut rx: mpsc::Receiver<GeminiCmd>) {
    // Build a shared reqwest client with a generous timeout for large uploads and
    // long-running generation streams. The default 60 s timeout in gemini-client-api
    // is too short for multi-minute audio files.
    let http = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .expect("failed to build reqwest client");

    let transcriber = Gemini::new_with_client(
        &api_keys.0,
        Model::TranscriptionModel.as_str(),
        Some(SystemInstruction::from(
            constants::TRANSCRIPTION_SYSTEM_PROMPT,
        )),
        http.clone(),
    )
    .set_json_mode(ComplexTranscriptOutput::gemini_schema());

    let translator = Gemini::new(
        &api_keys.1,
        Model::TranslationModel.as_str(),
        Some(SystemInstruction::from(
            constants::TRANSLATION_SYSTEM_PROMPT,
        )),
    )
    .set_json_mode(TranslationOutput::gemini_schema());

    while let Some(cmd) = rx.recv().await {
        match cmd {
            GeminiCmd::TranscribeStream {
                bytes,
                mime_type,
                chunk_tx,
            } => {
                let transcriber = transcriber.clone();
                let api_key = api_keys.0.clone();
                let http = http.clone();
                tokio::spawn(async move {
                    do_transcribe_stream(&transcriber, &http, &api_key, bytes, &mime_type, chunk_tx)
                        .await;
                });
            }
            GeminiCmd::Translate { input, reply } => {
                let result = do_translate(&translator, input).await;
                let _ = reply.send(result);
            }
        }
    }
}

// =========================================================================
// File API upload (resumable protocol)
// =========================================================================

#[derive(Deserialize, Debug)]
struct FileApiResponse {
    file: FileApiFile,
}

#[derive(Deserialize, Debug)]
struct FileApiFile {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    name: String,
}

/// Uploads `bytes` to the Gemini File API using the two-step resumable protocol:
///
/// 1. POST to initiate — returns an `X-Goog-Upload-URL` header.
/// 2. PUT the raw bytes to that URL to finalise — returns the file metadata JSON.
///
/// Returns the `file_uri` and confirmed `mime_type` on success.
async fn upload_via_file_api(
    http: &HttpClient,
    api_key: &str,
    bytes: &[u8],
    mime_type: &str,
) -> Result<FileApiFile> {
    let num_bytes = bytes.len();

    tracing::debug!(
        bytes = num_bytes,
        mime_type = %mime_type,
        "initiating resumable File API upload"
    );

    // Step 1: initiate upload, collect the resumable upload URL from headers.
    let init_resp = http
        .post(format!("{FILE_API_UPLOAD_URL}?key={api_key}"))
        .header("X-Goog-Upload-Protocol", "resumable")
        .header("X-Goog-Upload-Command", "start")
        .header("X-Goog-Upload-Header-Content-Length", num_bytes.to_string())
        .header("X-Goog-Upload-Header-Content-Type", mime_type)
        .header("Content-Type", "application/json")
        .body(r#"{"file":{"display_name":"audio"}}"#)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("File API init request failed: {e}"))?;

    let status = init_resp.status();
    tracing::debug!(status = %status, "File API init response");

    let upload_url = init_resp
        .headers()
        .get("x-goog-upload-url")
        .ok_or_else(|| anyhow::anyhow!("File API init response missing x-goog-upload-url header (status {status})"))?
        .to_str()
        .map_err(|e| anyhow::anyhow!("x-goog-upload-url header is not valid UTF-8: {e}"))?
        .to_owned();

    tracing::debug!(upload_url = %upload_url, "received resumable upload URL");

    // Step 2: upload the raw bytes and finalise.
    let finalize_resp = http
        .put(&upload_url)
        .header("Content-Length", num_bytes.to_string())
        .header("X-Goog-Upload-Offset", "0")
        .header("X-Goog-Upload-Command", "upload, finalize")
        .body(bytes.to_vec())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("File API upload request failed: {e}"))?;

    let finalize_status = finalize_resp.status();
    let body = finalize_resp
        .text()
        .await
        .map_err(|e| anyhow::anyhow!("failed to read File API finalize response body: {e}"))?;

    tracing::debug!(status = %finalize_status, body = %body, "File API finalize response");

    if !finalize_status.is_success() {
        return Err(anyhow::anyhow!(
            "File API finalize failed (HTTP {finalize_status}): {body}"
        ));
    }

    let parsed: FileApiResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("failed to parse File API response: {e}\nbody: {body}"))?;

    tracing::info!(
        file_name = %parsed.file.name,
        file_uri = %parsed.file.uri,
        "File API upload complete"
    );

    Ok(parsed.file)
}

// =========================================================================
// Transcription streaming
// =========================================================================

async fn do_transcribe_stream(
    ai: &Gemini,
    http: &HttpClient,
    api_key: &str,
    bytes: Vec<u8>,
    mime_type: &str,
    chunk_tx: mpsc::Sender<Result<String>>,
) {
    let num_bytes = bytes.len();
    tracing::info!(
        bytes = num_bytes,
        mime_type = %mime_type,
        threshold = FILE_API_THRESHOLD_BYTES,
        use_file_api = num_bytes >= FILE_API_THRESHOLD_BYTES,
        "starting Gemini transcription stream"
    );

    let mime = match Mime::from_str(mime_type) {
        Ok(m) => m,
        Err(e) => {
            let _ = chunk_tx
                .send(Err(anyhow::anyhow!("invalid mime type: {e}")))
                .await;
            return;
        }
    };

    // Build the session part: inline base64 for small files, File API URI for large ones.
    // The File API avoids the ~20 MiB inline payload limit and keeps HTTP request sizes
    // manageable, reducing the chance of mid-stream body decode failures.
    let mut session = Session::new(2);

    if num_bytes >= FILE_API_THRESHOLD_BYTES {
        let file = match upload_via_file_api(http, api_key, &bytes, mime_type).await {
            Ok(f) => f,
            Err(e) => {
                tracing::error!(error = %e, "File API upload failed");
                let _ = chunk_tx
                    .send(Err(anyhow::anyhow!("File API upload failed: {e}")))
                    .await;
                return;
            }
        };
        // FileData::new(mime_type: Option<String>, file_uri: String)
        let file_data = FileData::new(Some(file.mime_type), file.uri);
        session.ask(PartType::FileData(file_data));
    } else {
        tracing::debug!(bytes = num_bytes, "encoding audio as inline base64");
        let b64 = STANDARD.encode(&bytes);
        let inline = InlineData::new(mime, b64);
        session.ask(inline);
    }

    let mut stream = match ai.ask_as_stream(session).await {
        Ok(s) => s,
        Err((_session, e)) => {
            tracing::error!(error = %e, "Gemini stream init failed");
            let _ = chunk_tx
                .send(Err(anyhow::anyhow!("Gemini stream init failed: {e}")))
                .await;
            return;
        }
    };

    tracing::debug!("Gemini stream opened, reading chunks");
    let mut chunk_count = 0usize;
    let mut total_text_bytes = 0usize;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(response) => {
                let text = response.get_chat().get_text_no_think("");
                total_text_bytes += text.len();
                tracing::debug!(
                    chunk = chunk_count,
                    text_len = text.len(),
                    total_text_bytes,
                    "Gemini chunk received"
                );
                chunk_count += 1;
                if !text.is_empty() {
                    if chunk_tx.send(Ok(text)).await.is_err() {
                        tracing::warn!(chunk = chunk_count, "chunk receiver dropped, aborting stream");
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    chunks_received = chunk_count,
                    total_text_bytes,
                    "Gemini stream chunk error"
                );
                let _ = chunk_tx
                    .send(Err(anyhow::anyhow!("Gemini stream chunk error: {e}")))
                    .await;
                break;
            }
        }
    }

    tracing::info!(
        chunks = chunk_count,
        total_text_bytes,
        "Gemini transcription stream complete"
    );
    // Dropping chunk_tx here closes the channel, signalling EOF to the receiver.
}

async fn do_translate(ai: &Gemini, input: TranslationInput) -> Result<TranslationOutput> {
    let mut session = Session::new(2);
    let serialized = serde_json::to_string(&input)
        .map_err(|_| anyhow::anyhow!("Failed to serialize TranslationInput"))?;
    session.ask(serialized);
    let reply = ai.ask(&mut session).await?;

    Ok(reply.get_json()?)
}
