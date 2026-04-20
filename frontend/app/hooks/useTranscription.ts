"use client";
import { useState, useRef } from "react";
import { Transcript, TranscriptFile, StreamStatus, RawSegment } from "../types";
import { API_BASE_URL } from "../constants";
import { parseSseBlock, normaliseTranscript, normaliseSegment } from "../lib/utils";
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
  const streamSegmentCounterRef = useRef(0);

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
            setStreamStatus(String(parsed.data.message ?? "working").toLowerCase());
            continue;
          }

          // chunk events carry raw JSON text — we accumulate but don't render yet
          if (parsed.event === "chunk") continue;

          if (parsed.event === "segment") {
            const seg = normaliseSegment(parsed.data as unknown as RawSegment);
            setFiles((prev) => {
              const updated = [...prev];
              const file = updated[newIndex];
              if (!file) return prev;
              updated[newIndex] = {
                ...file,
                transcript: {
                  ...file.transcript,
                  segments: [...file.transcript.segments, seg],
                },
              };
              return updated;
            });
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
              if (!existing || existing.transcript.segments.length > 0) return prev;
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
          if (!existing || existing.transcript.segments.length > 0) return prev;
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
        if (!existing || existing.transcript.segments.length > 0) return prev;
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
    loadMock,
    transcribeAudio,
    selectFile,
    removeFile,
    clearLibrary,
  };
}
