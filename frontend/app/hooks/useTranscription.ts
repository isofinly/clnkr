"use client";
import { useState, useRef } from "react";
import { Transcript, TranscriptFile, StreamStatus } from "../types";
import { API_BASE_URL } from "../constants";
import { parseSseBlock } from "../lib/utils";

export type ModelType = "gemini-flash" | "gemini-flash-lite";

export function useTranscription() {
  const [files, setFiles] = useState<TranscriptFile[]>([]);
  const [activeFileIndex, setActiveFileIndex] = useState(-1);
  const [isTranscribing, setIsTranscribing] = useState(false);
  const [streamStatus, setStreamStatus] = useState<StreamStatus>("idle");
  const [streamError, setStreamError] = useState("");
  const streamSegmentCounterRef = useRef(0);

  const loadMock = async (path: string) => {
    const res = await fetch(path);
    const transcript: Transcript = await res.json();
    const name = path.split("/").pop() ?? path;
    setFiles((prev) => [...prev, { name, transcript }]);
    setActiveFileIndex((prev) => (prev === -1 ? 0 : prev));
  };

  const transcribeAudio = async (audioFile: File, modelType: ModelType) => {
    const displayName = audioFile.name.replace(/\.[^.]+$/, "") || "transcript";

    const placeholder: TranscriptFile = {
      name: `${displayName} [streaming]`,
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

    try {
      const response = await fetch(
        `${API_BASE_URL}/api/v1/transcriptions/stream?model_type=${modelType}`,
        { method: "POST", body: formData },
      );

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

          if (parsed.event === "segment") {
            streamSegmentCounterRef.current += 1;
            const streamId = `stream-${streamSegmentCounterRef.current}`;
            const seg = parsed.data as Record<string, unknown>;

            setFiles((prev) => {
              const updated = [...prev];
              const existing = updated[newIndex];
              if (!existing) return prev;
              updated[newIndex] = {
                ...existing,
                transcript: {
                  ...existing.transcript,
                  segments: [
                    ...existing.transcript.segments,
                    {
                      id: Number(seg.id ?? streamSegmentCounterRef.current),
                      start_seconds: Number(seg.start_seconds ?? 0),
                      end_seconds: Number(seg.end_seconds ?? 0),
                      raw_text: String(seg.raw_text ?? ""),
                      words: Array.isArray(seg.words) ? (seg.words as never) : [],
                      translation: String(seg.translation ?? ""),
                      speaker: (seg.speaker as { speaker_id: string }) ?? { speaker_id: "s0" },
                      isStreaming: true,
                      streamId,
                    },
                  ],
                },
              };
              return updated;
            });
            continue;
          }

          if (parsed.event === "complete") {
            const transcript = parsed.data as unknown as Transcript;
            setFiles((prev) => {
              const updated = [...prev];
              updated[newIndex] = { name: displayName, transcript };
              return updated;
            });
            setStreamStatus("done");
            continue;
          }

          if (parsed.event === "error") {
            setStreamError(String(parsed.data.message ?? "transcription failed"));
            setStreamStatus("error");
          }
        }
      }

      const trailing = parseSseBlock(buffer.trim());
      if (trailing?.event === "error") {
        setStreamError(String(trailing.data.message ?? "transcription failed"));
        setStreamStatus("error");
      }
    } catch (err) {
      setStreamError(err instanceof Error ? err.message : "transcription failed");
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
