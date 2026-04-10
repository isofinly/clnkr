"use client";
import { useState, useEffect, useRef, useCallback } from "react";
import { PLAYBACK_SPEEDS } from "../constants";

export function usePlayback(totalDuration: number) {
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [playbackSpeed, setPlaybackSpeed] = useState(1.0);

  const lastUpdateRef = useRef(performance.now());
  const requestRef = useRef<number | null>(null);

  const updateTime = useCallback(
    (now: number) => {
      if (isPlaying) {
        const delta = (now - lastUpdateRef.current) / 1000;
        setCurrentTime((prev) => {
          const next = prev + delta * playbackSpeed;
          return totalDuration > 0 ? Math.min(next, totalDuration) : next;
        });
      }
      lastUpdateRef.current = now;
      requestRef.current = requestAnimationFrame(updateTime);
    },
    [isPlaying, playbackSpeed, totalDuration],
  );

  useEffect(() => {
    lastUpdateRef.current = performance.now();
    requestRef.current = requestAnimationFrame(updateTime);
    return () => {
      if (requestRef.current) cancelAnimationFrame(requestRef.current);
    };
  }, [updateTime]);

  const togglePlay = () => setIsPlaying((p) => !p);

  const cycleSpeed = () =>
    setPlaybackSpeed(
      (prev) =>
        PLAYBACK_SPEEDS[(PLAYBACK_SPEEDS.indexOf(prev) + 1) % PLAYBACK_SPEEDS.length],
    );

  const jumpToTime = (time: number) => setCurrentTime(time);

  const reset = () => {
    setCurrentTime(0);
    setIsPlaying(false);
  };

  return { isPlaying, currentTime, playbackSpeed, togglePlay, cycleSpeed, jumpToTime, reset };
}
