"use client";
import { TranscriptFile, StreamStatus } from "../types";

type Props = {
  files: TranscriptFile[];
  activeFileIndex: number;
  isTranscribing: boolean;
  streamStatus: StreamStatus;
  streamError: string;
  onSelect: (i: number) => void;
  onRemove: (i: number) => void;
  onClear: () => void;
};

export default function Sidebar({
  files,
  activeFileIndex,
  isTranscribing,
  streamStatus,
  streamError,
  onSelect,
  onRemove,
  onClear,
}: Props) {
  const activeFile = files[activeFileIndex];

  return (
    <aside className="app-sidebar">
      <div className="sidebar-heading">
        <span>Library</span>
        <button
          className="library-action"
          onClick={onClear}
          disabled={files.length === 0 || isTranscribing}
          title={isTranscribing ? "Disabled while streaming" : "Clear all library items"}
        >
          CLEAR
        </button>
      </div>

      <div className="sidebar-list">
        {files.length === 0 ? (
          <div className="sidebar-empty">No files loaded.</div>
        ) : (
          files.map((file, idx) => (
            <div
              key={idx}
              className={`file-item${activeFileIndex === idx ? " is-active" : ""}`}
              onClick={() => onSelect(idx)}
            >
              <div className="file-item-row">
                <span className="file-item-name">{file.name}</span>
                <button
                  className="library-item-delete"
                  disabled={isTranscribing}
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(idx);
                  }}
                  title={isTranscribing ? "Disabled while streaming" : "Remove this item"}
                >
                  ×
                </button>
              </div>
            </div>
          ))
        )}
      </div>

      <div className="sidebar-stats">
        <div>
          files: <span className="stat-value">{files.length}</span>
        </div>
        <div>
          stream:{" "}
          <span className={`stat-value${streamError ? " error" : ""}`}>
            {streamStatus}
          </span>
        </div>
        {activeFile && (
          <div>
            segments: <span className="stat-value">{activeFile.transcript.segments.length}</span>
          </div>
        )}
        {activeFile && activeFile.transcript.speakers.length > 0 && (
          <div>
            speakers: <span className="stat-value">{activeFile.transcript.speakers.length}</span>
          </div>
        )}
      </div>
    </aside>
  );
}
