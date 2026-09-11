import { queryOptions } from "@tanstack/react-query";
import type { KeyboardEvent, ReactNode } from "react";
import type { Episode, TitleMedia, PlaybackProgress } from "../../../catalogue";
import { Artwork } from "../../../media-card";
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
  | { kind: "auto"; discoveryId?: string }
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
  videoCodec: string | null;
  sizeBytes: number | null;
  webReady: boolean;
};

type DirectPlayProfile = {
  container: "mp4" | "webm" | "mkv";
  videoCodec?: "h264" | "hevc" | "vp8" | "vp9" | "av1";
};

type PlayerCapabilities = {
  containers: string[];
  videoCodecs: string[];
  audioCodecs: string[];
  directPlayProfiles: DirectPlayProfile[];
  hls: boolean;
  maxStreamingBitrate: number;
};

let cachedPlayerCapabilities: PlayerCapabilities | null = null;

function playerCapabilities() {
  if (cachedPlayerCapabilities) return cachedPlayerCapabilities;
  const video = document.createElement("video");
  const profiles: DirectPlayProfile[] = [];
  const addProfile = (
    container: DirectPlayProfile["container"],
    videoCodec: DirectPlayProfile["videoCodec"],
    mimeTypes: string[],
  ) => {
    if (mimeTypes.some((mimeType) => Boolean(video.canPlayType(mimeType)))) {
      profiles.push(videoCodec ? { container, videoCodec } : { container });
    }
  };

  addProfile("mp4", undefined, ["video/mp4"]);
  addProfile("mp4", "h264", [
    'video/mp4; codecs="avc1.42E01E, mp4a.40.2"',
  ]);
  addProfile("webm", undefined, ["video/webm"]);
  addProfile("webm", "vp8", ['video/webm; codecs="vp8, vorbis"']);
  addProfile("webm", "vp9", ['video/webm; codecs="vp9, opus"']);
  addProfile("webm", "av1", ['video/webm; codecs="av01, opus"']);
  addProfile("mkv", undefined, ["video/x-matroska", "video/matroska"]);
  addProfile("mkv", "h264", [
    'video/x-matroska; codecs="avc1"',
    'video/matroska; codecs="avc1"',
  ]);
  addProfile("mkv", "hevc", [
    'video/x-matroska; codecs="hvc1"',
    'video/matroska; codecs="hvc1"',
  ]);
  addProfile("mkv", "vp9", [
    'video/x-matroska; codecs="vp9"',
    'video/matroska; codecs="vp9"',
  ]);
  addProfile("mkv", "av1", [
    'video/x-matroska; codecs="av01"',
    'video/matroska; codecs="av01"',
  ]);

  cachedPlayerCapabilities = {
    containers: [...new Set(profiles.map((profile) => profile.container))],
    videoCodecs: [...new Set(profiles.flatMap((profile) =>
      profile.videoCodec ? [profile.videoCodec] : [],
    ))],
    audioCodecs: ["aac", "mp3", "opus", "vorbis"],
    directPlayProfiles: profiles,
    hls:
      Boolean(video.canPlayType("application/vnd.apple.mpegurl")) ||
      "MediaSource" in window,
    maxStreamingBitrate: 40_000_000,
  };
  return cachedPlayerCapabilities;
}

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
  | { status: "ready"; discovery: SourceDiscovery; refreshing: boolean }
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
      imdbId: media.imdbId,
      seasonNumber: episode.seasonNumber,
      episodeNumber: episode.episodeNumber,
    };
  }
  if (media.kind === "series") {
    throw new Error("Choose an episode before starting playback.");
  }
  return {
    kind: "movie" as const,
    tmdbId: media.tmdbId,
    imdbId: media.imdbId,
  };
}

export async function discoverPlaybackSources(
  media: TitleMedia,
  episode?: Episode,
  signal?: AbortSignal,
) {
  const response = await fetch("/v1/playback/sources", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({
      target: playbackTarget(media, episode),
      capabilities: playerCapabilities(),
    }),
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
      media.imdbId,
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
  const response = await fetch("/v1/playback/activate", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({
      target: playbackTarget(media, episode),
      selection: sourceSelection,
      startPositionSeconds: resumeSeconds,
      ...trackSelection,
      capabilities: playerCapabilities(),
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
  focusBack = false,
}: {
  children: ReactNode;
  onBack: () => void;
  backLabel: string;
  focusBack?: boolean;
}) {
  return (
    <div className="min-h-dvh bg-[radial-gradient(ellipse_at_50%_0%,#233336_0%,transparent_48%)]">
      <NavigationHeader onBack={onBack} label={backLabel} autoFocusBack={focusBack} />
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
        data-source-picker-trigger
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

export function SourcePickerView({
  state,
  media,
  episode,
  onClose,
  onRetry,
  onChoose,
}: {
  state: SourcePickerState;
  media: TitleMedia;
  episode?: Episode;
  onClose: () => void;
  onRetry: () => void;
  onChoose: (source: PlaybackCandidate) => void;
}) {
  const playableSources =
    state.status === "ready"
      ? state.discovery.sources.filter((source) => source.webReady)
      : [];
  const defaultSource =
    playableSources.find((source) => source.preferred) ?? playableSources[0];
  const providersUnavailable =
    state.status === "ready" &&
    state.discovery.issues.some(
      (issue) => issue.source === "aioStreams" && issue.code === "upstreamUnavailable",
    );
  const incompatibleSources =
    state.status === "ready" &&
    state.discovery.sources.length > 0 &&
    playableSources.length === 0;

  function moveSourceFocus(event: KeyboardEvent<HTMLElement>) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (![
      "ArrowDown",
      "ArrowUp",
      "Home",
      "End",
    ].includes(event.key)) return;
    const options = Array.from(
      event.currentTarget.querySelectorAll<HTMLButtonElement>(
        "[data-source-option]:not(:disabled)",
      ),
    );
    if (!options.length) return;
    event.preventDefault();
    const current = options.indexOf(document.activeElement as HTMLButtonElement);
    const next =
      event.key === "Home"
        ? 0
        : event.key === "End"
          ? options.length - 1
          : event.key === "ArrowDown"
            ? (Math.max(current, -1) + 1) % options.length
            : (current <= 0 ? options.length : current) - 1;
    options[next]?.focus();
    options[next]?.scrollIntoView({ block: "nearest" });
  }

  return (
    <div
      className="fixed inset-0 z-50 h-dvh overflow-hidden bg-background"
      role="dialog"
      aria-modal="true"
      aria-labelledby="source-picker-title"
      data-keyboard-navigation="managed"
      onKeyDown={moveSourceFocus}
    >
      <FullScreenShell
        onBack={onClose}
        backLabel="Back to title"
        focusBack={state.status === "loading"}
      >
        <main className="relative isolate h-[calc(100dvh-5rem)] overflow-hidden px-5 pt-5 pb-[max(2.5rem,env(safe-area-inset-bottom))] sm:px-8 sm:pt-6 lg:px-12">
        <div
          className="pointer-events-none absolute inset-0 -z-20 opacity-20"
          aria-hidden="true"
        >
          <Artwork
            src={episode?.still ?? media.images.backdrop}
            sizes="100vw"
            priority
          />
        </div>
        <div className="pointer-events-none absolute inset-0 -z-10 bg-[linear-gradient(90deg,var(--color-background)_0%,rgb(9_11_15/0.93)_48%,rgb(9_11_15/0.78)_100%),linear-gradient(0deg,var(--color-background)_0%,transparent_70%)]" />

        <section className="mx-auto flex h-full min-h-0 w-full max-w-5xl flex-col">
          <div className="max-w-3xl shrink-0">
            <Eyebrow>Playback sources</Eyebrow>
            <h1
              id="source-picker-title"
              className="text-3xl leading-none font-semibold tracking-[-0.045em] text-balance sm:text-4xl lg:text-5xl"
            >
              {episode?.title ?? media.title}
            </h1>
            <p className="mt-2 text-sm text-muted sm:text-base">
              {episode
                ? `${media.title} · Season ${episode.seasonNumber}, Episode ${episode.episodeNumber}`
                : "Choose where you want to watch"}
            </p>
          </div>

          <div className="mt-5 flex min-h-0 flex-1 flex-col sm:mt-6">
            {state.status === "loading" ? (
              <SourcePickerLoading />
            ) : state.status === "error" ? (
              <SourcePickerMessage
                title="Sources could not be loaded"
                message={state.message}
                onRetry={onRetry}
                onClose={onClose}
              />
            ) : playableSources.length ? (
              <div className="flex min-h-0 flex-1 flex-col">
                <div className="flex shrink-0 items-end justify-between gap-4">
                  <div>
                    <h2 className="text-lg font-semibold sm:text-xl">Available now</h2>
                    <p className="mt-1 text-sm text-muted">
                      Local playback is preferred, followed by your AIOStreams order.
                    </p>
                  </div>
                  {state.refreshing ? (
                    <span className="text-xs font-semibold tracking-wide text-muted uppercase" role="status">
                      Refreshing…
                    </span>
                  ) : null}
                </div>
                <ul
                  className="mt-3 grid min-h-0 flex-1 content-start gap-2 overflow-y-auto overscroll-contain pr-2 pb-2 [scrollbar-width:thin]"
                  aria-label="Available playback sources"
                >
                  {playableSources.map((source, index) => {
                    const isDefault = source.id === defaultSource?.id;
                    return (
                      <li key={`${source.source}:${source.id}`}>
                        <button
                          type="button"
                          data-source-option
                          autoFocus={isDefault}
                          className={`group flex min-h-16 w-full items-center gap-3 rounded-xl border px-4 py-2.5 text-left shadow-lg transition-[border-color,background-color,transform] duration-200 focus-visible:outline-none sm:gap-4 sm:px-5 ${
                            isDefault
                              ? "border-accent/55 bg-accent/10 hover:bg-accent/15 focus-visible:border-accent focus-visible:bg-accent/18"
                              : "border-white/12 bg-white/6 hover:border-white/30 hover:bg-white/10 focus-visible:border-accent focus-visible:bg-white/12"
                          }`}
                          aria-label={`${source.label}. ${source.description ?? "Direct stream"}${isDefault ? ". Recommended" : ""}`}
                          onClick={() => onChoose(source)}
                        >
                          <span
                            className={`grid size-9 shrink-0 place-items-center rounded-full text-xs font-semibold sm:size-10 ${
                              isDefault
                                ? "bg-accent text-background"
                                : "bg-white/9 text-ink/75 group-focus-visible:bg-accent group-focus-visible:text-background"
                            }`}
                            aria-hidden="true"
                          >
                            {source.source === "jellyfin" ? <PlayIcon /> : index + 1}
                          </span>
                          <span className="min-w-0 flex-1">
                            <span className="flex flex-wrap items-center gap-2">
                              <span className="text-base font-semibold sm:text-lg">
                                {source.label}
                              </span>
                              {isDefault ? (
                                <span className="rounded-full bg-accent/15 px-2.5 py-1 text-[0.65rem] font-bold tracking-[0.12em] text-accent uppercase">
                                  Recommended
                                </span>
                              ) : null}
                            </span>
                            <span className="mt-0.5 block truncate text-xs text-muted sm:text-sm">
                              {source.description ??
                                (source.source === "jellyfin"
                                  ? "Your local Jellyfin library"
                                  : "Direct stream")}
                            </span>
                          </span>
                          <span className="hidden shrink-0 items-center gap-3 sm:flex">
                            {source.cached === true ? (
                              <span className="text-xs font-semibold text-emerald-200">
                                Cached
                              </span>
                            ) : null}
                            <span className="rounded-full border border-white/14 bg-black/15 px-3 py-1.5 text-xs font-semibold tracking-wide text-ink/70 uppercase">
                              {source.preferred
                                ? "Local"
                                : source.container?.toUpperCase() ?? "Direct"}
                            </span>
                            <ChevronRightIcon />
                          </span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
                {state.discovery.issues.some(
                  (issue) => issue.code === "partialResults",
                ) ? (
                  <p className="mt-2 shrink-0 text-xs leading-5 text-muted">
                    Some providers did not respond. The sources above are still ready to use.
                  </p>
                ) : null}
              </div>
            ) : incompatibleSources ? (
              <SourcePickerMessage
                title="No browser-compatible streams"
                message="AIOStreams returned direct streams, but their containers or codecs are not playable in this browser. Try another device or adjust the source formats in AIOStreams."
                onRetry={onRetry}
                onClose={onClose}
              />
            ) : providersUnavailable ? (
              <SourcePickerMessage
                title="Streaming providers need attention"
                message="AIOStreams could not return a direct stream because one or more providers failed. Check the provider and debrid connections in AIOStreams, then try again."
                onRetry={onRetry}
                onClose={onClose}
              />
            ) : (
              <SourcePickerMessage
                title="No direct streams found"
                message="There are no direct HTTP streams for this title yet. Torrent sources are not supported in Reel right now."
                onRetry={onRetry}
                onClose={onClose}
              />
            )}
          </div>
        </section>
        </main>
      </FullScreenShell>
    </div>
  );
}

function SourcePickerLoading() {
  return (
    <div role="status" aria-label="Finding direct streams">
      <h2 className="text-xl font-semibold sm:text-2xl">Finding the best sources…</h2>
      <div className="mt-5 grid gap-3" aria-hidden="true">
        {[0, 1, 2].map((item) => (
          <div
            key={item}
            className="h-20 rounded-2xl border border-white/8 bg-white/5 motion-safe:animate-pulse sm:h-24"
          />
        ))}
      </div>
    </div>
  );
}

function SourcePickerMessage({
  title,
  message,
  onRetry,
  onClose,
}: {
  title: string;
  message: string;
  onRetry: () => void;
  onClose: () => void;
}) {
  return (
    <div className="max-w-2xl rounded-3xl border border-white/12 bg-white/6 p-6 shadow-2xl sm:p-9" role="status">
      <h2 className="text-2xl font-semibold tracking-tight sm:text-3xl">{title}</h2>
      <p className="mt-3 max-w-xl text-sm leading-6 text-muted sm:text-base sm:leading-7">
        {message}
      </p>
      <div className="mt-7 flex flex-wrap gap-3">
        <button
          type="button"
          autoFocus
          className={primaryButtonClass}
          onClick={onRetry}
        >
          Try again
        </button>
        <button type="button" className={buttonClass} onClick={onClose}>
          Back to title
        </button>
      </div>
    </div>
  );
}

function ChevronRightIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-5 fill-none stroke-current text-muted group-focus-visible:text-accent"
      viewBox="0 0 16 16"
      strokeWidth="1.75"
    >
      <path d="m6 3.5 4.5 4.5L6 12.5" />
    </svg>
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
