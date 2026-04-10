"use client";
import { useState, useEffect } from "react";
import { useTranscription, ModelType } from "./hooks/useTranscription";
import { usePlayback } from "./hooks/usePlayback";
import { ASCII_SPINNER } from "./constants";
import Header from "./components/Header";
import Sidebar from "./components/Sidebar";
import SegmentView from "./components/SegmentView";
import Footer from "./components/Footer";

export default function App() {
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
  } = useTranscription();

  const activeFile = files[activeFileIndex];
  const totalDuration = activeFile?.transcript.total_duration_seconds ?? 0;

  const { isPlaying, currentTime, playbackSpeed, togglePlay, cycleSpeed, jumpToTime, reset } =
    usePlayback(totalDuration);

  const [modelType, setModelType] = useState<ModelType>("gemini-flash");
  const [spinnerIdx, setSpinnerIdx] = useState(0);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  useEffect(() => { reset(); }, [activeFileIndex]);

  useEffect(() => {
    let interval: ReturnType<typeof setInterval> | undefined;
    if (isPlaying)
      interval = setInterval(() => setSpinnerIdx((p) => (p + 1) % ASCII_SPINNER.length), 150);
    return () => clearInterval(interval);
  }, [isPlaying]);

  return (
    <div className="app-shell">
      <div className="dot-grid" />
      <div className="app-content">
        <div className="rule" />

        <Header
          isPlaying={isPlaying}
          playbackSpeed={playbackSpeed}
          isTranscribing={isTranscribing}
          modelType={modelType}
          activeFile={activeFile}
          spinnerIdx={spinnerIdx}
          onTogglePlay={togglePlay}
          onCycleSpeed={cycleSpeed}
          onModelChange={setModelType}
          onLoadMock={loadMock}
          onAudioFile={(f) => transcribeAudio(f, modelType)}
        />

        <main style={{ flex: 1, display: "flex", minHeight: 0 }}>
          <Sidebar
            files={files}
            activeFileIndex={activeFileIndex}
            isTranscribing={isTranscribing}
            streamStatus={streamStatus}
            streamError={streamError}
            onSelect={selectFile}
            onRemove={removeFile}
            onClear={clearLibrary}
          />
          <SegmentView
            activeFile={activeFile}
            currentTime={currentTime}
            isPlaying={isPlaying}
            onJumpToTime={jumpToTime}
          />
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
