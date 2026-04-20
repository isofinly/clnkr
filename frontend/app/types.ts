export type Word = {
  text: string;
  reading: string;
  romanization: string;
};

export type Speaker = {
  speaker_id: string;
  label?: string;
};

export type RawSegment = {
  id: number;
  start_seconds: number;
  end_seconds: number;
  raw_text: string;
  words?: Word[];
  translation: string;
  speaker?: Speaker;
  speaker_id?: string;
  isStreaming?: boolean;
  streamId?: string;
  pendingTranslation?: boolean;
};

export type Segment = Omit<RawSegment, 'speaker' | 'speaker_id' | 'words'> & {
  speaker: Speaker;
  words: Word[];
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
  wordsMode?: boolean;
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

export type LibraryTab = "transcriptions" | "translations";

// ── Translation types ──────────────────────────────────────

export type AnalysisInputRequest = {
  translation_input: string;
  context?: string;
  segment_id: number;
};

export type Token = {
  token: string;
  meaning: string;
};

export type PhraseBreakdown = {
  phrase: string;
  tokens: Token[];
};

export type GrammarConstruction = {
  name: string;
  pattern?: string;
  description: string;
};

export type KanjiWord = {
  kanji: string;
  reading: string;
};

export type TranslationOutput = {
  source_text: string;
  phrase_breakdowns: PhraseBreakdown[];
  grammar_constructions: GrammarConstruction[];
  kanji_words: KanjiWord[];
  translations: string[];
};

export type TranslationEntry = {
  id: string;
  input_hash: string;
  input_json: { translation_input: string; context?: string };
  response_json: TranslationOutput;
  cached: boolean;
  orphaned: boolean;
  origin_segment_id: number | null;
  created_at: string;
  note?: string;
};
