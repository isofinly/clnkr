"use client";
import { useState } from "react";
import { setToken } from "../lib/auth";

type Props = { onAuthenticated: () => void };

export default function AuthGate({ onAuthenticated }: Props) {
  const [value, setValue] = useState("");
  const [error, setError] = useState("");

  const submit = () => {
    const trimmed = value.trim();
    if (!trimmed) { setError("token cannot be empty"); return; }
    setToken(trimmed);
    onAuthenticated();
  };

  return (
    <div className="auth-gate">
      <div className="auth-box">
        <div className="auth-logo">CLNKR</div>
        <div className="auth-label">PASTE YOUR TOKEN</div>
        <textarea
          className="auth-input"
          value={value}
          onChange={(e) => { setValue(e.target.value); setError(""); }}
          onKeyDown={(e) => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); submit(); } }}
          placeholder="eyJ..."
          rows={3}
          autoFocus
        />
        {error && <div className="auth-error">{error}</div>}
        <button className="ctrl-btn" onClick={submit}>
          AUTHENTICATE
        </button>
      </div>
    </div>
  );
}
