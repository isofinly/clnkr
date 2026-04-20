use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use gemini_client_api::gemini::{
    ask::Gemini,
    types::{
        request::{InlineData, SystemInstruction},
        sessions::Session,
    },
    utils::GeminiSchema,
};
use mime::Mime;
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
    let transcriber = Gemini::new(
        &api_keys.0,
        Model::TranscriptionModel.as_str(),
        Some(SystemInstruction::from(
            constants::TRANSCRIPTION_SYSTEM_PROMPT,
        )),
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
                tokio::spawn(async move {
                    do_transcribe_stream(&transcriber, bytes, &mime_type, chunk_tx).await;
                });
            }
            GeminiCmd::Translate { input, reply } => {
                let result = do_translate(&translator, input).await;
                let _ = reply.send(result);
            }
        }
    }
}

async fn do_transcribe_stream(
    ai: &Gemini,
    bytes: Vec<u8>,
    mime_type: &str,
    chunk_tx: mpsc::Sender<Result<String>>,
) {
    tracing::info!(bytes = bytes.len(), mime_type = %mime_type, "starting Gemini transcription stream (inline)");
    let mime = match Mime::from_str(mime_type) {
        Ok(m) => m,
        Err(e) => {
            let _ = chunk_tx
                .send(Err(anyhow::anyhow!("invalid mime type: {e}")))
                .await;
            return;
        }
    };
    let b64 = STANDARD.encode(&bytes);
    let inline = InlineData::new(mime, b64);
    let mut session = Session::new(2);
    session.ask(inline);

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
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(response) => {
                let text = response.get_chat().get_text_no_think("");
                tracing::debug!(chunk = chunk_count, text_len = text.len(), text = %text, "Gemini chunk");
                chunk_count += 1;
                if !text.is_empty() {
                    if chunk_tx.send(Ok(text)).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Gemini stream chunk error");
                let _ = chunk_tx
                    .send(Err(anyhow::anyhow!("Gemini stream chunk error: {e}")))
                    .await;
                break;
            }
        }
    }
    tracing::info!(chunks = chunk_count, "Gemini transcription stream complete");
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
