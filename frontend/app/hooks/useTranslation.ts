"use client";
import { useState, useCallback, useRef } from "react";
import { TranslationEntry, AnalysisInputRequest, TranslationOutput } from "../types";
import { API_BASE_URL } from "../constants";
import { authHeaders, clearToken } from "../lib/auth";

export function useTranslation(onUnauthorized: () => void) {
  const [entries, setEntries] = useState<TranslationEntry[]>([]);
  const [activeEntry, setActiveEntry] = useState<TranslationEntry | null>(null);
  const [pendingSegmentId, setPendingSegmentId] = useState<number | null>(null);
  const [isTranslating, setIsTranslating] = useState(false);
  // Ref mirrors isTranslating so the stable useCallback closure can read the
  // latest value without adding isTranslating to its dependency array.
  const isTranslatingRef = useRef(false);

  const handleUnauth = (status: number) => {
    if (status === 401) { clearToken(); onUnauthorized(); return true; }
    return false;
  };

  const translate = useCallback(async (req: AnalysisInputRequest, force = false) => {
    // Guard: drop duplicate requests while one is already in flight.
    if (isTranslatingRef.current) return;

    isTranslatingRef.current = true;
    setPendingSegmentId(req.segment_id);
    setIsTranslating(true);
    try {
      const url = force
        ? `${API_BASE_URL}/api/v1/translations?force=true`
        : `${API_BASE_URL}/api/v1/translations`;

      const res = await fetch(url, {
        method: "POST",
        headers: { "Content-Type": "application/json", ...authHeaders() },
        body: JSON.stringify(req),
      });

      if (handleUnauth(res.status)) return;
      if (!res.ok) return;

      const body = await res.json();
      const translation = body.translation;
      const cached: boolean = body.served_from_cache ?? false;

      const serverHash: string = body.input_hash;

      const entry: TranslationEntry = {
        id: crypto.randomUUID(),
        input_hash: serverHash,
        input_json: { translation_input: req.translation_input, context: req.context },
        response_json: translation,
        cached,
        orphaned: false,
        origin_segment_id: req.segment_id || null,
        created_at: new Date().toISOString(),
      };

      setEntries((prev) => {
        const existing = prev.findIndex(
          (e) => e.input_json.translation_input === req.translation_input,
        );
        if (existing !== -1) {
          const updated = [...prev];
          updated[existing] = { ...entry, id: prev[existing].id };
          return updated;
        }
        return [entry, ...prev];
      });
      setActiveEntry(entry);
    } finally {
      isTranslatingRef.current = false;
      setIsTranslating(false);
      setPendingSegmentId(null);
    }
  }, []);

  const retranslate = useCallback(
    async (entry: TranslationEntry) => {
      await translate(
        {
          translation_input: entry.input_json.translation_input,
          context: entry.input_json.context,
          segment_id: entry.origin_segment_id ?? 0,
        },
        true,
      );
    },
    [translate],
  );

  const deleteEntry = (id: string) => {
    setEntries((prev) => prev.filter((e) => e.id !== id));
    setActiveEntry((cur) => (cur?.id === id ? null : cur));
  };

  const clearAll = () => {
    setEntries([]);
    setActiveEntry(null);
  };

  const openEntry = (entry: TranslationEntry) => setActiveEntry(entry);
  const closePanel = () => setActiveEntry(null);

  const fetchLibrary = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE_URL}/api/v1/user/translations`, {
        headers: authHeaders(),
      });
      if (res.status === 401) { clearToken(); onUnauthorized(); return; }
      if (!res.ok) return;
      const body = await res.json();
      const loaded: TranslationEntry[] = (body.translations ?? []).map(
        (entry: { response_json: TranslationOutput; input_hash: string }) => ({
          id: crypto.randomUUID(),
          input_hash: entry.input_hash,
          input_json: { translation_input: (entry.response_json as TranslationOutput).source_text, context: undefined },
          response_json: entry.response_json,
          cached: true,
          orphaned: false,
          origin_segment_id: null,
          created_at: new Date().toISOString(),
        }),
      );
      if (loaded.length > 0) setEntries(loaded);
    } catch {
      // non-fatal
    }
  }, [onUnauthorized]);

  return {
    entries,
    activeEntry,
    pendingSegmentId,
    isTranslating,
    translate,
    retranslate,
    fetchLibrary,
    deleteEntry,
    clearAll,
    openEntry,
    closePanel,
  };
}
