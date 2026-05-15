"use client";
import { useState, useEffect, useCallback } from "react";
import { useTranscription, ModelType, WordsMode } from "./hooks/useTranscription";
import { useTranslation } from "./hooks/useTranslation";
import { getToken, clearToken } from "./lib/auth";
import AuthGate from "./components/AuthGate";
import Header from "./components/Header";
import Sidebar from "./components/Sidebar";
import SegmentView from "./components/SegmentView";
import Footer from "./components/Footer";
import HelpModal from "./components/HelpModal";
import TranslationPanel from "./components/TranslationPanel";

type LogoMode = "idle" | "streaming" | "done";

export default function App() {
  const [authed, setAuthed] = useState(false);
  const [authChecked, setAuthChecked] = useState(false);

  useEffect(() => {
    setAuthed(!!getToken());
    setAuthChecked(true);
  }, []);

  const handleUnauthorized = useCallback(() => {
    clearToken();
    setAuthed(false);
  }, []);

  const {
    files,
    activeFileIndex,
    isTranscribing,
    streamStatus,
    streamError,
    totalChunks,
    currentChunk,
    loadMock,
    fetchLibrary: fetchTranscriptions,
    transcribeAudio,
    selectFile,
    renameFile,
    removeFile,
    clearLibrary,
  } = useTranscription(handleUnauthorized);

  const {
    entries: translationEntries,
    activeEntry,
    pendingSegmentId,
    isTranslating,
    translate,
    retranslate,
    fetchLibrary: fetchTranslations,
    deleteEntry,
    clearAll: clearTranslations,
    openEntry,
    closePanel,
  } = useTranslation(handleUnauthorized);

  // Restore library from server whenever the user becomes authenticated.
  useEffect(() => {
    if (!authed) return;
    fetchTranscriptions();
    fetchTranslations();
  }, [authed, fetchTranscriptions, fetchTranslations]);

  const activeFile = files[activeFileIndex];

  const [showHelp, setShowHelp] = useState(false);
  const [modelType, setModelType] = useState<ModelType>("gemini-flash");
  const [wordsMode, setWordsMode] = useState<WordsMode>("words");
  // Index of the segment the user last clicked; -1 means none selected.
  const [activeSegmentIdx, setActiveSegmentIdx] = useState(-1);

  // Reset selection when switching files.
  useEffect(() => {
    setActiveSegmentIdx(-1);
  }, [activeFileIndex]);

  const logoMode: LogoMode = isTranscribing
    ? "streaming"
    : streamStatus === "done"
      ? "done"
      : "idle";

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.key === "h" || e.key === "H") {
        setShowHelp((p) => !p);
        return;
      }
      if (e.key === "w" || e.key === "W") setWordsMode((p) => (p === "words" ? "simple" : "words"));
      if (e.key === "Escape") {
        closePanel();
        setShowHelp(false);
      }
      if (e.key === "t" || e.key === "T") {
        const segs = activeFile?.transcript.segments;
        if (!segs || activeSegmentIdx === -1) return;
        const seg = segs[activeSegmentIdx];
        if (!seg) return;
        const prevSeg = segs[activeSegmentIdx - 1];
        const nextSeg = segs[activeSegmentIdx + 1];
        const contextParts = [
          prevSeg ? `[previous] ${prevSeg.raw_text}` : null,
          nextSeg ? `[next] ${nextSeg.raw_text}` : null,
        ].filter(Boolean);
        translate({
          translation_input: seg.raw_text,
          segment_id: seg.id,
          context: contextParts.length > 0 ? contextParts.join("\n") : undefined,
        });
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [closePanel, translate, activeFile, activeSegmentIdx]);

  if (!authChecked) return null;
  if (!authed) return <AuthGate onAuthenticated={() => setAuthed(true)} />;

  const handleRetranscribe = (idx: number) => {
    const file = files[idx];
    if (!file) return;
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "audio/*";
    input.onchange = () => {
      const f = input.files?.[0];
      if (f) transcribeAudio(f, { force: true, wordsMode });
    };
    input.click();
  };

  void modelType;

  return (
    <div className="app-shell">
      <div className="dot-grid" />
      <div className="app-content">
        <div className="rule" />

        <Header
          isTranscribing={isTranscribing}
          wordsMode={wordsMode}
          activeFile={activeFile}
          onWordsModeToggle={() => setWordsMode((p) => (p === "words" ? "simple" : "words"))}
          onHelpOpen={() => setShowHelp(true)}
          onLoadMock={loadMock}
          onAudioFile={(f) => transcribeAudio(f, { wordsMode })}
        />

        {showHelp && <HelpModal onClose={() => setShowHelp(false)} />}

        <main style={{ flex: 1, display: "flex", minHeight: 0, position: "relative" }}>
          <Sidebar
            files={files}
            activeFileIndex={activeFileIndex}
            isTranscribing={isTranscribing}
            streamStatus={streamStatus}
            streamError={streamError}
            totalChunks={totalChunks}
            currentChunk={currentChunk}
            translationEntries={translationEntries}
            activeTranslationEntry={activeEntry}
            onSelect={selectFile}
            onRename={renameFile}
            onRemove={removeFile}
            onClear={clearLibrary}
            onRetranscribe={handleRetranscribe}
            onSelectTranslation={openEntry}
            onDeleteTranslation={deleteEntry}
            onClearTranslations={clearTranslations}
          />
          <SegmentView
            activeFile={activeFile}
            activeSegmentIdx={activeSegmentIdx}
            isTranscribing={isTranscribing}
            pendingSegmentId={pendingSegmentId}
            logoMode={logoMode}
            onSelectSegment={setActiveSegmentIdx}
            onTranslateSegment={(segId, text, context) =>
              translate({ translation_input: text, segment_id: segId, context })
            }
          />
          {activeEntry && (
            <TranslationPanel
              entry={activeEntry}
              onClose={closePanel}
              onRetranslate={retranslate}
              isRetranslating={isTranslating}
            />
          )}
        </main>

        <Footer
          activeFile={activeFile}
          activeSegmentIdx={activeSegmentIdx}
          streamError={streamError}
        />

        <div className="rule" />
      </div>
    </div>
  );
}
