use anyhow::{Context, Result};
use tokio::process::Command;

/// Target chunk length in seconds.
pub const CHUNK_TARGET_SECONDS: f64 = 90.0;

/// Overlap between consecutive chunks in seconds, used when stitching together chunks.
pub const CHUNK_OVERLAP_SECONDS: f64 = 15.0;

/// A single slice of the original audio ready for an LLM call.
pub struct AudioChunk {
    pub index: usize,
    pub bytes: Vec<u8>,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// Splits `input_bytes` into overlapping chunks using ffmpeg.
///
/// The flow is:
/// 1. Write input to a temp file (ffprobe/ffmpeg need a file path).
/// 2. ffprobe → total duration.
/// 3. For each chunk: ffmpeg -ss start -t duration … pipe:1.
pub async fn chunk_audio(input_bytes: &[u8], mime_type: &str) -> Result<Vec<AudioChunk>> {
    let ext = extension_from_mime(mime_type);

    let tmp = std::env::temp_dir().join(format!("audio_{}.{})", uuid::Uuid::new_v4(), ext));
    tokio::fs::write(&tmp, input_bytes)
        .await
        .context("failed to write temp audio file for ffmpeg")?;

    let duration = match ffprobe_duration(&tmp).await {
        Ok(d) => d,
        Err(e) => {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(e);
        }
    };
    if duration <= 0.0 {
        let _ = tokio::fs::remove_file(&tmp).await;
        anyhow::bail!("ffprobe returned non-positive duration ({duration})");
    }

    let mut chunks = Vec::new();
    let chunk_stride = CHUNK_TARGET_SECONDS - CHUNK_OVERLAP_SECONDS; // 125 s
    let mut cursor = 0.0;
    let mut idx = 0usize;

    while cursor < duration {
        let chunk_start = cursor;
        let chunk_end = (cursor + CHUNK_TARGET_SECONDS).min(duration);
        let chunk_dur = chunk_end - chunk_start;

        let bytes = match ffmpeg_extract_chunk(&tmp, chunk_start, chunk_dur).await {
            Ok(b) => b,
            Err(e) => {
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(e);
            }
        };

        chunks.push(AudioChunk {
            index: idx,
            bytes,
            start_seconds: chunk_start,
            end_seconds: chunk_end,
        });

        cursor += chunk_stride;
        idx += 1;
    }

    let _ = tokio::fs::remove_file(&tmp).await;
    Ok(chunks)
}

async fn ffprobe_duration(path: &std::path::Path) -> Result<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
            &path.to_string_lossy(),
        ])
        .output()
        .await
        .context("ffprobe command failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffprobe failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let dur: f64 = stdout
        .trim()
        .parse()
        .context("failed to parse ffprobe duration output")?;
    Ok(dur)
}

async fn ffmpeg_extract_chunk(
    input: &std::path::Path,
    start: f64,
    duration: f64,
) -> Result<Vec<u8>> {
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-ss",
        &format!("{start:.3}"),
        "-t",
        &format!("{duration:.3}"),
        "-i",
        &input.to_string_lossy(),
        "-f",
        "mp3",
        "-c:a",
        "libmp3lame",
        "-q:a",
        "4",
        "pipe:1",
    ]);

    let output = cmd
        .output()
        .await
        .context("ffmpeg chunk extraction command failed")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg failed: {stderr}");
    }

    Ok(output.stdout)
}

fn extension_from_mime(mime_type: &str) -> &'static str {
    match mime_type {
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/webm" => "webm",
        "audio/ogg" => "ogg",
        "audio/mp4" | "audio/x-m4a" => "m4a",
        _ => "bin",
    }
}
