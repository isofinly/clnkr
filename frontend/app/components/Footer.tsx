"use client";
import { TranscriptFile } from "../types";
import { formatTime } from "../lib/utils";

type Props = {
  activeFile: TranscriptFile | undefined;
  isPlaying: boolean;
  currentTime: number;
  streamError: string;
};

export default function Footer({ activeFile, isPlaying, currentTime, streamError }: Props) {
  const totalTime = activeFile?.transcript.total_duration_seconds ?? 0;
  const progress = totalTime > 0 ? Math.min(currentTime / totalTime, 1) : 0;

  return (
    <div className="app-footer-wrap">
      {streamError && (
        <div className="footer-error">STREAM ERROR: {streamError}</div>
      )}

      <div className="progress-track">
        <div className="progress-fill" style={{ width: `${progress * 100}%` }} />
      </div>

      <footer className="app-footer">
        <div className="footer-left">
          <span className={`footer-status${isPlaying ? " playing" : ""}`}>
            {isPlaying ? "● REC" : "○ IDLE"}
          </span>
          {activeFile && (
            <span className="footer-filename">{activeFile.name}</span>
          )}
        </div>
        <div className="footer-time">
          {formatTime(currentTime)}
          <span className="footer-time-sep">/</span>
          {formatTime(totalTime)}
        </div>
      </footer>
    </div>
  );
}
