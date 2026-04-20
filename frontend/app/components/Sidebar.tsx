"use client";
import { useState, useRef, useEffect } from "react";
import { TranscriptFile, StreamStatus, TranslationEntry, LibraryTab } from "../types";

const DELETE_TIMEOUT_MS = 5000;

type Props = {
  files: TranscriptFile[];
  activeFileIndex: number;
  isTranscribing: boolean;
  streamStatus: StreamStatus;
  streamError: string;
  translationEntries: TranslationEntry[];
  activeTranslationEntry: TranslationEntry | null;
  onSelect: (i: number) => void;
  onRename: (i: number, name: string) => void;
  onRemove: (i: number) => void;
  onClear: () => void;
  onRetranscribe: (i: number) => void;
  onSelectTranslation: (entry: TranslationEntry) => void;
  onDeleteTranslation: (id: string) => void;
  onClearTranslations: () => void;
};

function RenameInput({
  initial,
  onCommit,
  onCancel,
}: {
  initial: string;
  onCommit: (v: string) => void;
  onCancel: () => void;
}) {
  const [value, setValue] = useState(initial);
  const ref = useRef<HTMLInputElement>(null);

  useEffect(() => { ref.current?.select(); }, []);

  return (
    <input
      ref={ref}
      className="file-rename-input"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onKeyDown={(e) => {
        if (e.key === "Enter") { e.preventDefault(); onCommit(value.trim() || initial); }
        if (e.key === "Escape") { e.preventDefault(); onCancel(); }
      }}
      onBlur={() => onCommit(value.trim() || initial)}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

export default function Sidebar({
  files,
  activeFileIndex,
  isTranscribing,
  streamStatus,
  streamError,
  translationEntries,
  activeTranslationEntry,
  onSelect,
  onRename,
  onRemove,
  onClear,
  onRetranscribe,
  onSelectTranslation,
  onDeleteTranslation,
  onClearTranslations,
}: Props) {
  const [tab, setTab] = useState<LibraryTab>("transcriptions");
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [clearPending, setClearPending] = useState(false);
  const [renamingIdx, setRenamingIdx] = useState<number | null>(null);
  const deleteTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const clearTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const activeFile = files[activeFileIndex];

  const armDelete = (id: string) => {
    if (pendingDeleteId === id) {
      if (deleteTimerRef.current) clearTimeout(deleteTimerRef.current);
      setPendingDeleteId(null);
      if (tab === "transcriptions") {
        onRemove(Number(id));
      } else {
        onDeleteTranslation(id);
      }
      return;
    }
    if (deleteTimerRef.current) clearTimeout(deleteTimerRef.current);
    setPendingDeleteId(id);
    deleteTimerRef.current = setTimeout(() => setPendingDeleteId(null), DELETE_TIMEOUT_MS);
  };

  const armClear = () => {
    if (clearPending) {
      if (clearTimerRef.current) clearTimeout(clearTimerRef.current);
      setClearPending(false);
      tab === "transcriptions" ? onClear() : onClearTranslations();
      return;
    }
    setClearPending(true);
    clearTimerRef.current = setTimeout(() => setClearPending(false), DELETE_TIMEOUT_MS);
  };

  return (
    <aside className="app-sidebar">
      <div className="sidebar-heading">
        <span>Library</span>
        <button
          className={`library-action${clearPending ? " delete-pending" : ""}`}
          onClick={armClear}
          disabled={
            (tab === "transcriptions" ? files.length === 0 : translationEntries.length === 0) ||
            isTranscribing
          }
          title="Clear all — click twice to confirm"
        >
          {clearPending ? "CONFIRM?" : "CLEAR"}
        </button>
      </div>

      <div className="library-tab-group">
        <button
          className={`library-tab${tab === "transcriptions" ? " active" : ""}`}
          onClick={() => setTab("transcriptions")}
        >
          TRANS
        </button>
        <button
          className={`library-tab${tab === "translations" ? " active" : ""}`}
          onClick={() => setTab("translations")}
        >
          TRANS&rsquo;N
        </button>
      </div>

      <div className="sidebar-list">
        {tab === "transcriptions" ? (
          files.length === 0 ? (
            <div className="sidebar-empty">No files loaded.</div>
          ) : (
            files.map((file, idx) => {
              const itemId = String(idx);
              const isPending = pendingDeleteId === itemId;
              const isRenaming = renamingIdx === idx;
              return (
                <div
                  key={idx}
                  className={`file-item${activeFileIndex === idx ? " is-active" : ""}${isPending ? " delete-pending" : ""}`}
                  onClick={() => { if (!isRenaming) onSelect(idx); }}
                >
                  <div className="file-item-row">
                    {isRenaming ? (
                      <RenameInput
                        initial={file.name}
                        onCommit={(name) => { onRename(idx, name); setRenamingIdx(null); }}
                        onCancel={() => setRenamingIdx(null)}
                      />
                    ) : (
                      <span
                        className="file-item-name"
                        onDoubleClick={(e) => { e.stopPropagation(); setRenamingIdx(idx); }}
                        title="Double-click to rename"
                      >
                        {file.name}
                        {file.wordsMode && <span className="item-badge">[W]</span>}
                      </span>
                    )}
                    <div className="file-item-actions">
                      <button
                        className="library-item-action"
                        disabled={isTranscribing}
                        onClick={(e) => { e.stopPropagation(); onRetranscribe(idx); }}
                        title="Re-transcribe"
                      >
                        R
                      </button>
                      <button
                        className={`library-item-delete${isPending ? " pending" : ""}`}
                        disabled={isTranscribing}
                        onClick={(e) => { e.stopPropagation(); armDelete(itemId); }}
                        title={isPending ? "Click again to confirm" : "Remove"}
                      >
                        {isPending ? "?" : "×"}
                      </button>
                    </div>
                  </div>
                  {isPending && <div className="delete-drain-bar" />}
                </div>
              );
            })
          )
        ) : (
          translationEntries.length === 0 ? (
            <div className="sidebar-empty">No translations yet.</div>
          ) : (
            translationEntries.map((entry) => {
              const isPending = pendingDeleteId === entry.id;
              const isActive = activeTranslationEntry?.id === entry.id;
              const preview = entry.input_json.translation_input.slice(0, 40);
              return (
                <div
                  key={entry.id}
                  className={`file-item${isActive ? " is-active" : ""}${isPending ? " delete-pending" : ""}`}
                  onClick={() => onSelectTranslation(entry)}
                >
                  <div className="file-item-row">
                    <span className="file-item-name">
                      {entry.orphaned && <span className="orphan-glyph">~ </span>}
                      {preview}{preview.length < entry.input_json.translation_input.length ? "…" : ""}
                      {entry.note && <span className="item-badge">✎</span>}
                    </span>
                    <button
                      className={`library-item-delete${isPending ? " pending" : ""}`}
                      onClick={(e) => { e.stopPropagation(); armDelete(entry.id); }}
                      title={isPending ? "Click again to confirm" : "Remove"}
                    >
                      {isPending ? "?" : "×"}
                    </button>
                  </div>
                  {isPending && <div className="delete-drain-bar" />}
                </div>
              );
            })
          )
        )}
      </div>

      <div className="sidebar-stats">
        <div>files: <span className="stat-value">{files.length}</span></div>
        <div>
          stream:{" "}
          <span className={`stat-value${streamError ? " error" : ""}`}>{streamStatus}</span>
        </div>
        {activeFile && (
          <div>segments: <span className="stat-value">{activeFile.transcript.segments.length}</span></div>
        )}
        {activeFile && activeFile.transcript.speakers.length > 0 && (
          <div>speakers: <span className="stat-value">{activeFile.transcript.speakers.length}</span></div>
        )}
        <div>translations: <span className="stat-value">{translationEntries.length}</span></div>
      </div>
    </aside>
  );
}
