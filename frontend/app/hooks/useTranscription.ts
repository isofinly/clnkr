"use client";
import { useState, useRef, useCallback } from "react";
import { Transcript, TranscriptFile, StreamStatus } from "../types";
import { API_BASE_URL } from "../constants";
import { parseSseBlock, normaliseTranscript } from "../lib/utils";
import { authHeaders, clearToken } from "../lib/auth";

export type ModelType = "gemini-flash" | "gemini-flash-lite";
export type WordsMode = "simple" | "words";

export type TranscribeOptions = {
  force?: boolean;
  wordsMode?: WordsMode;
};

export function useTranscription(onUnauthorized: () => void) {
  const [files, setFiles] = useState<TranscriptFile[]>([]);
  const [activeFileIndex, setActiveFileIndex] = useState(-1);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [streamStatus, setStreamStatus] = useState<StreamStatus>("idle");
  const [streamError, setStreamError] = useState("");
  const [totalChunks, setTotalChunks] = useState(0);
  const [currentChunk, setCurrentChunk] = useState(0);

  const loadMock = async (path: string) => {
    const res = await fetch(path);
    const raw: Transcript = await res.json();
    const transcript = normaliseTranscript(raw);
    const name = path.split("/").pop() ?? path;
    setFiles((prev) => [...prev, { name, transcript }]);
    setActiveFileIndex((prev) => (prev === -1 ? 0 : prev));
  };

  const transcribeAudio = async (audioFile: File, options: TranscribeOptions = {}) => {
    const { force = false, wordsMode = "words" } = options;
    const displayName = audioFile.name.replace(/\.[^.]+$/, "") || "transcript";
    const transcriptWords = wordsMode === "words";

    const placeholder: TranscriptFile = {
      name: `${displayName} [streaming]`,
      wordsMode: transcriptWords,
      transcript: {
        source_language: "",
        target_language: "en",
        total_duration_seconds: 0,
        speakers: [],
        segments: [],
      },
    };

    setStreamError("");
    setStreamStatus("uploading");
    setTotalChunks(0);
    setCurrentChunk(0);
    setIsTranscribing(true);

    const newIndex = files.length;
    setFiles((prev) => [...prev, placeholder]);
    setActiveFileIndex(newIndex);

    const formData = new FormData();
    formData.append("audio", audioFile);

    const params = new URLSearchParams({
      transcript_words: String(transcriptWords),
      ...(force ? { force: "true" } : {}),
    });

    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/transcriptions/stream?${params}`, {
        method: "POST",
        body: formData,
        headers: authHeaders(),
      });

      if (response.status === 401) {
        clearToken();
        onUnauthorized();
        throw new Error("Unauthorized");
      }

      if (!response.ok || !response.body) {
        throw new Error(`Upload failed (${response.status})`);
      }

      setStreamStatus("transcribing");
      const reader = response.body.getReader();
      const decoder = new TextDecoder("utf-8");
      let buffer = "";

      while (true) {
        const { value, done } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const blocks = buffer.split("\n\n");
        buffer = blocks.pop() ?? "";

        for (const block of blocks) {
          const parsed = parseSseBlock(block.trim());
          if (!parsed) continue;

          if (parsed.event === "status") {
            const msg = String(parsed.data.message ?? "working");
            setStreamStatus(msg);
            if (typeof parsed.data.total_chunks === "number") {
              setTotalChunks(parsed.data.total_chunks as number);
            }
            if (typeof parsed.data.chunk_index === "number") {
              setCurrentChunk((parsed.data.chunk_index as number) + 1);
            }
            continue;
          }

          if (parsed.event === "complete") {
            const transcript = normaliseTranscript(parsed.data as unknown as Transcript);
            setFiles((prev) => {
              const updated = [...prev];
              updated[newIndex] = { name: displayName, transcript, wordsMode: transcriptWords };
              return updated;
            });
            setStreamStatus("done");
            continue;
          }

          if (parsed.event === "error") {
            const msg = String(parsed.data.message ?? "transcription failed");
            setStreamError(msg);
            setStreamStatus("error");
            setFiles((prev) => {
              const existing = prev[newIndex];
              if (!existing) return prev;
              return prev.filter((_, i) => i !== newIndex);
            });
            setActiveFileIndex((cur) => {
              if (cur === newIndex) return -1;
              if (cur > newIndex) return cur - 1;
              return cur;
            });
          }
        }
      }

      const trailing = parseSseBlock(buffer.trim());
      if (trailing?.event === "error") {
        const msg = String(trailing.data.message ?? "transcription failed");
        setStreamError(msg);
        setStreamStatus("error");
        setFiles((prev) => {
          const existing = prev[newIndex];
          if (!existing) return prev;
          return prev.filter((_, i) => i !== newIndex);
        });
        setActiveFileIndex((cur) => {
          if (cur === newIndex) return -1;
          if (cur > newIndex) return cur - 1;
          return cur;
        });
      }
    } catch (err) {
      if ((err as Error).message !== "Unauthorized") {
        setStreamError(err instanceof Error ? err.message : "transcription failed");
        setStreamStatus("error");
      }
      setFiles((prev) => {
        const existing = prev[newIndex];
        if (!existing) return prev;
        return prev.filter((_, i) => i !== newIndex);
      });
      setActiveFileIndex((cur) => {
        if (cur === newIndex) return -1;
        if (cur > newIndex) return cur - 1;
        return cur;
      });
    } finally {
      setIsTranscribing(false);
    }
  };

  const selectFile = (index: number) => setActiveFileIndex(index);

  const fetchLibrary = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/transcriptions`, {
        headers: authHeaders(),
      });
      if (res.status === 401) { clearToken(); onUnauthorized(); return; }
      if (!res.ok) return;
      const body = await res.json();
      const loaded: TranscriptFile[] = (body.transcriptions ?? []).map(
        (entry: { response_json: Transcript; audio_signature: string; file_name?: string }) => ({
          name: entry.file_name ?? entry.audio_signature.slice(0, 8),
          transcript: normaliseTranscript(entry.response_json),
          wordsMode: true,
          audioSignature: entry.audio_signature,
        }),
      );
      if (loaded.length > 0) {
        setFiles(loaded);
        setActiveFileIndex(0);
      }
    } catch {
      // non-fatal — user simply sees an empty library
    }
  }, [onUnauthorized]);

  const renameFile = useCallback(async (index: number, newName: string) => {
    const file = files[index];
    if (!file?.audioSignature) {
      setFiles((prev) => {
        const updated = [...prev];
        if (updated[index]) updated[index] = { ...updated[index], name: newName };
        return updated;
      });
      return;
    }
    try {
      await fetch(
        `${API_BASE_URL}/api/v1/transcriptions/${file.audioSignature}/rename`,
        {
          method: "PATCH",
          headers: { "Content-Type": "application/json", ...authHeaders() },
          body: JSON.stringify({ file_name: newName }),
        },
      );
    } catch { /* best-effort */ }
    setFiles((prev) => {
      const updated = [...prev];
      if (updated[index]) updated[index] = { ...updated[index], name: newName };
      return updated;
    });
  }, [files]);

  const removeFile = (index: number) => {
    setFiles((prev) => prev.filter((_, i) => i !== index));
    setActiveFileIndex((cur) => {
      if (cur === index) return -1;
      if (cur > index) return cur - 1;
      return cur;
    });
  };

  const clearLibrary = () => {
    setFiles([]);
    setActiveFileIndex(-1);
    setStreamStatus("idle");
    setStreamError("");
  };

  return {
    files,
    activeFileIndex,
    isTranscribing,
    streamStatus,
    streamError,
    totalChunks,
    currentChunk,
    loadMock,
    fetchLibrary,
    transcribeAudio,
    selectFile,
    renameFile,
    removeFile,
    clearLibrary,
  };
}
