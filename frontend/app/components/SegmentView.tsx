"use client";
import { useEffect, useRef } from "react";
import { Segment, TranscriptFile } from "../types";
import { getSpeakerColor, formatTime } from "../lib/utils";
import { ASCII_LOGO } from "../constants";

type Props = {
  activeFile: TranscriptFile | undefined;
  currentTime: number;
  isPlaying: boolean;
  onJumpToTime: (t: number) => void;
};

export default function SegmentView({ activeFile, currentTime, isPlaying, onJumpToTime }: Props) {
  const activeLineRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (isPlaying && activeLineRef.current) {
      activeLineRef.current.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [currentTime, isPlaying]);

  if (!activeFile) {
    return (
      <div className="segment-empty">
        <pre className="segment-empty-logo">{ASCII_LOGO}</pre>
        <div className="segment-empty-label">WAITING FOR INPUT</div>
      </div>
    );
  }

  const segments = activeFile.transcript.segments;

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

          const lineClass = [
            "segment-line",
            isActive ? "is-active" : "",
            isPast ? "is-past" : "",
            seg.isStreaming ? "streaming" : "",
          ]
            .filter(Boolean)
            .join(" ");

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
                    : seg.raw_text}
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
