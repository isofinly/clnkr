"use client";

type Props = {
  onClose: () => void;
};

const SECTIONS = [
  {
    title: "KEYBOARD",
    rows: [
      ["Space", "Play / pause playback"],
      ["W", "Toggle words / simple mode"],
      ["T", "Translate active segment"],
      ["H", "Toggle this help panel"],
      ["Escape", "Close open panels"],
    ],
  },
  {
    title: "HEADER BUTTONS",
    rows: [
      ["[ WORDS ] / [ SIMPLE ]", "Switch between word-chip and plain-text view"],
      ["UPLOAD AUDIO", "Pick an audio file and start transcription (SSE stream)"],
      ["0.50x … 2.00x", "Cycle playback speed"],
      ["[ PLAY ] / [ PAUSE ]", "Start or stop the playback clock"],
      ["?", "Open this help panel"],
    ],
  },
  {
    title: "SEGMENT VIEW",
    rows: [
      ["Click segment", "Jump playback to that segment's timestamp"],
      ["Hover word chip", "Reveal kana reading above the word"],
      ["T button (words mode)", "Translate the segment via Gemini"],
      ["Click raw text (simple mode)", "Translate the segment via Gemini"],
    ],
  },
  {
    title: "LIBRARY — TRANSCRIPTIONS",
    rows: [
      ["Click item", "Switch active transcript"],
      ["R button", "Re-upload audio to force-retranscribe"],
      ["× button (×2)", "Remove transcript from local library"],
      ["CLEAR (×2)", "Remove all transcripts"],
    ],
  },
  {
    title: "LIBRARY — TRANSLATIONS",
    rows: [
      ["Click item", "Open translation panel for that entry"],
      ["× button (×2)", "Delete translation entry"],
      ["CLEAR (×2)", "Delete all translation entries"],
      ["✎ badge", "Entry has a saved personal note"],
      ["~ prefix", "Source transcript was refreshed; translation preserved"],
    ],
  },
  {
    title: "TRANSLATION PANEL",
    rows: [
      ["R button", "Force re-translate via Gemini"],
      ["✕ button", "Close panel"],
      ["Note textarea", "Personal note, auto-saved per translation"],
    ],
  },
];

export default function HelpModal({ onClose }: Props) {
  return (
    <div className="help-overlay" onClick={onClose}>
      <div className="help-modal" onClick={(e) => e.stopPropagation()}>
        <div className="help-modal-header">
          <span className="help-modal-title">HELP</span>
          <button className="ctrl-btn" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="help-modal-body">
          {SECTIONS.map((sec) => (
            <section key={sec.title} className="help-section">
              <div className="help-section-title">{sec.title}</div>
              <table className="help-table">
                <tbody>
                  {sec.rows.map(([key, desc]) => (
                    <tr key={key}>
                      <td className="help-key">{key}</td>
                      <td className="help-desc">{desc}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </section>
          ))}
        </div>
      </div>
    </div>
  );
}
