"use client";
import { useEffect, useRef } from "react";
import { ASCII_LOGO, ASCII_SPINNER } from "../constants";

type AnimationMode = "idle" | "streaming" | "done";
type Props = { mode: AnimationMode };

const LOGO_ROWS = ASCII_LOGO.split("\n");

const JOYFUL_COLORS = [
  "#ff6b9d",
  "#45d4fa",
  "#7fff8a",
  "#ffe566",
  "#c77dff",
  "#00f5d4",
  "#ff9f43",
  "#f8f8f2",
  "#ff4d6d",
  "#48cae4",
  "#b5e48c",
  "#ffd166",
];

const ISO_CHARS = "╱╲╳▲▼◆◇▸▾◈";
const GLITCH_CHARS = "!@#$%&*░▒▓█▄▀■□▪▫";

export default function LogoAnimation({ mode }: Props) {
  const preRef = useRef<HTMLPreElement | null>(null);
  const frameRef = useRef<number | null>(null);
  const scanlineRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const idleGlitchRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const spinnerTickRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const bounceRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const spinnerIdxRef = useRef(0);
  const colorCycleRef = useRef(0);
  const isoOffsetRef = useRef(0);

  const getSpans = () =>
    preRef.current
      ? Array.from(preRef.current.querySelectorAll<HTMLSpanElement>(".logo-char"))
      : [];

  const resetChars = () => {
    getSpans().forEach((span) => {
      span.textContent = span.dataset.orig ?? " ";
      span.style.filter = "";
      span.style.color = "";
      span.style.textShadow = "";
      span.style.transform = "";
      span.style.display = "inline";
    });
  };

  // Isometric shimmer: diagonal color wave
  const runIsoShimmer = () => {
    const spans = getSpans();
    const flat = LOGO_ROWS.join("\n");
    spans.forEach((span, i) => {
      const ch = flat[i] ?? " ";
      if (ch === " " || ch === "\n") return;

      // Compute row/col for diagonal wave
      let row = 0,
        col = 0,
        cur = 0;
      for (let r = 0; r < LOGO_ROWS.length; r++) {
        if (cur + LOGO_ROWS[r].length + 1 > i) {
          col = i - cur;
          row = r;
          break;
        }
        cur += LOGO_ROWS[r].length + 1;
      }

      const diag = row + col + isoOffsetRef.current;
      const colorIdx = Math.floor(diag * 0.5) % JOYFUL_COLORS.length;
      const brightness = 0.7 + 0.6 * Math.sin((diag * Math.PI) / 6);
      span.style.color = JOYFUL_COLORS[(colorIdx + JOYFUL_COLORS.length) % JOYFUL_COLORS.length];
      span.style.filter = `brightness(${brightness.toFixed(2)})`;
    });
    isoOffsetRef.current = (isoOffsetRef.current + 1) % (JOYFUL_COLORS.length * 6);
  };

  const scheduleIdleGlitch = () => {
    const delay = 1800 + Math.random() * 2500;
    idleGlitchRef.current = setTimeout(() => {
      const spans = getSpans().filter((s) => s.textContent?.trim() && s.dataset.orig?.trim());
      if (spans.length) {
        const count = 1 + Math.floor(Math.random() * 3);
        for (let k = 0; k < count; k++) {
          const target = spans[Math.floor(Math.random() * spans.length)];
          const origText = target.dataset.orig ?? target.textContent ?? " ";
          const color = JOYFUL_COLORS[Math.floor(Math.random() * JOYFUL_COLORS.length)];
          const isoChar = ISO_CHARS[Math.floor(Math.random() * ISO_CHARS.length)];

          target.textContent = isoChar;
          target.style.color = color;
          target.style.textShadow = `0 0 8px ${color}, 0 0 16px ${color}`;

          const restore = () => {
            if (target) {
              target.textContent = origText;
              target.style.textShadow = "";
            }
          };
          setTimeout(restore, 300 + Math.random() * 200);
        }
      }
      scheduleIdleGlitch();
    }, delay);
  };

  useEffect(() => {
    if (!preRef.current) return;
    const flat = LOGO_ROWS.join("\n");
    preRef.current.innerHTML = flat
      .split("")
      .map(
        (ch, i) =>
          `<span class="logo-char" data-orig="${ch === '"' ? "&quot;" : ch}" data-idx="${i}">${ch}</span>`,
      )
      .join("");

    return () => {
      if (frameRef.current) cancelAnimationFrame(frameRef.current);
      if (scanlineRef.current) clearInterval(scanlineRef.current);
      if (idleGlitchRef.current) clearTimeout(idleGlitchRef.current);
      if (spinnerTickRef.current) clearInterval(spinnerTickRef.current);
      if (bounceRef.current) clearInterval(bounceRef.current);
    };
  }, []);

  useEffect(() => {
    if (scanlineRef.current) {
      clearInterval(scanlineRef.current);
      scanlineRef.current = null;
    }
    if (idleGlitchRef.current) {
      clearTimeout(idleGlitchRef.current);
      idleGlitchRef.current = null;
    }
    if (spinnerTickRef.current) {
      clearInterval(spinnerTickRef.current);
      spinnerTickRef.current = null;
    }
    if (bounceRef.current) {
      clearInterval(bounceRef.current);
      bounceRef.current = null;
    }
    resetChars();

    if (mode === "idle") {
      // Gentle iso shimmer in idle
      scanlineRef.current = setInterval(() => {
        runIsoShimmer();
      }, 120);
      scheduleIdleGlitch();
    }

    if (mode === "streaming") {
      let scanRow = 0;
      const rows = LOGO_ROWS.length;

      scanlineRef.current = setInterval(() => {
        const spans = getSpans();
        let charOffset = 0;

        for (let r = 0; r < rows; r++) {
          const rowLen = LOGO_ROWS[r].length + 1;
          const isActive = r === scanRow % rows;
          const isNext = r === (scanRow + 1) % rows;

          for (let c = charOffset; c < charOffset + rowLen && c < spans.length; c++) {
            const span = spans[c];
            if (isActive) {
              colorCycleRef.current = (colorCycleRef.current + 1) % JOYFUL_COLORS.length;
              const col = JOYFUL_COLORS[colorCycleRef.current];
              span.style.color = col;
              span.style.filter = "brightness(2.4)";
              span.style.textShadow = `0 0 10px ${col}, 0 0 20px ${col}`;
            } else if (isNext) {
              span.style.filter = "brightness(1.3)";
              span.style.textShadow = "";
              span.style.color = JOYFUL_COLORS[(colorCycleRef.current + 3) % JOYFUL_COLORS.length];
            } else {
              // Soft iso tint on non-active rows
              const diag = r + (charOffset % 8) + isoOffsetRef.current;
              span.style.color = JOYFUL_COLORS[Math.abs(diag) % JOYFUL_COLORS.length];
              span.style.filter = "brightness(0.7)";
              span.style.textShadow = "";
            }
          }
          charOffset += rowLen;
        }

        scanRow++;
        isoOffsetRef.current = (isoOffsetRef.current + 1) % 48;
      }, 90);

      // Spinner swap
      spinnerTickRef.current = setInterval(() => {
        spinnerIdxRef.current = (spinnerIdxRef.current + 1) % ASCII_SPINNER.length;
        const spans = getSpans();
        const target = spans.find((s) => s.dataset.orig === "~");
        if (target) {
          target.textContent = ASCII_SPINNER[spinnerIdxRef.current];
          target.style.color = JOYFUL_COLORS[spinnerIdxRef.current % JOYFUL_COLORS.length];
        }
      }, 150);
    }

    if (mode === "done") {
      const spans = getSpans();
      const originals = spans.map((s) => s.dataset.orig ?? s.textContent ?? " ");

      // Phase 1: joyful character explosion — randomize all printable chars with color bursts
      spans.forEach((span, i) => {
        const ch = originals[i];
        if (ch === " " || ch === "\n") return;
        const delay = Math.random() * 200;
        setTimeout(() => {
          const glitch = GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
          const color = JOYFUL_COLORS[Math.floor(Math.random() * JOYFUL_COLORS.length)];
          span.textContent = glitch;
          span.style.color = color;
          span.style.textShadow = `0 0 12px ${color}`;
          span.style.filter = "brightness(2)";
        }, delay);
      });

      // Phase 2: isometric decrypt — column by column, left to right, with rainbow trail
      const cols = LOGO_ROWS.reduce((m, r) => Math.max(m, r.length), 0);
      const colDelay = 900 / cols;

      for (let col = 0; col < cols; col++) {
        setTimeout(
          () => {
            let offset = 0;
            for (let r = 0; r < LOGO_ROWS.length; r++) {
              const idx = offset + col;
              if (col < LOGO_ROWS[r].length && spans[idx]) {
                const diag = (r + col) % JOYFUL_COLORS.length;
                const color = JOYFUL_COLORS[diag];
                spans[idx].textContent = originals[idx];
                spans[idx].style.color = color;
                spans[idx].style.textShadow = `0 0 14px ${color}, 0 0 28px ${color}`;
                spans[idx].style.filter = "brightness(2.2)";

                // Settle to normal
                setTimeout(
                  () => {
                    if (spans[idx]) {
                      spans[idx].style.filter = "";
                      spans[idx].style.textShadow = "";
                      spans[idx].style.color = "";
                    }
                  },
                  350 + Math.random() * 150,
                );
              }
              offset += LOGO_ROWS[r].length + 1;
            }
          },
          250 + col * colDelay,
        );
      }

      // Phase 3: after settle, start joyful idle shimmer
      setTimeout(
        () => {
          scanlineRef.current = setInterval(runIsoShimmer, 120);
          scheduleIdleGlitch();
        },
        250 + cols * colDelay + 600,
      );
    }
  }, [mode]);

  return (
    <pre
      className="segment-empty-logo"
      ref={preRef}
      aria-hidden="true"
      style={{ letterSpacing: "0.05em", lineHeight: "1.25" }}
    />
  );
}
