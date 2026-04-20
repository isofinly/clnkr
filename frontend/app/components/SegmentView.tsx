"use client";
import { useEffect, useRef } from "react";
import { Segment, TranscriptFile } from "../types";
import { getSpeakerColor, formatTime } from "../lib/utils";
import LogoAnimation from "./LogoAnimation";

/** Build a context string from the surrounding segments for the translator. */
function buildContext(segments: Segment[], idx: number): string | undefined {
  const parts: string[] = [];
  const prev = segments[idx - 1];
  const next = segments[idx + 1];
  if (prev) parts.push(`[previous] ${prev.raw_text}`);
  if (next) parts.push(`[next] ${next.raw_text}`);
  return parts.length > 0 ? parts.join("\n") : undefined;
}

type AnimationMode = "idle" | "streaming" | "done";

type Props = {
  activeFile: TranscriptFile | undefined;
  currentTime: number;
  isPlaying: boolean;
  isTranscribing: boolean;
  pendingSegmentId: number | null;
  logoMode: AnimationMode;
  onJumpToTime: (t: number) => void;
  onTranslateSegment: (segId: number, text: string, context?: string) => void;
};

export default function SegmentView({
  activeFile,
  currentTime,
  isPlaying,
  isTranscribing,
  pendingSegmentId,
  logoMode,
  onJumpToTime,
  onTranslateSegment,
}: Props) {
  const activeLineRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (isPlaying && activeLineRef.current) {
      activeLineRef.current.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [currentTime, isPlaying]);

  if (!activeFile) {
    return (
      <div className="segment-empty">
        <LogoAnimation mode={logoMode} />
        <div className="segment-empty-label">WAITING FOR INPUT</div>
      </div>
    );
  }

  const segments = activeFile.transcript.segments;

  // While streaming and no segments have arrived yet, show the animation
  if (isTranscribing && segments.length === 0) {
    return (
      <div className="segment-empty">
        <LogoAnimation mode={logoMode} />
        <div className="segment-empty-label">TRANSCRIBING…</div>
      </div>
    );
  }

  let activeSegmentIdx = -1;
  for (let i = 0; i < segments.length; i++) {
    if (currentTime >= segments[i].start_seconds) activeSegmentIdx = i;
    else break;
  }

  return (
    <div className="segment-scroll">
      <div className="segment-scroll-inner">
        {segments.map((seg: Segment, idx: number) => {
          const isActive = idx === activeSegmentIdx;
          const isPast = idx < activeSegmentIdx;
          const speakerColor = getSpeakerColor(seg.speaker.speaker_id);
          const speakerLabel = seg.speaker.label ?? seg.speaker.speaker_id;
          const isTranslationPending = pendingSegmentId === seg.id;

          const lineClass = [
            "segment-line",
            isActive ? "is-active" : "",
            isPast ? "is-past" : "",
            seg.isStreaming ? "streaming" : "",
          ].filter(Boolean).join(" ");

          return (
            <div
              key={seg.streamId ?? seg.id}
              ref={isActive ? activeLineRef : null}
              onClick={() => onJumpToTime(seg.start_seconds)}
              className={lineClass}
            >
              <span className="segment-timestamp">{formatTime(seg.start_seconds)}</span>

              <div
                className="segment-body"
                style={{ borderLeftColor: isActive ? speakerColor : undefined }}
              >
                <span className="segment-speaker" style={{ color: speakerColor }}>
                  {speakerLabel}
                </span>

                <div className="segment-text">
                  {seg.words.length > 0
                    ? seg.words.map((w, wi) => (
                        <span key={wi} className="word-chip">
                          <span className="word-reading">{w.reading}</span>
                          <span>{w.text}</span>
                        </span>
                      ))
                    : (
                      <span
                        className="raw-text-clickable"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!isTranscribing && !isTranslationPending) onTranslateSegment(seg.id, seg.raw_text, buildContext(segments, idx));
                        }}
                        title={isTranslationPending ? "Translating…" : "Click to translate"}
                        style={{ cursor: isTranslationPending ? "wait" : undefined }}
                      >
                        {isTranslationPending ? (
                          <span className="translation-pending">translating</span>
                        ) : seg.raw_text}
                      </span>
                    )
                  }
                  {seg.words.length > 0 && (
                    <button
                      className={`translate-chip${isTranslationPending ? " is-pending" : ""}`}
                      disabled={isTranscribing || isTranslationPending}
                      onClick={(e) => {
                        e.stopPropagation();
                        if (!isTranscribing && !isTranslationPending) onTranslateSegment(seg.id, seg.raw_text, buildContext(segments, idx));
                      }}
                      title={isTranslationPending ? "Translating…" : "Translate"}
                    >
                      {isTranslationPending ? "·" : "T"}
                    </button>
                  )}
                </div>

                {seg.translation && (
                  <div className="segment-translation">{seg.translation}</div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
