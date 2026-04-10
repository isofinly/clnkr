export type Word = {
  text: string;
  reading: string;
  romanization: string;
};

export type Speaker = {
  speaker_id: string;
  label?: string;
};

export type Segment = {
  id: number;
  start_seconds: number;
  end_seconds: number;
  raw_text: string;
  words: Word[];
  translation: string;
  speaker: Speaker;
  isStreaming?: boolean;
  streamId?: string;
};

export type Transcript = {
  source_language: string;
  target_language: string;
  total_duration_seconds: number;
  speakers: Speaker[];
  segments: Segment[];
};

export type TranscriptFile = {
  name: string;
  transcript: Transcript;
};

export type StreamStatus =
  | "idle"
  | "uploading"
  | "transcribing"
  | "done"
  | "error"
  | string;

export type SseEvent = {
  event: string;
  data: Record<string, unknown>;
};
