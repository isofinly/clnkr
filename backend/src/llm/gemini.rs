use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use futures::StreamExt;
use gemini_client_api::gemini::{
    ask::Gemini,
    error::GeminiResponseError,
    types::{
        request::{FileData, InlineData, PartType, SystemInstruction},
        sessions::Session,
    },
    utils::GeminiSchema,
};
use mime::Mime;
use reqwest::Client as HttpClient;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use tokio::sync::mpsc;

// Files smaller than this threshold are sent as inline base64 data.
// Files at or above this threshold are uploaded via the resumable File API,
// which avoids the ~20 MiB base64 inline payload limit.
const FILE_API_THRESHOLD_BYTES: usize = 10 * 1024 * 1024; // 10 MiB

const FILE_API_UPLOAD_URL: &str = "https://generativelanguage.googleapis.com/upload/v1beta/files";

/// Streams a request to Gemini and returns an `mpsc::Receiver` that yields
/// text chunks as they arrive. The receiver closes when streaming is complete
/// or on error (the error itself is delivered as the final `Err` item).
///
/// If `schema` is provided the session is configured for JSON mode.
pub async fn stream(
    http: &HttpClient,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    schema: Option<serde_json::Value>,
    bytes: Vec<u8>,
    mime_type: &str,
) -> Result<mpsc::Receiver<Result<String>>> {
    let mut ai = Gemini::new_with_client(
        api_key,
        model,
        Some(SystemInstruction::from(system_prompt)),
        http.clone(),
    );
    if let Some(s) = schema {
        ai = ai.set_json_mode(s);
    }

    let mut session = Session::new(2);

    if bytes.len() >= FILE_API_THRESHOLD_BYTES {
        let file = upload_via_file_api(http, api_key, &bytes, mime_type).await?;
        let file_data = FileData::new(Some(file.mime_type), file.uri);
        session.ask(PartType::FileData(file_data));
    } else {
        let mime = Mime::from_str(mime_type)?;
        let b64 = STANDARD.encode(&bytes);
        let inline = InlineData::new(mime, b64);
        session.ask(inline);
    }

    let mut stream = match ai.ask_as_stream(session).await {
        Ok(s) => s,
        Err((_sess, err)) => return Err(wrap_gemini_err(err)),
    };

    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(response) => {
                    let text = response.get_chat().get_text_no_think("");
                    if !text.is_empty() && tx.send(Ok(text)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(Err(anyhow::anyhow!("Gemini stream chunk error: {e}")))
                        .await;
                    break;
                }
            }
        }
    });

    Ok(rx)
}

/// Non-streaming JSON request.
pub async fn ask_json<T>(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_content: String,
) -> Result<T>
where
    T: GeminiSchema + DeserializeOwned,
{
    let ai = Gemini::new(api_key, model, Some(SystemInstruction::from(system_prompt)))
        .set_json_mode(T::gemini_schema());

    let mut session = Session::new(2);
    session.ask(user_content);
    let reply = ai.ask(&mut session).await.map_err(wrap_gemini_err)?;
    Ok(reply.get_json()?)
}

/// Non-streaming plain-text request.
pub async fn ask_text(
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_content: String,
) -> Result<String> {
    let ai = Gemini::new(api_key, model, Some(SystemInstruction::from(system_prompt)));

    let mut session = Session::new(2);
    session.ask(user_content);
    let reply = ai.ask(&mut session).await.map_err(wrap_gemini_err)?;
    Ok(reply.get_chat().get_text_no_think("").to_string())
}

fn wrap_gemini_err(err: GeminiResponseError) -> anyhow::Error {
    err.into()
}

#[derive(Deserialize, Debug)]
struct FileApiResponse {
    file: FileApiFile,
}

#[derive(Deserialize, Debug)]
struct FileApiFile {
    uri: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[allow(dead_code)]
    name: String,
}

async fn upload_via_file_api(
    http: &HttpClient,
    api_key: &str,
    bytes: &[u8],
    mime_type: &str,
) -> Result<FileApiFile> {
    let num_bytes = bytes.len();

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
    let upload_url = init_resp
        .headers()
        .get("x-goog-upload-url")
        .ok_or_else(|| {
            anyhow::anyhow!(
                "File API init response missing x-goog-upload-url header (status {status})"
            )
        })?
        .to_str()
        .map_err(|e| anyhow::anyhow!("x-goog-upload-url header is not valid UTF-8: {e}"))?
        .to_owned();

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

    if !finalize_status.is_success() {
        return Err(anyhow::anyhow!(
            "File API finalize failed (HTTP {finalize_status}): {body}"
        ));
    }

    let parsed: FileApiResponse = serde_json::from_str(&body)
        .map_err(|e| anyhow::anyhow!("failed to parse File API response: {e}\nbody: {body}"))?;

    Ok(parsed.file)
}
