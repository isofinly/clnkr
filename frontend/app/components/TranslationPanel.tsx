"use client";
import { useState, useEffect, useRef } from "react";
import { TranslationEntry } from "../types";
import { API_BASE_URL } from "../constants";
import { authHeaders } from "../lib/auth";

type Props = {
  entry: TranslationEntry;
  onClose: () => void;
  onRetranslate: (entry: TranslationEntry) => void;
  isRetranslating: boolean;
};

export default function TranslationPanel({ entry, onClose, onRetranslate, isRetranslating }: Props) {
  const [noteText, setNoteText] = useState("");
  const [noteStatus, setNoteStatus] = useState<"idle" | "saving" | "saved">("idle");
  const [grammarOpen, setGrammarOpen] = useState(false);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const load = async () => {
      setNoteText("");
      try {
        const res = await fetch(
          `${API_BASE_URL}/api/v1/notes/${entry.input_hash}`,
          { headers: authHeaders() },
        );
        if (res.ok) {
          const body = await res.json();
          setNoteText(body.note_text ?? "");
        }
      } catch { /* no note */ }
    };
    load();
    setNoteStatus("idle");
  }, [entry.input_hash]);

  const handleNoteChange = (val: string) => {
    setNoteText(val);
    setNoteStatus("saving");
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(async () => {
      try {
        await fetch(`${API_BASE_URL}/api/v1/notes/${entry.input_hash}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json", ...authHeaders() },
          body: JSON.stringify({ note_text: val }),
        });
        setNoteStatus("saved");
      } catch { setNoteStatus("idle"); }
    }, 800);
  };

  const o = entry.response_json;

  return (
    <div className="translation-panel">
      <div className="translation-panel-header">
        <div className="translation-panel-title">
          <span className="translation-source">{o.source_text}</span>
          {entry.cached && !isRetranslating && (
            <span className="cache-badge">[cached]</span>
          )}
        </div>
        <div className="translation-panel-actions">
          <button
            className="ctrl-btn"
            onClick={() => onRetranslate(entry)}
            disabled={isRetranslating}
            title="Force re-translate"
          >
            {isRetranslating ? "…" : "R"}
          </button>
          <button className="ctrl-btn" onClick={onClose}>✕</button>
        </div>
      </div>

      {entry.orphaned && (
        <div className="orphan-warning">
          ~ source transcript was refreshed — translation preserved
        </div>
      )}

      <div className="translation-panel-body">
        {/* Translations */}
        <section className="tp-section">
          <div className="tp-section-label">TRANSLATION</div>
          {o.translations.map((t, i) => (
            <div key={i} className="tp-translation">{t}</div>
          ))}
        </section>

        {/* Phrase breakdowns */}
        {o.phrase_breakdowns.length > 0 && (
          <section className="tp-section">
            <div className="tp-section-label">PHRASES</div>
            {o.phrase_breakdowns.map((pb, i) => (
              <div key={i} className="tp-phrase">
                <div className="tp-phrase-head">{pb.phrase}</div>
                <div className="tp-tokens">
                  {pb.tokens.map((tk, ti) => (
                    <span key={ti} className="tp-token">
                      <span className="tp-token-text">{tk.token}</span>
                      <span className="tp-token-meaning">{tk.meaning}</span>
                    </span>
                  ))}
                </div>
              </div>
            ))}
          </section>
        )}

        {/* Kanji list */}
        {o.kanji_words.length > 0 && (
          <section className="tp-section">
            <div className="tp-section-label">KANJI</div>
            <div className="tp-kanji-grid">
              {o.kanji_words.map((kw, i) => (
                <div key={i} className="tp-kanji-row">
                  <span className="tp-kanji-char">{kw.kanji}</span>
                  <span className="tp-kanji-reading">{kw.reading}</span>
                </div>
              ))}
            </div>
          </section>
        )}

        {/* Grammar — collapsible */}
        {o.grammar_constructions.length > 0 && (
          <section className="tp-section">
            <button className="tp-collapsible" onClick={() => setGrammarOpen((p) => !p)}>
              GRAMMAR {grammarOpen ? "▲" : "▼"}
            </button>
            {grammarOpen && o.grammar_constructions.map((gc, i) => (
              <div key={i} className="tp-grammar">
                <span className="tp-grammar-name">{gc.name}</span>
                {gc.pattern && <span className="tp-grammar-pattern">{gc.pattern}</span>}
                <span className="tp-grammar-desc">{gc.description}</span>
              </div>
            ))}
          </section>
        )}

        {/* Note editor */}
        <section className="tp-section">
          <div className="tp-section-label">NOTE</div>
          <textarea
            className="note-editor"
            value={noteText}
            onChange={(e) => handleNoteChange(e.target.value)}
            placeholder="personal note..."
            rows={3}
          />
          <div className="note-save-status">
            {noteStatus === "saving" ? "saving…" : noteStatus === "saved" ? "saved" : ""}
          </div>
        </section>
      </div>
    </div>
  );
}
