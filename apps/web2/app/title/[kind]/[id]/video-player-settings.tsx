"use client";

import { useEffect, useRef, useState } from "react";

export type PlayerMenu = "settings" | "audio" | "subtitles" | "speed" | null;
export type TrackChoice = { id: number; label: string; language?: string };

const PLAYBACK_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

export function PlayerSettings({
  menu,
  audioTracks,
  subtitleTracks,
  selectedAudio,
  selectedSubtitle,
  playbackRate,
  onMenuChange,
  onAudio,
  onSubtitle,
  onPlaybackRate,
  onClose,
  error,
  busy,
}: {
  menu: Exclude<PlayerMenu, null>;
  audioTracks: TrackChoice[];
  subtitleTracks: TrackChoice[];
  selectedAudio: number;
  selectedSubtitle: number;
  playbackRate: number;
  onMenuChange: (menu: Exclude<PlayerMenu, null>) => void;
  onAudio: (id: number) => void;
  onSubtitle: (id: number) => void;
  onPlaybackRate: (rate: number) => void;
  onClose: () => void;
  error: string | null;
  busy: boolean;
}) {
  const [search, setSearch] = useState({ category: menu, text: "" });
  const query = search.category === menu ? search.text : "";
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const matches = (track: TrackChoice) =>
    track.label.toLocaleLowerCase().includes(normalizedQuery);
  const visibleAudioTracks = menu === "audio" ? audioTracks.filter(matches) : [];
  const visibleSubtitleTracks =
    menu === "subtitles" ? subtitleTracks.filter(matches) : [];
  const searchable =
    menu === "audio"
      ? audioTracks.length > 6
      : menu === "subtitles" && subtitleTracks.length > 6;
  const panelRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const opener = document.activeElement as HTMLElement | null;
    return () => {
      if (opener?.isConnected) opener.focus();
    };
  }, []);

  useEffect(() => {
    panelRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
  }, [menu]);

  useEffect(() => {
    const dismiss = (event: Event) => {
      const target = event.target as HTMLElement;
      if (
        !panelRef.current?.contains(target) &&
        !target.closest('button[aria-label="Playback settings"]')
      ) onClose();
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [onClose]);

  const categories = [
    {
      key: "audio",
      label: "Audio",
      value: splitTrackLabel(
        audioTracks.find((track) => track.id === selectedAudio)?.label ?? "Default",
      )[0],
      icon: AudioIcon,
    },
    {
      key: "subtitles",
      label: "Subtitles",
      value:
        selectedSubtitle === -1
          ? "Off"
          : splitTrackLabel(
              subtitleTracks.find((track) => track.id === selectedSubtitle)?.label ?? "On",
            )[0],
      icon: CaptionsIcon,
    },
    {
      key: "speed",
      label: "Speed",
      value: playbackRate === 1 ? "Normal" : `${playbackRate}×`,
      icon: SpeedIcon,
    },
  ] as const;
  const title = categories.find((category) => category.key === menu)?.label;

  return (
    <section
      ref={panelRef}
      role="dialog"
      aria-modal="true"
      className="player-settings absolute right-4 bottom-[calc(100%+0.75rem)] left-4 overflow-hidden rounded-2xl border border-line bg-panel/70 p-1.5 shadow-[0_8px_32px_#0005] backdrop-blur-xl sm:right-7 sm:left-auto sm:w-80 lg:right-10"
      aria-label="Playback settings"
      onKeyDown={(event) => {
        const target = event.target as HTMLElement;
        if (event.key === "Tab") {
          const focusable = Array.from(
            event.currentTarget.querySelectorAll<HTMLElement>(
              "button:not(:disabled), input:not(:disabled)",
            ),
          );
          const first = focusable[0];
          const last = focusable.at(-1);
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last?.focus();
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first?.focus();
          }
          return;
        }
        if (target.matches("input")) return;
        if (event.key === "ArrowLeft" && menu !== "settings") {
          event.preventDefault();
          onMenuChange("settings");
          return;
        }
        if (event.key === "ArrowRight" && menu === "settings") {
          event.preventDefault();
          target.closest("button")?.click();
          return;
        }
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) return;
        event.preventDefault();
        const buttons = Array.from(
          event.currentTarget.querySelectorAll<HTMLButtonElement>(
            "button:not(:disabled)",
          ),
        );
        const index = buttons.indexOf(
          target.closest("button") as HTMLButtonElement,
        );
        const next =
          event.key === "Home"
            ? 0
            : event.key === "End"
              ? buttons.length - 1
              : (index + (event.key === "ArrowDown" ? 1 : -1) + buttons.length) %
                buttons.length;
        buttons[next]?.focus();
      }}
    >
      {menu === "settings" ? (
        <div className="py-0.5">
          {categories.map(({ key, label, value, icon: Icon }) => (
            <button
              key={key}
              type="button"
              aria-label={label}
              onClick={() => onMenuChange(key)}
              className="flex min-h-12 w-full items-center gap-3 rounded-xl px-3 text-left transition-colors hover:bg-white/8 focus-visible:outline-offset-[-2px]"
            >
              <Icon className="size-[18px] shrink-0 text-white/75" />
              <span className="text-sm font-medium text-ink">{label}</span>
              <span className="ml-auto max-w-28 truncate text-[13px] text-muted">{value}</span>
              <ChevronIcon className="size-3.5 shrink-0 text-white/35" />
            </button>
          ))}
        </div>
      ) : (
        <>
          <div className="mb-1 border-b border-white/8 pb-1">
            <button
              type="button"
              aria-label="Back to settings"
              onClick={() => onMenuChange("settings")}
              className="flex min-h-10 w-full items-center gap-2 rounded-xl px-2 text-sm font-semibold text-white/90 transition-colors hover:bg-white/5 focus-visible:outline-offset-[-2px]"
            >
              <ChevronIcon className="size-4 rotate-180 text-white/55" />
              {title}
            </button>
          </div>
          <div aria-busy={busy} className="player-settings-content flex min-h-0 min-w-0 flex-col">
            {busy ? <span role="status" className="block shrink-0 px-3 py-2 text-xs text-white/60">Switching…</span> : null}
            {searchable ? (
              <input
                type="search"
                aria-label="Search tracks"
                placeholder="Search languages"
                value={query}
                onChange={(event) =>
                  setSearch({ category: menu, text: event.currentTarget.value })
                }
                className="mx-1 my-1 h-9 shrink-0 rounded-lg border-0 bg-black/20 px-3 text-[13px] text-white placeholder:text-white/35 focus-visible:outline-offset-[-2px]"
              />
            ) : null}
            <div key={menu} className="player-settings-options min-h-0 overflow-y-auto overscroll-contain">
              {query && !(menu === "audio" ? visibleAudioTracks : visibleSubtitleTracks).length ? (
                <p role="status" className="px-2 py-3 text-sm text-white/50">No matching tracks.</p>
              ) : null}
              {error ? <p className="mb-2 rounded-lg bg-red-400/10 px-3 py-2 text-xs leading-5 text-red-200" role="alert">{error}</p> : null}
              {menu === "audio" ? (
                audioTracks.length ? visibleAudioTracks.map((track) => (
                  <Choice key={track.id} selected={selectedAudio === track.id} label={track.label} disabled={busy} onClick={() => onAudio(track.id)} />
                )) : <EmptyTrackState label="The stream uses its default audio track." />
              ) : null}
              {menu === "subtitles" ? (
                <>
                  <Choice selected={selectedSubtitle === -1} label="Off" disabled={busy} onClick={() => onSubtitle(-1)} />
                  {subtitleTracks.length ? visibleSubtitleTracks.map((track) => (
                    <Choice key={track.id} selected={selectedSubtitle === track.id} label={track.label} disabled={busy} onClick={() => onSubtitle(track.id)} />
                  )) : <EmptyTrackState label="No subtitle tracks are available." />}
                </>
              ) : null}
              {menu === "speed" ? PLAYBACK_RATES.map((rate) => (
                <Choice key={rate} selected={playbackRate === rate} label={rate === 1 ? "Normal" : `${rate}×`} disabled={false} onClick={() => onPlaybackRate(rate)} />
              )) : null}
            </div>
          </div>
        </>
      )}
    </section>
  );
}

function splitTrackLabel(label: string) {
  const parts = label.split(/\s+(?:-|·|–|—)\s+/);
  if (parts.length > 1 && /^(SDH|forced|CC)$/i.test(parts[0])) {
    [parts[0], parts[1]] = [parts[1], parts[0]];
  }
  return parts.filter(
    (part, index) =>
      index === 0 || !/^(SUBRIP|SRT|ASS|SSA|WEBVTT|VTT|PGSSUB)$/i.test(part),
  );
}

function Choice({ selected, label, disabled, onClick }: { selected: boolean; label: string; disabled: boolean; onClick: () => void }) {
  const [title, ...metadata] = splitTrackLabel(label);
  const description = metadata.join(" · ");
  return (
    <button type="button" aria-label={label} aria-pressed={selected} disabled={disabled} className="flex min-h-10 w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left transition-colors hover:bg-white/8 focus-visible:outline-offset-[-2px] disabled:cursor-wait disabled:opacity-50" onClick={onClick}>
      <span className="min-w-0 flex-1"><span className={`block text-sm font-medium [overflow-wrap:anywhere] ${selected ? "text-white" : "text-white/70"}`}>{title}</span>{description ? <span className="mt-1 block text-[11px] leading-4 text-white/40 [overflow-wrap:anywhere]">{description}</span> : null}</span>
      {selected ? <CheckIcon className="size-4 shrink-0 text-accent" /> : null}
    </button>
  );
}

function EmptyTrackState({ label }: { label: string }) {
  return <p className="px-2 py-3 text-sm leading-6 text-white/45">{label}</p>;
}

type IconProps = { className?: string };
function CheckIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 16 16" strokeWidth="2.2"><path d="m3 8.5 3 3 7-7" /></svg>; }
function ChevronIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 16 16" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round"><path d="m6 3 5 5-5 5" /></svg>; }
function AudioIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d="M4 14v-3a8 8 0 0 1 16 0v3"/><rect x="3" y="12" width="4" height="8" rx="2"/><rect x="17" y="12" width="4" height="8" rx="2"/></svg>; }
function CaptionsIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><rect x="2" y="5" width="20" height="14" rx="3"/><path d="M10 10a2.5 2.5 0 1 0 0 4m8-4a2.5 2.5 0 1 0 0 4"/></svg>; }
function SpeedIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><circle cx="12" cy="12" r="9"/><path d="M12 6v6l4 2"/></svg>; }
