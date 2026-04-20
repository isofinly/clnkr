"use client";
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
  activeSegmentIdx: number;
  isTranscribing: boolean;
  pendingSegmentId: number | null;
  logoMode: AnimationMode;
  onSelectSegment: (idx: number) => void;
  onTranslateSegment: (segId: number, text: string, context?: string) => void;
};

export default function SegmentView({
  activeFile,
  activeSegmentIdx,
  isTranscribing,
  pendingSegmentId,
  logoMode,
  onSelectSegment,
  onTranslateSegment,
}: Props) {
  if (!activeFile) {
    return (
      <div className="segment-empty">
        <LogoAnimation mode={logoMode} />
        <div className="segment-empty-label">WAITING FOR INPUT</div>
      </div>
    );
  }

  const segments = activeFile.transcript.segments;

  if (isTranscribing && segments.length === 0) {
    return (
      <div className="segment-empty">
        <LogoAnimation mode={logoMode} />
        <div className="segment-empty-label">TRANSCRIBING…</div>
      </div>
    );
  }

  return (
    <div className="segment-scroll">
      <div className="segment-scroll-inner">
        {segments.map((seg: Segment, idx: number) => {
          const isActive = idx === activeSegmentIdx;
          const speakerColor = getSpeakerColor(seg.speaker.speaker_id);
          const speakerLabel = seg.speaker.label ?? seg.speaker.speaker_id;
          const isTranslationPending = pendingSegmentId === seg.id;

          const lineClass = [
            "segment-line",
            isActive ? "is-active" : "",
            seg.isStreaming ? "streaming" : "",
          ].filter(Boolean).join(" ");

          return (
            <div
              key={seg.streamId ?? seg.id}
              onClick={() => onSelectSegment(idx)}
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
