"use client";
import React, { useRef, useState } from "react";
import { MODELS } from "../constants";
import { ModelType, WordsMode } from "../hooks/useTranscription";
import { TranscriptFile } from "../types";

type Props = {
  isTranscribing: boolean;
  wordsMode: WordsMode;
  activeFile: TranscriptFile | undefined;
  onWordsModeToggle: () => void;
  onLoadMock: (path: string) => void;
  onAudioFile: (file: File) => void;
  onHelpOpen: () => void;
};

export default function Header({
  isTranscribing,
  wordsMode,
  activeFile,
  onWordsModeToggle,
  onLoadMock,
  onAudioFile,
  onHelpOpen,
}: Props) {
  const audioInputRef = useRef<HTMLInputElement | null>(null);
  const [largeFileWarning, setLargeFileWarning] = useState(false);

  const WARN_BYTES = 50 * 1024 * 1024; // 50 MiB — advise splitting

  const handleAudioChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0];
    if (!f) return;
    setLargeFileWarning(f.size > WARN_BYTES);
    onAudioFile(f);
    if (audioInputRef.current) audioInputRef.current.value = "";
  };

  void activeFile;
  void MODELS;

  return (
    <>
      <header className="app-header">
        <div className="header-brand">
          <span className="brand-title">CLNKR</span>
          <span className="brand-version">v0.1</span>
          <button className="ctrl-btn help-btn" onClick={onHelpOpen} title="Help (H)">
            ?
          </button>
        </div>

        <nav className="header-nav">
          {/*TODO */}
          {/*<button
            className={`ctrl-btn${wordsMode === "words" ? " active" : ""}`}
            onClick={onWordsModeToggle}
            title="Toggle word-level reading mode (W)"
          >
            {wordsMode === "words" ? "[ WORDS ]" : "[ SIMPLE ]"}
          </button>*/}

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
        </nav>
      </header>
      {largeFileWarning && (
        <div className="file-size-warning" role="alert">
          <span>
            ⚠ file is large (&gt;50 MiB) — consider splitting into smaller parts for better results
          </span>
          <button className="file-size-warning-dismiss" onClick={() => setLargeFileWarning(false)}>
            ✕
          </button>
        </div>
      )}
    </>
  );
}
