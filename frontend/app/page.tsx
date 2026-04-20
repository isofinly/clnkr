"use client";
import { useState, useEffect, useCallback } from "react";
import { useTranscription, ModelType, WordsMode } from "./hooks/useTranscription";
import { usePlayback } from "./hooks/usePlayback";
import { useTranslation } from "./hooks/useTranslation";
import { ASCII_SPINNER } from "./constants";
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
    loadMock,
    transcribeAudio,
    selectFile,
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
    deleteEntry,
    clearAll: clearTranslations,
    openEntry,
    closePanel,
  } = useTranslation(handleUnauthorized);

  const activeFile = files[activeFileIndex];
  const totalDuration = activeFile?.transcript.total_duration_seconds ?? 0;

  const { isPlaying, currentTime, playbackSpeed, togglePlay, cycleSpeed, jumpToTime, reset } =
    usePlayback(totalDuration);

  const [showHelp, setShowHelp] = useState(false);
  const [modelType, setModelType] = useState<ModelType>("gemini-flash");
  const [wordsMode, setWordsMode] = useState<WordsMode>("words");
  const [spinnerIdx, setSpinnerIdx] = useState(0);

  // Logo animation mode derived from stream state
  const logoMode: LogoMode = isTranscribing
    ? "streaming"
    : streamStatus === "done"
      ? "done"
      : "idle";

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => {
    reset();
  }, [activeFileIndex]);

  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | undefined;
    if (isPlaying)
      interval = setInterval(() => setSpinnerIdx((p) => (p + 1) % ASCII_SPINNER.length), 150);
    return () => clearInterval(interval);
  }, [isPlaying]);

  // Keyboard shortcuts
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement).tagName;
      if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

      if (e.key === "h" || e.key === "H") { setShowHelp((p) => !p); return; }
      if (e.code === "Space") {
        e.preventDefault();
        togglePlay();
      }
      if (e.key === "w" || e.key === "W") setWordsMode((p) => (p === "words" ? "simple" : "words"));
      if (e.key === "Escape") { closePanel(); setShowHelp(false); }
      if (e.key === "t" || e.key === "T") {
        const segs = activeFile?.transcript.segments;
        if (!segs) return;
        let activeIdx = -1;
        for (let i = 0; i < segs.length; i++) {
          if (currentTime >= segs[i].start_seconds) activeIdx = i;
          else break;
        }
        if (activeIdx !== -1) {
          const seg = segs[activeIdx];
          translate({ translation_input: seg.raw_text, segment_id: seg.id });
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [togglePlay, closePanel, translate, activeFile, currentTime]);

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
          isPlaying={isPlaying}
          playbackSpeed={playbackSpeed}
          isTranscribing={isTranscribing}
          wordsMode={wordsMode}
          activeFile={activeFile}
          spinnerIdx={spinnerIdx}
          onTogglePlay={togglePlay}
          onCycleSpeed={cycleSpeed}
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
            translationEntries={translationEntries}
            activeTranslationEntry={activeEntry}
            onSelect={selectFile}
            onRemove={removeFile}
            onClear={clearLibrary}
            onRetranscribe={handleRetranscribe}
            onSelectTranslation={openEntry}
            onDeleteTranslation={deleteEntry}
            onClearTranslations={clearTranslations}
          />
          <SegmentView
            activeFile={activeFile}
            currentTime={currentTime}
            isPlaying={isPlaying}
            isTranscribing={isTranscribing}
            pendingSegmentId={pendingSegmentId}
            logoMode={logoMode}
            onJumpToTime={jumpToTime}
            onTranslateSegment={(segId, text) =>
              translate({ translation_input: text, segment_id: segId })
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
          isPlaying={isPlaying}
          currentTime={currentTime}
          streamError={streamError}
        />

        <div className="rule" />
      </div>
    </div>
  );
}
