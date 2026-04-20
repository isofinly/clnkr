CREATE TABLE transcriptions (
    audio_signature  TEXT NOT NULL,
    transcript_type  TEXT NOT NULL CHECK (transcript_type IN ('simple', 'complex')),
    response_json    JSONB NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (audio_signature, transcript_type)
);

CREATE TABLE translations (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           TEXT NOT NULL,
    input_hash        TEXT NOT NULL,     -- hex(SHA-256(translation_input || context))
    input_json        JSONB NOT NULL,    -- AnalysisInput (without segment_id — that's routing metadata)
    response_json     JSONB NOT NULL,    -- AnalysisOutput
    served_from_cache BOOLEAN NOT NULL DEFAULT false,
    origin_audio_sig  TEXT,             -- backend-resolved from active transcription; null = standalone
    origin_segment_id BIGINT,           -- from AnalysisInputRequest.segment_id; null = standalone
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX uq_translations_user_input ON translations(user_id, input_hash);
CREATE INDEX idx_translations_user ON translations(user_id);
CREATE INDEX idx_translations_origin ON translations(origin_audio_sig);


CREATE TABLE notes (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         TEXT NOT NULL,
    input_hash      TEXT NOT NULL,
    note_text       TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX uq_notes_user_input ON notes(user_id, input_hash);
CREATE INDEX idx_notes_user ON notes(user_id);


CREATE TABLE request_log (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         TEXT NOT NULL,
    request_type    TEXT NOT NULL,
    request_meta    JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_request_log_user ON request_log(user_id);
