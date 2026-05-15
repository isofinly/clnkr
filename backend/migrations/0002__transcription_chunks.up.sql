-- Chunk-level storage for long-audio transcription pipeline.
-- Each row stores the segments and speakers from one ~140-second chunk,
-- plus the LLM-generated summary used as context for the next chunk.

CREATE TABLE transcription_chunks (
    id              SERIAL PRIMARY KEY,
    audio_signature TEXT NOT NULL,
    chunk_index     INT NOT NULL,
    start_seconds   FLOAT NOT NULL,
    end_seconds     FLOAT NOT NULL,
    segments_json   JSONB NOT NULL DEFAULT '[]',
    speakers_json   JSONB NOT NULL DEFAULT '[]',
    summary         TEXT,
    stitched        BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(audio_signature, chunk_index)
);

CREATE INDEX idx_chunks_audio ON transcription_chunks(audio_signature, chunk_index);
