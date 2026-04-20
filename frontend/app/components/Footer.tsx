"use client";
import { TranscriptFile } from "../types";
import { formatTime } from "../lib/utils";

type Props = {
  activeFile: TranscriptFile | undefined;
  activeSegmentIdx: number;
  streamError: string;
};

export default function Footer({ activeFile, activeSegmentIdx, streamError }: Props) {
  const segments = activeFile?.transcript.segments ?? [];
  const activeSeg = segments[activeSegmentIdx] ?? null;

  return (
    <div className="app-footer-wrap">
      {streamError && (
        <div className="footer-error">STREAM ERROR: {streamError}</div>
      )}

      <footer className="app-footer">
        <div className="footer-left">
          <span className="footer-status">○ IDLE</span>
          {activeFile && (
            <span className="footer-filename">{activeFile.name}</span>
          )}
        </div>
        {activeSeg && (
          <div className="footer-time">
            {formatTime(activeSeg.start_seconds)}
            <span className="footer-time-sep">–</span>
            {formatTime(activeSeg.end_seconds)}
          </div>
        )}
      </footer>
    </div>
  );
}
