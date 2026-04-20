"use client";
import React, { useRef } from "react";
import { ASCII_SPINNER, MODELS } from "../constants";
import { ModelType, WordsMode } from "../hooks/useTranscription";
import { TranscriptFile } from "../types";

type Props = {
  isPlaying: boolean;
  playbackSpeed: number;
  isTranscribing: boolean;
  wordsMode: WordsMode;
  activeFile: TranscriptFile | undefined;
  spinnerIdx: number;
  onTogglePlay: () => void;
  onCycleSpeed: () => void;
  onWordsModeToggle: () => void;
  onLoadMock: (path: string) => void;
  onAudioFile: (file: File) => void;
  onHelpOpen: () => void;
};

export default function Header({
  isPlaying,
  playbackSpeed,
  isTranscribing,
  wordsMode,
  activeFile,
  spinnerIdx,
  onTogglePlay,
  onCycleSpeed,
  onWordsModeToggle,
  onLoadMock,
  onAudioFile,
  onHelpOpen,
}: Props) {
  const audioInputRef = useRef<HTMLInputElement | null>(null);

  const handleAudioChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (f) onAudioFile(f);
    if (audioInputRef.current) audioInputRef.current.value = "";
  };

  return (
    <header className="app-header">
      <div className="header-brand">
        <span className="brand-title">CLNKR</span>
        <span className="brand-version">v0.1</span>
        <button className="ctrl-btn help-btn" onClick={onHelpOpen} title="Help (H)">?</button>
        {isPlaying && (
          <span className="live-indicator">
            <span>{ASCII_SPINNER[spinnerIdx]}</span>
            <span className="blink">LIVE</span>
          </span>
        )}
      </div>

      <nav className="header-nav">
        {/*<span className="nav-sep">|</span>*/}

        <button
          className={`ctrl-btn${wordsMode === "words" ? " active" : ""}`}
          onClick={onWordsModeToggle}
          title="Toggle word-level reading mode (W)"
        >
          {wordsMode === "words" ? "[ WORDS ]" : "[ SIMPLE ]"}
        </button>

        <button
          className={`ctrl-btn${isTranscribing ? " active" : ""}`}
          onClick={() => audioInputRef.current?.click()}
          disabled={isTranscribing}
        >
          {isTranscribing ? "TRANSCRIBING" : "UPLOAD AUDIO"}
        </button>
        <input
          type="file"
          ref={audioInputRef}
          accept="audio/*"
          style={{ display: "none" }}
          onChange={handleAudioChange}
        />

        <span className="nav-sep">|</span>

        <button className="ctrl-btn" onClick={onCycleSpeed}>
          {playbackSpeed.toFixed(2)}x
        </button>

        <button
          className={`ctrl-btn${isPlaying ? " active" : ""}`}
          onClick={onTogglePlay}
          disabled={!activeFile}
        >
          {isPlaying ? "[ PAUSE ]" : "[ PLAY ]"}
        </button>
      </nav>
    </header>
  );
}
