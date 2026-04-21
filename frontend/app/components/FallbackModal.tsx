"use client";

type Props = {
  onClose: () => void;
};

export default function FallbackModal({ onClose }: Props) {
  return (
    <div className="help-overlay" onClick={onClose}>
      <div className="help-modal" onClick={(e) => e.stopPropagation()}>
        <div className="help-modal-header">
          <span className="help-modal-title">FALLBACK ACTIVE</span>
          <button className="ctrl-btn" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="help-modal-body">
          <section className="help-section">
            <div className="help-section-title">GEMINI UNAVAILABLE</div>
            <p
              style={{
                fontSize: "var(--text-sm)",
                color: "var(--color-text)",
                lineHeight: 1.6,
                margin: 0,
              }}
            >
              Gemini returned{" "}
              <span style={{ color: "var(--color-error)" }}>503 Service Unavailable</span>.
              Transcription is continuing via{" "}
              <span style={{ color: "var(--color-done)" }}>OpenRouter</span> as a fallback.
            </p>
          </section>
          <section className="help-section" style={{ alignItems: "flex-end" }}>
            <button className="ctrl-btn" onClick={onClose} style={{ padding: "4px 12px" }}>
              OK
            </button>
          </section>
        </div>
      </div>
    </div>
  );
}
