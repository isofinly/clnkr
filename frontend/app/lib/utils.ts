import { SPEAKER_COLORS } from "../constants";
import { RawSegment, Segment, Transcript } from "../types";

// Accepts both the canonical shape ({ speaker: { speaker_id, label? } })
// and the flat shape ({ speaker_id: string }) used by mock3 / older backend output.
export function normaliseSegment(raw: RawSegment): Segment {
  const speaker =
    raw.speaker ??
    { speaker_id: raw.speaker_id ?? "s0" };
  return {
    ...raw,
    speaker,
    words: raw.words ?? [],
  };
}

export function normaliseTranscript(t: Transcript): Transcript {
  return {
    ...t,
    segments: (t.segments as unknown as RawSegment[]).map(normaliseSegment),
  };
}

export function getSpeakerColor(speakerId: string): string {
  const num = parseInt(speakerId.replace(/\D/g, ""), 10);
  return SPEAKER_COLORS[(isNaN(num) ? 0 : num) % SPEAKER_COLORS.length];
}

export function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60).toString().padStart(2, "0");
  const s = Math.floor(seconds % 60).toString().padStart(2, "0");
  const ms = Math.floor((seconds % 1) * 100).toString().padStart(2, "0");
  return `${m}:${s}.${ms}`;
}

export function parseSseBlock(block: string): { event: string; data: Record<string, unknown> } | null {
  const lines = block.split("\n");
  let event = "message";
  const dataLines: string[] = [];

  for (const line of lines) {
    if (line.startsWith("event:")) event = line.slice(6).trim();
    if (line.startsWith("data:")) dataLines.push(line.slice(5).trim());
  }

  if (!dataLines.length) return null;
  try {
    return { event, data: JSON.parse(dataLines.join("\n")) };
  } catch {
    return null;
  }
}
