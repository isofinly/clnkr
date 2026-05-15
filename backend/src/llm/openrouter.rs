use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use reqwest::Client as HttpClient;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::mpsc;

const OPENROUTER_CHAT_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

/// Streams a request via OpenRouter's OpenAI-compatible `/v1/chat/completions`
/// endpoint with `stream: true`.
///
/// Audio is always sent as inline base64.  The format field is derived from
/// the MIME type (e.g. `audio/webm` → `"webm"`).
pub async fn stream(
    http: &HttpClient,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    schema: Option<Value>,
    bytes: &[u8],
    mime_type: &str,
) -> Result<mpsc::Receiver<Result<String>>> {
    let format = mime_type
        .split('/')
        .nth(1)
        .unwrap_or("wav")
        .split(';')
        .next()
        .unwrap_or("wav")
        .to_owned();

    let b64 = STANDARD.encode(bytes);

    let content = vec![
        serde_json::json!({
            "type": "text",
            "text": system_prompt,
        }),
        serde_json::json!({
            "type": "input_audio",
            "input_audio": { "data": b64, "format": format }
        }),
    ];

    let mut body = serde_json::json!({
        "model": model,
        "stream": true,
        "messages": [
            { "role": "user", "content": content }
        ]
    });

    if let Some(s) = schema {
        body["response_format"] = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "Output",
                "strict": true,
                "schema": gemini_to_json_schema(s)
            }
        });
    }

    let resp = http
        .post(OPENROUTER_CHAT_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenRouter request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter returned HTTP {status}: {body_text}");
    }

    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        let mut byte_stream = resp.bytes_stream();
        let mut leftover = String::new();

        while let Some(chunk_result) = byte_stream.next().await {
            let raw = match chunk_result {
                Ok(b) => b,
                Err(e) => {
                    let _ = tx
                        .send(Err(anyhow::anyhow!("OpenRouter stream read error: {e}")))
                        .await;
                    return;
                }
            };

            leftover.push_str(&String::from_utf8_lossy(&raw));

            while let Some(newline_pos) = leftover.find('\n') {
                let line = leftover[..newline_pos].trim_end_matches('\r').to_owned();
                leftover = leftover[newline_pos + 1..].to_owned();

                let data = match line.strip_prefix("data: ") {
                    Some(d) => d,
                    None => continue,
                };

                if data == "[DONE]" {
                    return;
                }

                let text = serde_json::from_str::<Value>(data)
                    .ok()
                    .and_then(|v| {
                        v["choices"][0]["delta"]["content"]
                            .as_str()
                            .map(|s| s.to_owned())
                    })
                    .unwrap_or_default();

                if !text.is_empty() && tx.send(Ok(text)).await.is_err() {
                    return;
                }
            }
        }
    });

    Ok(rx)
}

// ---------------------------------------------------------------------------
// Non-streaming JSON
// ---------------------------------------------------------------------------

pub async fn ask_json<T>(
    http: &HttpClient,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    schema: Option<Value>,
    user_content: String,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ]
    });

    if let Some(s) = schema {
        body["response_format"] = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "Output",
                "strict": true,
                "schema": gemini_to_json_schema(s)
            }
        });
    }

    let resp = http
        .post(OPENROUTER_CHAT_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenRouter request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter returned HTTP {status}: {text}");
    }

    let json: Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OpenRouter response missing content"))?;

    Ok(serde_json::from_str(content)?)
}

// ---------------------------------------------------------------------------
// Non-streaming plain text
// ---------------------------------------------------------------------------

pub async fn ask_text(
    http: &HttpClient,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_content: String,
) -> Result<String> {
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ]
    });

    let resp = http
        .post(OPENROUTER_CHAT_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("OpenRouter request failed: {e}"))?;

    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("OpenRouter returned HTTP {status}: {text}");
    }

    let json: Value = resp.json().await?;
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("OpenRouter response missing content"))?;

    Ok(content.to_string())
}

// ---------------------------------------------------------------------------
// JSON-schema converter (Gemini → OpenAI)
// ---------------------------------------------------------------------------

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
