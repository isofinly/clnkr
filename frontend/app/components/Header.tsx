"use client";
import React, { useRef } from "react";
import { ASCII_SPINNER, MODELS } from "../constants";
import { ModelType } from "../hooks/useTranscription";
import { TranscriptFile } from "../types";

type Props = {
  isPlaying: boolean;
  playbackSpeed: number;
  isTranscribing: boolean;
  modelType: ModelType;
  activeFile: TranscriptFile | undefined;
  spinnerIdx: number;
  onTogglePlay: () => void;
  onCycleSpeed: () => void;
  onModelChange: (m: ModelType) => void;
  onLoadMock: (path: string) => void;
  onAudioFile: (file: File) => void;
};

export default function Header({
  isPlaying,
  playbackSpeed,
  isTranscribing,
  modelType,
  activeFile,
  spinnerIdx,
  onTogglePlay,
  onCycleSpeed,
  onModelChange,
  onLoadMock,
  onAudioFile,
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
        {isPlaying && (
          <span className="live-indicator">
            <span>{ASCII_SPINNER[spinnerIdx]}</span>
            <span className="blink">LIVE</span>
          </span>
        )}
      </div>

      <nav className="header-nav">
        <button className="ctrl-btn" onClick={() => onLoadMock("/mock1.json")}>
          MOCK 1
        </button>
        <button className="ctrl-btn" onClick={() => onLoadMock("/mock2.json")}>
          MOCK 2
        </button>

        <span className="nav-sep">|</span>

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

        <select
          className="ctrl-btn"
          value={modelType}
          onChange={(e) => onModelChange(e.target.value as ModelType)}
          style={{ minWidth: "148px" }}
        >
          {MODELS.map((m) => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
        </select>

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
