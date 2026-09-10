import type { ReactNode } from "react";
import type { Episode, MediaCard, PlaybackProgress } from "../../../catalogue";
import { Eyebrow, NavigationHeader, primaryButtonClass } from "../../../ui";

export type Source = {
  id: string;
  provider: string;
  quality: string;
  detail: string;
  available: boolean;
};

export type ActivePlayback = {
  sourceId: string;
  sourceLabel: string;
  sourceDetail: string;
  resumeSeconds?: number;
  episode?: Episode;
};

export const exampleSources: Source[] = [
  {
    id: "stremio-1",
    provider: "Stremio · 1",
    quality: "4K · HDR · 5.1",
    detail: "12.4 GB",
    available: true,
  },
  {
    id: "stremio-2",
    provider: "Stremio · 2",
    quality: "1080p · H.264 · 5.1",
    detail: "3.8 GB",
    available: true,
  },
  {
    id: "stremio-3",
    provider: "Stremio · 3",
    quality: "720p · H.264 · Stereo",
    detail: "1.2 GB",
    available: true,
  },
];

const jellyfinSource: Source = {
  id: "local",
  provider: "Jellyfin",
  quality: "Local library",
  detail: "In library",
  available: true,
};

export function PlayerView({
  media,
  playback,
  onBack,
}: {
  media: MediaCard;
  playback: ActivePlayback;
  onBack: () => void;
}) {
  return (
    <FullScreenShell onBack={onBack} backLabel="Back">
      <main className="grid min-h-[calc(100dvh-5rem)] place-items-center bg-black px-5 py-10">
        <div className="text-center">
          <span className="mx-auto grid size-20 place-items-center rounded-full bg-accent text-background shadow-[0_0_0_14px_#f4bc521c]">
            <PlayIcon />
          </span>
          <div className="mt-8">
            <Eyebrow>Now playing</Eyebrow>
          </div>
          <h1 className="mt-[-0.5rem] text-3xl font-semibold tracking-tight sm:text-5xl">
            {playback.episode ? playback.episode.title : media.title}
          </h1>
          {playback.episode ? (
            <p className="mt-3 text-sm text-muted">
              {media.title} · S{playback.episode.seasonNumber} E
              {playback.episode.episodeNumber}
            </p>
          ) : null}
          <p className="mt-6 font-semibold">
            {playback.resumeSeconds
              ? `Resuming near ${formatTime(playback.resumeSeconds)}`
              : "Starting from the beginning"}
          </p>
          <p className="mt-2 text-sm text-muted">
            {playback.sourceLabel} · {playback.sourceDetail}
          </p>
          <p className="mt-8 text-xs text-muted">
            Playback preview only. The player endpoint is not connected yet.
          </p>
        </div>
      </main>
    </FullScreenShell>
  );
}

export function FullScreenShell({
  children,
  onBack,
  backLabel,
}: {
  children: ReactNode;
  onBack: () => void;
  backLabel: string;
}) {
  return (
    <div className="min-h-dvh bg-[radial-gradient(ellipse_at_50%_0%,#233336_0%,transparent_48%)]">
      <NavigationHeader onBack={onBack} label={backLabel} />
      {children}
    </div>
  );
}

export function ProgressBar({
  progress,
  className = "",
}: {
  progress: PlaybackProgress;
  className?: string;
}) {
  const percent = Math.min(
    100,
    Math.max(
      0,
      (progress.positionSeconds / progress.durationSeconds) * 100,
    ),
  );
  return (
    <span
      className={`block h-1 bg-white/25 ${className}`}
      aria-hidden="true"
    >
      <span
        className="block h-full bg-accent"
        style={{ width: `${percent}%` }}
      />
    </span>
  );
}

export function PlaybackControl({
  sources,
  selected,
  progress,
  disabled,
  onPlay,
}: {
  sources: Source[];
  selected: Source;
  progress: PlaybackProgress | null;
  disabled: boolean;
  onPlay: (source: Source, resumeSeconds?: number) => void;
}) {
  const resumeSeconds = progress?.positionSeconds;
  return (
    <div className="relative flex">
      <button
        type="button"
        className={`${primaryButtonClass} rounded-r-none border-r-0 pr-4`}
        disabled={disabled}
        onClick={() => onPlay(selected, resumeSeconds)}
      >
        <PlayIcon /> {progress ? `Resume · ${formatRemaining(progress)}` : "Play"}
      </button>
      <details className="group relative">
        <summary
          aria-label="Choose playback source"
          className={`flex min-h-11 list-none items-center rounded-r-full border border-accent bg-accent px-3 text-background transition-colors marker:hidden hover:bg-amber-300 [&::-webkit-details-marker]:hidden ${disabled ? "pointer-events-none opacity-50" : ""}`}
        >
          <ChevronIcon />
        </summary>
        <div className="absolute top-[calc(100%+0.65rem)] left-0 z-30 w-[min(22rem,calc(100vw-2.5rem))] overflow-hidden rounded-2xl border border-white/15 bg-[#11151a]/98 p-2 shadow-[0_24px_70px_#000000aa] backdrop-blur-2xl">
          <p className="px-3 pt-2 pb-3 text-xs text-muted">
            {resumeSeconds
              ? `Resume near ${formatTime(resumeSeconds)} with`
              : "Play with"}
          </p>
          {sources.map((source, index) => (
            <button
              key={source.id}
              type="button"
              aria-label={
                resumeSeconds
                  ? `Resume with ${source.provider} near ${formatTime(resumeSeconds)}`
                  : `Play with ${source.provider}`
              }
              disabled={!source.available}
              className={`flex min-h-16 w-full items-center gap-3 rounded-xl px-3 text-left hover:bg-white/8 disabled:opacity-40 ${selected.id === source.id ? "bg-white/6" : ""}`}
              onClick={(event) => {
                onPlay(source, resumeSeconds);
                event.currentTarget.closest("details")?.removeAttribute("open");
              }}
            >
              <span className="grid size-7 shrink-0 place-items-center rounded-full border border-line font-mono text-[10px] text-accent">
                {source.id === "local"
                  ? "L"
                  : (source.provider.match(/\d+$/)?.[0] ?? index + 1)}
              </span>
              <span className="grid min-w-0 flex-1 gap-0.5">
                <strong className="text-sm">{source.provider}</strong>
                <span className="truncate text-xs text-muted">
                  {source.quality}
                </span>
              </span>
              <span className="text-xs text-muted">{source.detail}</span>
            </button>
          ))}
        </div>
      </details>
    </div>
  );
}

export function PlaybackHint({
  progress,
  lastSourceAvailable,
  resumeSourceLabel,
}: {
  progress: PlaybackProgress | null;
  lastSourceAvailable: boolean;
  resumeSourceLabel: string | null;
}) {
  if (progress && lastSourceAvailable) {
    return (
      <p className="mt-4 text-xs leading-5 text-muted">
        Resume uses {progress.lastSourceLabel}. Choose another stream source to
        resume at approximately the same point.
      </p>
    );
  }
  if (progress) {
    return (
      <p className="mt-4 text-xs leading-5 text-amber-200/80">
        {progress.lastSourceLabel} is no longer available. Resume will use{" "}
        {resumeSourceLabel} at approximately the same point.
      </p>
    );
  }
  return null;
}

export function playbackSources(hasLocalCopy: boolean) {
  return hasLocalCopy ? [jellyfinSource, ...exampleSources] : exampleSources;
}

export function preferredPlaybackSource(
  progress: PlaybackProgress | null,
  hasLocalCopy: boolean,
  fallback: Source,
) {
  const sources = playbackSources(hasLocalCopy);
  if (progress) {
    const lastSource = sources.find(
      (source) => source.id === progress.lastSourceId && source.available,
    );
    if (lastSource) return lastSource;
  }
  return hasLocalCopy ? jellyfinSource : fallback;
}

export function isLastSourceAvailable(
  progress: PlaybackProgress | null,
  hasLocalCopy: boolean,
) {
  if (!progress) return false;
  return progress.lastSourceKind === "local"
    ? hasLocalCopy
    : exampleSources.some(
        (source) => source.id === progress.lastSourceId && source.available,
      );
}

export function formatRemaining(progress: PlaybackProgress) {
  const minutes = Math.max(
    1,
    Math.ceil(
      (progress.durationSeconds - progress.positionSeconds) / 60,
    ),
  );
  return `${minutes} min left`;
}

export function formatTime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h ${minutes}m` : `${minutes}m`;
}

export function DownloadIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4 fill-none stroke-current"
      viewBox="0 0 16 16"
      strokeWidth="1.5"
    >
      <path d="M8 2v8m-3-3 3 3 3-3M3 13.5h10" />
    </svg>
  );
}

function PlayIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4 fill-current"
      viewBox="0 0 16 16"
    >
      <path d="M3.5 2.2a1 1 0 0 1 1.5-.86l9 5.8a1 1 0 0 1 0 1.72l-9 5.8a1 1 0 0 1-1.5-.86V2.2Z" />
    </svg>
  );
}

function ChevronIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-3.5 fill-none stroke-current transition-transform group-open:rotate-180"
      viewBox="0 0 16 16"
      strokeWidth="2"
    >
      <path d="m4 6 4 4 4-4" />
    </svg>
  );
}
