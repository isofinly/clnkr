# AGENTS.md — clnkr-trnscrb

Speech-to-text app that transcribes audio via a Rust/Axum backend and displays timestamped, word-annotated transcripts in a Next.js frontend. The backend calls Gemini Flash (or Flash Lite), streams back structured segments, and the frontend renders them as a karaoke-style viewer with per-word kana readings.

---

## Repository Layout

```
/                        # Cargo workspace root (members: ["backend"])
├── backend/
│   ├── src/main.rs      # Entire API (stub — one GET / route)
│   ├── Cargo.toml       # edition = "2024"; axum 0.8, sqlx 0.8, tokio, serde, thiserror
│   ├── Dockerfile       # Multi-stage build → debian:bookworm-slim, EXPOSE 8080
│   └── fly.toml         # fly.io app "transcribe-backend", region ams, port 8080
├── frontend/
│   ├── app/
│   │   ├── page.tsx               # Root client component — composes all pieces
│   │   ├── layout.tsx             # Minimal root layout (no font imports — IBM Plex Mono via CSS)
│   │   ├── globals.css            # Tailwind v4 @theme tokens + all component CSS classes
│   │   ├── types.ts               # All shared TypeScript types (Word, Segment, Transcript, …)
│   │   ├── constants.ts           # ASCII_LOGO, SPINNER, SPEAKER_COLORS, MODELS, API_BASE_URL
│   │   ├── lib/utils.ts           # getSpeakerColor, formatTime, parseSseBlock
│   │   ├── hooks/
│   │   │   ├── usePlayback.ts     # RAF-based playback clock, speed cycling, jump
│   │   │   └── useTranscription.ts# Library state, mock loading, SSE streaming
│   │   └── components/
│   │       ├── Header.tsx         # Top bar: mock buttons, audio upload, model select, controls
│   │       ├── Sidebar.tsx        # File library list + stream status stats
│   │       ├── SegmentView.tsx    # Karaoke scroll panel with word chips
│   │       └── Footer.tsx         # Progress bar + time display
│   ├── public/
│   │   ├── mock1.json             # Single-speaker JP transcript (Transcript schema)
│   │   └── mock2.json             # Three-speaker JP transcript (Transcript schema)
│   └── .env                       # NEXT_PUBLIC_API_BASE_URL (Tailscale IP, port 8000)
└── docs/
    ├── openapi_v1.json            # Full API contract (Transcript/Segment/Word schemas)
    └── structured_output.json     # JSON Schema for Gemini structured output
```

---

## Commands

### Backend (Rust)
```bash
cargo build                      # debug, run from repo root or backend/
cargo run -p transcriber-api    # starts on 0.0.0.0:8080
cargo test
cargo clippy
```

### Frontend (Next.js)
Use **bun** (provided by the Nix devshell):
```bash
cd frontend
bun install
bun run dev       # dev server, default port 3000
bun run build     # production build + typecheck
bun run lint
```

### Deployment
```bash
cd backend && fly deploy   # builds Dockerfile, pushes to fly.io
```

### Nix Dev Shell
```bash
nix develop   # stable Rust + rust-analyzer + bun + postgresql + openssl
```

---

## Data Model

The frontend's source of truth is `Transcript` (from `app/types.ts`), matching `docs/openapi_v1.json`:

```
Transcript
  source_language: string          # BCP-47, e.g. "ja"
  target_language: string          # e.g. "en"
  total_duration_seconds: number
  speakers: Speaker[]              # { speaker_id, label? }
  segments: Segment[]

Segment
  id: number                       # zero-based
  start_seconds / end_seconds: number
  raw_text: string                 # unsegmented original
  words: Word[]                    # { text, reading (kana), romanization }
  translation: string              # English translation
  speaker: Speaker
  isStreaming?: boolean            # runtime only — true while SSE is live
  streamId?: string                # runtime only — stable React key during stream
```

`TranscriptFile = { name: string; transcript: Transcript }` — this is what the library stores.

---

## Architecture & Data Flow

```
[Browser]
  → MOCK: fetch /mock1.json or /mock2.json  (loaded directly, no backend)
  → AUDIO: POST /api/v1/transcriptions/stream?model_type=gemini-flash[-lite]
           multipart/form-data, field "audio"

[Backend — not yet implemented beyond stub]
  → Calls Gemini Flash / Flash Lite
  → Streams SSE back:
      event: status   { message: string }
      event: segment  { …Segment fields… }   ← one per transcribed segment
      event: complete { …Transcript fields… } ← final full transcript replaces streaming segments
      event: error    { message: string }

[Frontend]
  → useTranscription: manages TranscriptFile[] library, streams via raw ReadableStream
  → usePlayback: RAF clock advancing currentTime × playbackSpeed each frame
  → SegmentView: active segment = last one where start_seconds ≤ currentTime
  → Word chips: hover → kana reading appears above the word
```

---

## Frontend Patterns & Gotchas

- **Styling lives in `globals.css`**, not in components. All reusable classes (`.ctrl-btn`, `.segment-line`, `.word-chip`, `.dot-grid`, etc.) and Tailwind v4 `@theme inline` tokens (`--color-*`, `--font-mono`) are defined there. Components use `style={{}}` for layout and one-off values; they reference CSS variables via `"var(--color-*)"` strings.
- **`@import` order in globals.css**: the Google Fonts `@import url(...)` must come _before_ `@import "tailwindcss"` — Tailwind v4/PostCSS enforces this.
- **No Geist fonts**: `layout.tsx` no longer imports `next/font/google`. IBM Plex Mono is loaded entirely from the Google Fonts `@import` in `globals.css`.
- **Mock data for development**: "MOCK 1" and "MOCK 2" buttons in the header load `/public/mock1.json` and `/public/mock2.json` directly in the browser (`fetch("/mock1.json")`). The mock files include an optional `label` field on `Speaker` that the backend schema doesn't require — the frontend renders it when present.
- **SSE parsing**: raw `ReadableStream` + `TextDecoder`, not `EventSource`. Blocks split on `\n\n`; each block parsed for `event:` / `data:` lines in `lib/utils.ts:parseSseBlock`.
- **Playback is timer-only** — there is no audio element. `usePlayback` uses `requestAnimationFrame` (not `setInterval`) to advance `currentTime` by wall-clock delta × speed. The active segment is determined by finding the last segment whose `start_seconds ≤ currentTime`.
- **`NEXT_PUBLIC_API_BASE_URL`**: set in `.env` to a Tailscale IP. Backend binds to port **8080**; the `.env` currently points to port **8000** — keep them consistent when running locally.
- **Next.js 16 / React 19**: breaking API changes vs. older training data. Check `node_modules/next/dist/docs/` before writing Next.js-specific patterns.
- **`total_duration_seconds`** drives the progress bar and caps playback. Streaming placeholder files set it to `0` until the `complete` event replaces the transcript.

---

## Backend Patterns & Gotchas

- **Cargo edition 2024** — ensure all Rust code is compatible.
- **Current state**: `main.rs` is a stub with one `GET /` route. Full `/api/v1/transcriptions/stream` is not implemented.
- **Dependencies declared but not wired**: `sqlx` (Neon PSQL), `thiserror`. No migrations or DB connection code yet.
- **Port**: always `0.0.0.0:8080` — enforced by `fly.toml` and Dockerfile EXPOSE.
- **Dockerfile copies workspace root**, so `cargo build --release` runs at workspace level. Binary: `transcriber-api`.
- **fly.io cold-starts**: `min_machines_running = 0`, `auto_stop_machines = 'stop'` — machine cold-starts on first request.

---

## Planned Work (TODO.md)

1. Personal notes / manual phrase highlighting for visual separation
2. Recognition and highlight (via toggle) of personally marked phrases
