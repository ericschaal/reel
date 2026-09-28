"use client";

import { useSyncExternalStore, type CSSProperties } from "react";
import type { TimelineStore } from "./video-player-state";

function useTimeline(store: TimelineStore) {
  return useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot);
}

export function PlayerTimeline({
  store,
  onSeek,
}: {
  store: TimelineStore;
  onSeek: (seconds: number) => void;
}) {
  const { currentTime, duration } = useTimeline(store);
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  return (
    <>
      <div className="mb-2 flex items-center justify-between text-xs font-medium text-white/70">
        <span>{formatPlayerTime(currentTime)}</span>
        <span>-{formatPlayerTime(Math.max(0, duration - currentTime))}</span>
      </div>
      <input
        type="range"
        className="player-range player-progress-range w-full"
        min="0"
        max={duration || 0}
        step="0.1"
        value={Math.min(currentTime, duration || 0)}
        style={{ "--range-progress": `${progress}%` } as CSSProperties}
        aria-label="Seek through video"
        aria-valuetext={`${formatPlayerTime(currentTime)} of ${formatPlayerTime(duration)}`}
        onChange={(event) => onSeek(Number(event.currentTarget.value))}
      />
    </>
  );
}

export function PlayerTimeSummary({ store }: { store: TimelineStore }) {
  const { currentTime, duration } = useTimeline(store);
  return (
    <span className="ml-1 hidden text-xs font-medium text-white/65 md:block">
      {formatPlayerTime(currentTime)} / {formatPlayerTime(duration)}
    </span>
  );
}

function formatPlayerTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const whole = Math.floor(seconds);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const remaining = whole % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}
