import { SPEAKER_COLORS } from "../constants";

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
