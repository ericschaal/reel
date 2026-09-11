import { queryOptions } from "@tanstack/react-query";
import type { ReactNode } from "react";
import type { Episode, TitleMedia, PlaybackProgress } from "../../../catalogue";
import { Dialog } from "../../../dialog";
import {
  buttonClass,
  Eyebrow,
  NavigationHeader,
  primaryButtonClass,
} from "../../../ui";
import { ReelVideoPlayer } from "./video-player";

export type PlaybackDescriptor = {
  sessionId: string;
  source: "jellyfin" | "aioStreams";
  delivery: "direct" | "hls";
  mediaUrl: string;
  container: string | null;
  durationSeconds: number | null;
  audioTracks: PlaybackTrack[];
  subtitleTracks: PlaybackTrack[];
  selectedAudioIndex: number | null;
  selectedSubtitleIndex: number | null;
};

export type PlaybackTrack = {
  index: number;
  label: string;
  language: string | null;
  codec: string | null;
  isDefault: boolean;
  isForced: boolean;
};

export type PlaybackTrackSelection = {
  audioStreamIndex?: number;
  subtitleStreamIndex?: number;
};

export type SourceSelection =
  | { kind: "auto" }
  | { kind: "jellyfin" }
  | { kind: "aioStreams"; discoveryId: string; candidateId: string };

export type PlaybackCandidate = {
  id: string;
  source: "jellyfin" | "aioStreams";
  label: string;
  description: string | null;
  preferred: boolean;
  addon: string | null;
  service: string | null;
  cached: boolean | null;
  resolution: string | null;
  quality: string | null;
  container: string | null;
  sizeBytes: number | null;
  webReady: boolean;
};

export type SourceDiscovery = {
  discoveryId: string;
  sources: PlaybackCandidate[];
  issues: Array<{
    source: "jellyfin" | "aioStreams";
    code: "partialResults" | "upstreamUnavailable";
  }>;
};

export type SourcePickerState =
  | { status: "loading" }
  | { status: "ready"; discovery: SourceDiscovery }
  | { status: "error"; message: string };

type PlaybackSelection = PlaybackTrackSelection & {
  resumeSeconds?: number;
  episode?: Episode;
  sourceSelection?: SourceSelection;
};

export type ActivePlayback = PlaybackSelection &
  (
    | { status: "loading" }
    | { status: "ready"; descriptor: PlaybackDescriptor }
    | { status: "error"; message: string }
  );

function playbackTarget(media: TitleMedia, episode?: Episode) {
  if (episode) {
    return {
      kind: "episode" as const,
      tmdbId: episode.tmdbId,
      seriesTmdbId: media.tmdbId,
      seasonNumber: episode.seasonNumber,
      episodeNumber: episode.episodeNumber,
    };
  }
  if (media.kind === "series") {
    throw new Error("Choose an episode before starting playback.");
  }
  return { kind: "movie" as const, tmdbId: media.tmdbId };
}

export async function discoverPlaybackSources(
  media: TitleMedia,
  episode?: Episode,
  signal?: AbortSignal,
) {
  const response = await fetch("/v1/playback/sources", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({ target: playbackTarget(media, episode) }),
    signal,
  });
  if (!response.ok) throw await playbackResponseError(response, "Source discovery");
  return response.json() as Promise<SourceDiscovery>;
}

export function playbackSourcesQuery(media: TitleMedia, episode?: Episode) {
  return queryOptions({
    queryKey: [
      "reel",
      "playback",
      "sources",
      media.kind,
      media.tmdbId,
      episode?.tmdbId ?? null,
      episode?.seasonNumber ?? null,
      episode?.episodeNumber ?? null,
    ] as const,
    queryFn: ({ signal }) => discoverPlaybackSources(media, episode, signal),
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
    retry: 1,
  });
}

export async function activatePlayback(
  media: TitleMedia,
  episode?: Episode,
  resumeSeconds?: number,
  signal?: AbortSignal,
  trackSelection: PlaybackTrackSelection = {},
  sourceSelection: SourceSelection = { kind: "auto" },
) {
  const video = document.createElement("video");
  const supportsMp4 = Boolean(
    video.canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"'),
  );
  const supportsWebm = Boolean(
    video.canPlayType('video/webm; codecs="vp9, opus"'),
  );
  const response = await fetch("/v1/playback/activate", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({
      target: playbackTarget(media, episode),
      selection: sourceSelection,
      startPositionSeconds: resumeSeconds,
      ...trackSelection,
      capabilities: {
        containers: [supportsMp4 ? "mp4" : null, supportsWebm ? "webm" : null].filter(
          (value): value is string => value != null,
        ),
        videoCodecs: [supportsMp4 ? "h264" : null, supportsWebm ? "vp9" : null].filter(
          (value): value is string => value != null,
        ),
        audioCodecs: [supportsMp4 ? "aac" : null, supportsWebm ? "opus" : null].filter(
          (value): value is string => value != null,
        ),
        hls:
          Boolean(video.canPlayType("application/vnd.apple.mpegurl")) ||
          "MediaSource" in window,
        maxStreamingBitrate: 40_000_000,
      },
    }),
    signal,
  });
  if (!response.ok) throw await playbackResponseError(response, "Playback activation");
  return response.json() as Promise<PlaybackDescriptor>;
}

async function playbackResponseError(response: Response, action: string) {
  const body = (await response.json().catch(() => null)) as
    | { error?: { message?: string } }
    | null;
  return new Error(body?.error?.message ?? `${action} failed (${response.status})`);
}

export function PlayerView({
  media,
  playback,
  onBack,
  onRetry,
  onSelectTracks,
}: {
  media: TitleMedia;
  playback: ActivePlayback;
  onBack: () => void;
  onRetry: () => void;
  onSelectTracks: (
    resumeSeconds: number,
    selection: PlaybackTrackSelection,
  ) => Promise<PlaybackDescriptor>;
}) {
  if (playback.status === "ready") {
    return (
      <ReelVideoPlayer
        media={media}
        playback={playback}
        onBack={onBack}
        onSelectTracks={onSelectTracks}
      />
    );
  }

  return (
    <FullScreenShell onBack={onBack} backLabel="Back">
      <main className="grid min-h-[calc(100dvh-5rem)] place-items-center bg-black px-5 py-10">
        <div className="max-w-xl text-center" role="status">
          <div className="mt-8">
            <Eyebrow>
              {playback.status === "loading"
                ? "Preparing stream"
                : "Playback unavailable"}
            </Eyebrow>
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
          {playback.status === "loading" ? (
            <p className="mt-6 text-sm text-muted">
              Resolving the best compatible source…
            </p>
          ) : (
            <>
              <p className="mt-6 text-sm text-amber-200">{playback.message}</p>
              <button
                type="button"
                className={`${primaryButtonClass} mt-7`}
                onClick={onRetry}
              >
                Try again
              </button>
            </>
          )}
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
    Math.max(0, (progress.positionSeconds / progress.durationSeconds) * 100),
  );
  return (
    <span className={`block h-1 bg-white/25 ${className}`} aria-hidden="true">
      <span
        className="block h-full bg-accent"
        style={{ width: `${percent}%` }}
      />
    </span>
  );
}

export function WatchNowControl({
  progress,
  onPlay,
  onOpenSources,
  sourcesOpen = false,
  disabled = false,
}: {
  progress: PlaybackProgress | null;
  onPlay: (resumeSeconds?: number) => void;
  onOpenSources: (resumeSeconds?: number) => void;
  sourcesOpen?: boolean;
  disabled?: boolean;
}) {
  const resumeSeconds = progress?.positionSeconds;
  return (
    <div
      className="inline-flex min-h-11 overflow-hidden rounded-full border border-accent bg-accent text-background shadow-sm"
      role="group"
      aria-label="Playback actions"
    >
      <button
        type="button"
        className="inline-flex min-h-11 items-center justify-center gap-2 px-5 py-2.5 text-sm font-semibold transition-colors hover:bg-amber-300 focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-background disabled:cursor-not-allowed disabled:opacity-50"
        disabled={disabled}
        onClick={() => onPlay(resumeSeconds)}
      >
        <PlayIcon /> {progress ? `Resume · ${formatRemaining(progress)}` : "Watch Now"}
      </button>
      <button
        type="button"
        className="grid min-h-11 min-w-11 place-items-center border-l border-background/25 px-3 transition-colors hover:bg-amber-300 focus-visible:z-10 focus-visible:outline-2 focus-visible:outline-offset-[-3px] focus-visible:outline-background disabled:cursor-not-allowed disabled:opacity-50"
        aria-label="Choose another playback source"
        aria-haspopup="dialog"
        aria-expanded={sourcesOpen}
        title="Choose another source"
        disabled={disabled}
        onClick={() => onOpenSources(resumeSeconds)}
      >
        <ChevronDownIcon />
      </button>
    </div>
  );
}

export function PlaybackHint({
  progress,
}: {
  progress: PlaybackProgress | null;
}) {
  return progress ? (
    <p className="mt-4 text-xs leading-5 text-muted">
      Watch Now prefers the local Jellyfin copy when one is available.
    </p>
  ) : null;
}

export function SourcePickerDialog({
  state,
  onClose,
  onRetry,
  onChoose,
}: {
  state: SourcePickerState;
  onClose: () => void;
  onRetry: () => void;
  onChoose: (source: PlaybackCandidate) => void;
}) {
  return (
    <Dialog labelledBy="source-picker-title" onClose={onClose} className="max-w-xl">
      <div className="flex items-start justify-between gap-5">
        <div>
          <Eyebrow>Playback</Eyebrow>
          <h2 id="source-picker-title" className="text-2xl font-semibold">
            Choose a source
          </h2>
        </div>
        <button
          type="button"
          className="min-h-11 px-2 text-sm text-muted hover:text-ink"
          onClick={onClose}
        >
          Close
        </button>
      </div>
      {state.status === "loading" ? (
        <p className="mt-8 text-sm text-muted" role="status">
          Finding direct streams…
        </p>
      ) : state.status === "error" ? (
        <div className="mt-8" role="alert">
          <p className="text-sm text-amber-200">{state.message}</p>
          <button type="button" className={`${buttonClass} mt-5`} onClick={onRetry}>
            Try again
          </button>
        </div>
      ) : state.discovery.sources.length ? (
        <>
          <ul className="mt-6 grid gap-2">
            {state.discovery.sources.map((source, index) => (
              <li key={`${source.source}:${source.id}`}>
                <button
                  type="button"
                  className="group flex min-h-14 w-full items-center gap-3 rounded-xl border border-white/10 bg-white/4 px-3 py-2.5 text-left transition-colors hover:border-accent/55 hover:bg-white/8 focus-visible:border-accent disabled:cursor-not-allowed disabled:opacity-45"
                  disabled={!source.webReady}
                  onClick={() => onChoose(source)}
                >
                  <span className="grid size-8 shrink-0 place-items-center rounded-full bg-white/7 font-mono text-[0.65rem] font-semibold text-muted group-hover:text-accent">
                    {source.preferred ? <PlayIcon /> : index + 1}
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-sm font-semibold">
                      {source.label}
                    </span>
                    <span className="mt-1 block truncate text-xs text-muted">
                      {source.description ??
                        (source.source === "jellyfin" ? "Jellyfin" : "Direct stream")}
                    </span>
                  </span>
                  <span className="shrink-0 rounded-full border border-white/10 px-2 py-1 text-[0.65rem] font-semibold tracking-wide text-muted uppercase">
                    {!source.webReady
                      ? "Unavailable"
                      : source.preferred
                        ? "Local"
                        : source.container?.toUpperCase() ?? "Direct"}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          {state.discovery.issues.length ? (
            <p className="mt-5 text-xs leading-5 text-muted">
              Some providers could not be reached; available results are still shown.
            </p>
          ) : null}
        </>
      ) : (
        <p className="mt-8 text-sm text-muted" role="status">
          No direct streams are available for this title.
        </p>
      )}
    </Dialog>
  );
}

export function formatRemaining(progress: PlaybackProgress) {
  const minutes = Math.max(
    1,
    Math.ceil((progress.durationSeconds - progress.positionSeconds) / 60),
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

export function DownloadedStatus() {
  return (
    <span
      className="inline-flex min-h-11 items-center justify-center gap-2 rounded-full border border-white/15 bg-white/5 px-5 py-2.5 text-sm font-semibold text-ink/75"
      role="status"
    >
      <CheckIcon /> Downloaded
    </span>
  );
}

function CheckIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4 fill-none stroke-current"
      viewBox="0 0 16 16"
      strokeWidth="1.75"
    >
      <path d="m3 8.5 3 3 7-7" />
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

function ChevronDownIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4 fill-none stroke-current"
      viewBox="0 0 16 16"
      strokeWidth="1.75"
    >
      <path d="m4 6 4 4 4-4" />
    </svg>
  );
}
