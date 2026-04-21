use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use reqwest::Client as HttpClient;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::{constants, llm::common::Model, types::transcript::ComplexTranscriptOutput};
use gemini_client_api::gemini::utils::GeminiSchema;

const OPENROUTER_CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Streams transcription via OpenRouter using the OpenAI-compatible
/// `/v1/chat/completions` endpoint with `stream: true`.
///
/// Audio is always sent as inline base64. The format field
/// is derived from the MIME type (e.g. `audio/webm` → `"webm"`).
pub(super) async fn transcribe_as_stream(
    http: &HttpClient,
    api_key: &str,
    bytes: &[u8],
    mime_type: &str,
    chunk_tx: &mpsc::Sender<Result<String>>,
) {
    let format = mime_type
        .split('/')
        .nth(1)
        .unwrap_or("wav")
        .split(';')
        .next()
        .unwrap_or("wav")
        .to_owned();

    let b64 = STANDARD.encode(bytes);

    let json_schema = gemini_to_json_schema(ComplexTranscriptOutput::gemini_schema());

    let body = serde_json::json!({
        "model": Model::TranscriptionModel.as_str(),
        "stream": true,
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "ComplexTranscriptOutput",
                "strict": true,
                "schema": json_schema
            }
        },
        "messages": [
            {
                "role": "user",
                "content": [
                    {
                        "type": "text",
                        "text": constants::TRANSCRIPTION_SYSTEM_PROMPT
                    },
                    {
                        "type": "input_audio",
                        "input_audio": {
                            "data": b64,
                            "format": format
                        }
                    }
                ]
            }
        ]
    });

    tracing::info!(
        mime_type = %mime_type,
        format = %format,
        model = Model::TranscriptionModel.as_str(),
        "starting OpenRouter fallback transcription stream"
    );

    let resp = match http
        .post(OPENROUTER_CHAT_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = chunk_tx
                .send(Err(anyhow::anyhow!("OpenRouter request failed: {e}")))
                .await;
            return;
        }
    };

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        let _ = chunk_tx
            .send(Err(anyhow::anyhow!(
                "OpenRouter returned HTTP {status}: {body_text}"
            )))
            .await;
        return;
    }

    // Parse the SSE stream. Each line that starts with "data: " carries a JSON
    // object with `choices[0].delta.content`.
    // The stream ends with "data: [DONE]".
    let mut byte_stream = resp.bytes_stream();
    let mut leftover = String::new();
    let mut chunk_count = 0usize;
    let mut total_text_bytes = 0usize;

    while let Some(chunk_result) = byte_stream.next().await {
        let raw = match chunk_result {
            Ok(b) => b,
            Err(e) => {
                let _ = chunk_tx
                    .send(Err(anyhow::anyhow!("OpenRouter stream read error: {e}")))
                    .await;
                return;
            }
        };

        // Append the new bytes to any partial line carried over from the previous chunk.
        leftover.push_str(&String::from_utf8_lossy(&raw));

        // Process all complete SSE lines.
        while let Some(newline_pos) = leftover.find('\n') {
            let line = leftover[..newline_pos].trim_end_matches('\r').to_owned();
            leftover = leftover[newline_pos + 1..].to_owned();

            let data = match line.strip_prefix("data: ") {
                Some(d) => d,
                None => continue, // comment line, event:, id:, or blank
            };

            if data == "[DONE]" {
                tracing::info!(
                    chunks = chunk_count,
                    total_text_bytes,
                    "OpenRouter transcription stream complete"
                );
                return;
            }

            // Extract the delta content from the SSE JSON payload.
            // The path is: choices[0].delta.content
            let text = serde_json::from_str::<serde_json::Value>(data)
                .ok()
                .and_then(|v| {
                    v["choices"][0]["delta"]["content"]
                        .as_str()
                        .map(|s| s.to_owned())
                })
                .unwrap_or_default();

            if !text.is_empty() {
                total_text_bytes += text.len();
                chunk_count += 1;
                tracing::debug!(
                    chunk = chunk_count,
                    text_len = text.len(),
                    "OpenRouter chunk received"
                );
                if chunk_tx.send(Ok(text)).await.is_err() {
                    tracing::warn!(
                        chunk = chunk_count,
                        "chunk receiver dropped, aborting OpenRouter stream"
                    );
                    return;
                }
            }
        }
    }

    tracing::info!(
        chunks = chunk_count,
        total_text_bytes,
        "OpenRouter transcription stream complete"
    );
}

fn gemini_to_json_schema(v: Value) -> Value {
    match v {
        Value::Object(mut map) => {
            if let Some(Value::String(t)) = map.get("type") {
                let lower = t.to_lowercase();
                map.insert("type".to_string(), Value::String(lower));
            }

            if let Some(Value::Object(props)) = map.remove("properties") {
                let required: Vec<Value> = props.keys().map(|k| Value::String(k.clone())).collect();
                let converted: serde_json::Map<String, Value> = props
                    .into_iter()
                    .map(|(k, val)| (k, gemini_to_json_schema(val)))
                    .collect();
                map.insert("properties".to_string(), Value::Object(converted));
                map.insert("required".to_string(), Value::Array(required));
                map.insert("additionalProperties".to_string(), Value::Bool(false));
            }

            if let Some(items) = map.remove("items") {
                map.insert("items".to_string(), gemini_to_json_schema(items));
            }

            map.remove("nullable");

            Value::Object(map)
        }
        other => other,
    }
}
