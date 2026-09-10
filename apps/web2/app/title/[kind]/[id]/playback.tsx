import { useEffect, useRef, useState, type ReactNode } from "react";
import type { Episode, TitleMedia, PlaybackProgress } from "../../../catalogue";
import { Eyebrow, NavigationHeader, primaryButtonClass } from "../../../ui";

export type PlaybackDescriptor = {
  sessionId: string;
  source: "jellyfin";
  delivery: "direct" | "hls";
  mediaUrl: string;
  container: string | null;
  durationSeconds: number | null;
};

type PlaybackSelection = { resumeSeconds?: number; episode?: Episode };

export type ActivePlayback = PlaybackSelection &
  (
    | { status: "loading" }
    | { status: "ready"; descriptor: PlaybackDescriptor }
    | { status: "error"; message: string }
  );

export async function activateJellyfinPlayback(
  media: TitleMedia,
  episode?: Episode,
  resumeSeconds?: number,
  signal?: AbortSignal,
) {
  const video = document.createElement("video");
  const supportsMp4 = Boolean(
    video.canPlayType('video/mp4; codecs="avc1.42E01E, mp4a.40.2"'),
  );
  const supportsWebm = Boolean(
    video.canPlayType('video/webm; codecs="vp9, opus"'),
  );
  const target = episode
    ? {
        kind: "episode" as const,
        tmdbId: episode.tmdbId,
        seriesTmdbId: media.tmdbId,
        seasonNumber: episode.seasonNumber,
        episodeNumber: episode.episodeNumber,
      }
    : { kind: "movie" as const, tmdbId: media.tmdbId };
  const response = await fetch("/v1/playback/activate", {
    method: "POST",
    headers: { "content-type": "application/json", accept: "application/json" },
    body: JSON.stringify({
      target,
      startPositionSeconds: resumeSeconds,
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
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as
      | { error?: { message?: string } }
      | null;
    throw new Error(
      body?.error?.message ?? `Playback activation failed (${response.status})`,
    );
  }
  return response.json() as Promise<PlaybackDescriptor>;
}

export function PlayerView({
  media,
  playback,
  onBack,
  onRetry,
}: {
  media: TitleMedia;
  playback: ActivePlayback;
  onBack: () => void;
  onRetry: () => void;
}) {
  return (
    <FullScreenShell onBack={onBack} backLabel="Back">
      <main className="grid min-h-[calc(100dvh-5rem)] place-items-center bg-black px-5 py-10">
        {playback.status === "ready" ? (
          <JellyfinVideo playback={playback} />
        ) : (
          <div className="max-w-xl text-center" role="status">
            <div className="mt-8">
              <Eyebrow>
                {playback.status === "loading"
                  ? "Opening Jellyfin"
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
                Negotiating the best compatible local stream…
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
        )}
      </main>
    </FullScreenShell>
  );
}

function JellyfinVideo({
  playback,
}: {
  playback: Extract<ActivePlayback, { status: "ready" }>;
}) {
  const videoRef = useRef<HTMLVideoElement>(null);
  const [playerError, setPlayerError] = useState<string | null>(null);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    let cancelled = false;
    let destroyHls: (() => void) | undefined;
    const seekToResumePosition = () => {
      if (
        playback.descriptor.delivery === "direct" &&
        playback.resumeSeconds &&
        Number.isFinite(video.duration)
      ) {
        video.currentTime = Math.min(
          playback.resumeSeconds,
          Math.max(0, video.duration - 1),
        );
      }
    };
    video.addEventListener("loadedmetadata", seekToResumePosition, { once: true });

    if (
      playback.descriptor.delivery === "direct" ||
      video.canPlayType("application/vnd.apple.mpegurl")
    ) {
      video.src = playback.descriptor.mediaUrl;
    } else {
      void import("hls.js")
        .then(({ default: Hls }) => {
          if (cancelled) return;
          if (!Hls.isSupported()) {
            throw new Error("This browser cannot play HLS video.");
          }
          const hls = new Hls();
          destroyHls = () => hls.destroy();
          hls.on(Hls.Events.ERROR, (_event, data) => {
            if (data.fatal) {
              setPlayerError("The Jellyfin stream stopped unexpectedly.");
            }
          });
          hls.loadSource(playback.descriptor.mediaUrl);
          hls.attachMedia(video);
        })
        .catch((reason: unknown) => {
          if (!cancelled) {
            setPlayerError(
              reason instanceof Error
                ? reason.message
                : "The video player could not start.",
            );
          }
        });
    }

    return () => {
      cancelled = true;
      video.removeEventListener("loadedmetadata", seekToResumePosition);
      destroyHls?.();
      video.removeAttribute("src");
      video.load();
    };
  }, [
    playback.descriptor.delivery,
    playback.descriptor.mediaUrl,
    playback.resumeSeconds,
  ]);

  return (
    <div className="w-full max-w-6xl">
      <video
        ref={videoRef}
        className="aspect-video w-full bg-black shadow-2xl"
        controls
        autoPlay
        playsInline
        onError={() =>
          setPlayerError("The browser could not play this Jellyfin stream.")
        }
      />
      {playerError ? (
        <p className="mt-4 text-center text-sm text-amber-200">{playerError}</p>
      ) : null}
    </div>
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

export function PlaybackControl({
  progress,
  disabled,
  onPlay,
}: {
  progress: PlaybackProgress | null;
  disabled: boolean;
  onPlay: (resumeSeconds?: number) => void;
}) {
  return (
    <button
      type="button"
      className={primaryButtonClass}
      disabled={disabled}
      onClick={() => onPlay(progress?.positionSeconds)}
    >
      <PlayIcon /> {progress ? `Resume · ${formatRemaining(progress)}` : "Play"}
    </button>
  );
}

export function PlaybackHint({
  progress,
}: {
  progress: PlaybackProgress | null;
}) {
  return progress ? (
    <p className="mt-4 text-xs leading-5 text-muted">
      Resume uses the local Jellyfin copy.
    </p>
  ) : null;
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
