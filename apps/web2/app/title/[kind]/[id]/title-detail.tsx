"use client";

import Link from "next/link";
import { useMemo, useRef, useState, type ReactNode } from "react";
import type {
  Episode,
  MediaCard,
  PlaybackProgress,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import { Artwork, RatingBadge } from "../../../media-card";
import { buttonClass, primaryButtonClass, Eyebrow, glassClass, pageGutter } from "../../../ui";

type Source = {
  id: string;
  provider: string;
  quality: string;
  detail: string;
  available: boolean;
};

type DownloadScope =
  | { kind: "movie" }
  | { kind: "episode"; episode: Episode }
  | { kind: "series"; seasonNumbers: number[] };

type ActivePlayback = {
  sourceId: string;
  sourceLabel: string;
  sourceDetail: string;
  resumeSeconds?: number;
  episode?: Episode;
};

const exampleSources: Source[] = [
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

export function TitleDetail({
  media,
  series = null,
  initialSeason = null,
  seriesError = null,
}: {
  media: MediaCard;
  series?: SeriesDetails | null;
  initialSeason?: SeasonDetails | null;
  seriesError?: string | null;
}) {
  const [season, setSeason] = useState(initialSeason);
  const [seasonNumber, setSeasonNumber] = useState(
    initialSeason?.seasonNumber ?? null,
  );
  const [seasonLoading, setSeasonLoading] = useState(false);
  const [seasonError, setSeasonError] = useState(seriesError);
  const [nextEpisode] = useState(
    initialSeason?.episodes.find((episode) => episode.id === media.progress?.episodeId) ??
      initialSeason?.episodes[0] ??
      null,
  );
  const [episodeDialog, setEpisodeDialog] = useState<Episode | null>(null);
  const seriesScrollPosition = useRef(0);
  const localCopy =
    media.kind === "series" ? nextEpisode?.localCopy : media.localCopy;
  const [selectedSource, setSelectedSource] = useState("stremio-1");
  const [activePlayback, setActivePlayback] = useState<ActivePlayback | null>(null);
  const [downloadScope, setDownloadScope] = useState<DownloadScope | null>(null);
  const selected =
    exampleSources.find((source) => source.id === selectedSource) ?? exampleSources[0];
  const progress = progressForSelection(media.progress, nextEpisode);
  const sources = playbackSources(Boolean(localCopy));
  const preferredSource = preferredPlaybackSource(
    progress,
    Boolean(localCopy),
    selected,
  );
  const lastSourceAvailable = isLastSourceAvailable(progress, Boolean(localCopy));
  const resumeSourceLabel = progress
    ? lastSourceAvailable
      ? progress.lastSourceLabel
      : localCopy
        ? "Jellyfin"
        : selected.provider
    : null;

  async function selectSeason(nextSeasonNumber: number) {
    if (!series || nextSeasonNumber === seasonNumber) return;
    setSeasonNumber(nextSeasonNumber);
    setSeasonLoading(true);
    setSeasonError(null);
    try {
      const response = await fetch(
        `/api/reel/v1/titles/series/${series.tmdbId}/seasons/${nextSeasonNumber}?language=en`,
      );
      if (!response.ok) throw new Error("Season unavailable");
      const nextSeason: SeasonDetails = await response.json();
      setSeason(nextSeason);
      setSelectedSource("stremio-1");
    } catch {
      setSeason(null);
      setSeasonError("This season could not be loaded. Please try again.");
    } finally {
      setSeasonLoading(false);
    }
  }

  function playLocal(resumeSeconds?: number, episode = nextEpisode ?? undefined) {
    setActivePlayback({
      sourceId: "local",
      sourceLabel: "Jellyfin",
      sourceDetail: "Local library",
      resumeSeconds,
      episode,
    });
  }

  function stream(
    source = selected,
    resumeSeconds?: number,
    episode = nextEpisode ?? undefined,
  ) {
    setSelectedSource(source.id);
    setActivePlayback({
      sourceId: source.id,
      sourceLabel: source.provider,
      sourceDetail: source.quality,
      resumeSeconds,
      episode,
    });
  }

  function openDownload() {
    if (media.kind === "movie") setDownloadScope({ kind: "movie" });
    else if (seasonNumber != null) {
      setDownloadScope({
        kind: "series",
        seasonNumbers: [seasonNumber],
      });
    }
  }

  function openEpisode(episode: Episode) {
    if (!episodeDialog) seriesScrollPosition.current = window.scrollY;
    setEpisodeDialog(episode);
    window.scrollTo({ top: 0, behavior: "instant" });
  }

  function closeEpisode() {
    setEpisodeDialog(null);
    requestAnimationFrame(() =>
      window.scrollTo({
        top: seriesScrollPosition.current,
        behavior: "instant",
      }),
    );
  }

  if (activePlayback) {
    return (
      <PlayerView
        media={media}
        playback={activePlayback}
        onBack={() => setActivePlayback(null)}
      />
    );
  }

  if (downloadScope) {
    return (
      <DownloadView
        media={media}
        series={series}
        season={season}
        scope={downloadScope}
        onChange={setDownloadScope}
        onBack={() => setDownloadScope(null)}
      />
    );
  }

  if (episodeDialog) {
    return (
      <EpisodeDetailView
        media={media}
        episode={episodeDialog}
        series={series}
        season={season}
        seasonNumber={seasonNumber}
        seasonLoading={seasonLoading}
        seasonError={seasonError}
        selectedSource={selected}
        progress={progressForSelection(media.progress, episodeDialog)}
        onBack={closeEpisode}
        onSelectSeason={(number) => void selectSeason(number)}
        onOpenEpisode={openEpisode}
        onPlayLocal={(resumeSeconds) =>
          playLocal(resumeSeconds, episodeDialog)
        }
        onStream={(source, resumeSeconds) => {
          if (source.id === "local") playLocal(resumeSeconds, episodeDialog);
          else stream(source, resumeSeconds, episodeDialog);
        }}
        onDownload={() =>
          setDownloadScope({ kind: "episode", episode: episodeDialog })
        }
      />
    );
  }

  return (
    <div className="relative isolate min-h-dvh">
      <div
        className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[75vh] overflow-hidden opacity-35"
        aria-hidden="true"
      >
        <Artwork src={media.images.backdrop} sizes="100vw" priority />
        <div className="absolute inset-0 bg-linear-to-r from-background via-background/50 to-transparent" />
        <div className="absolute inset-0 bg-linear-to-t from-background via-background/30 to-transparent" />
      </div>
      <NavigationHeader href="/" label="Back to catalogue" />
      <main
        id="main-content"
        className={`mx-auto max-w-[1400px] pt-10 pb-16 sm:pt-16 lg:pt-20 ${pageGutter}`}
      >
        <section className="grid items-start gap-8 md:grid-cols-[220px_minmax(0,1fr)] md:gap-10 lg:grid-cols-[280px_minmax(0,1fr)] lg:gap-16">
          <div className="relative aspect-[2/3] w-32 overflow-hidden rounded-xl border border-line bg-panel shadow-2xl sm:w-40 md:w-full">
            <Artwork
              src={media.images.poster}
              sizes="(max-width: 768px) 160px, 280px"
              priority
            />
          </div>
          <div className="min-w-0 md:py-3">
            <Eyebrow>{media.kind === "movie" ? "Movie" : "Series"}</Eyebrow>
            <h1 className="text-4xl leading-[1.08] font-semibold tracking-[-0.045em] text-balance [overflow-wrap:anywhere] sm:text-5xl lg:text-6xl">
              {media.title}
            </h1>
            <p className="mt-5 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm leading-6 text-muted">
              <span>{media.year ?? "Year unavailable"}</span>
              {media.rating != null ? (
                <>
                  <span aria-hidden="true">·</span>
                  <RatingBadge rating={media.rating} variant="chip" />
                </>
              ) : null}
              {media.kind === "movie" && media.localCopy ? (
                <>
                  <span aria-hidden="true">·</span>
                  <span>In your library</span>
                </>
              ) : null}
              {series?.numberOfSeasons != null ? (
                <>
                  <span aria-hidden="true">·</span>
                  <span>
                    {series.numberOfSeasons} season
                    {series.numberOfSeasons === 1 ? "" : "s"}
                  </span>
                </>
              ) : null}
            </p>
            <p className="mt-6 max-w-2xl text-base leading-7 text-ink/80">
              {media.overview || "No synopsis is available for this title yet."}
            </p>
            {media.kind === "series" && nextEpisode ? (
              <NextUp episode={nextEpisode} progress={progress} />
            ) : null}
            <div className="mt-5 flex flex-wrap items-stretch gap-3">
              <PlaybackControl
                sources={sources}
                selected={preferredSource}
                progress={progress}
                disabled={media.kind === "series" && !nextEpisode}
                onPlay={(source, resumeSeconds) =>
                  source.id === "local"
                    ? playLocal(resumeSeconds)
                    : stream(source, resumeSeconds)
                }
              />
              <button className={buttonClass} type="button" onClick={openDownload}>
                <DownloadIcon /> Download
              </button>
            </div>
            <PlaybackHint
              progress={progress}
              lastSourceAvailable={lastSourceAvailable}
              resumeSourceLabel={resumeSourceLabel}
            />
          </div>
        </section>
        {media.kind === "series" ? (
          <SeriesHierarchy
            series={series}
            season={season}
            seasonNumber={seasonNumber}
            loading={seasonLoading}
            error={seasonError}
            onSelectSeason={(number) => void selectSeason(number)}
            onOpenEpisode={openEpisode}
            onDownloadEpisode={(episode) =>
              setDownloadScope({ kind: "episode", episode })
            }
          />
        ) : null}
      </main>
    </div>
  );
}

function SeriesHierarchy({
  series,
  season,
  seasonNumber,
  loading,
  error,
  onSelectSeason,
  onOpenEpisode,
  onDownloadEpisode,
}: {
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  seasonNumber: number | null;
  loading: boolean;
  error: string | null;
  onSelectSeason: (seasonNumber: number) => void;
  onOpenEpisode: (episode: Episode) => void;
  onDownloadEpisode: (episode: Episode) => void;
}) {
  return (
    <section className="mt-12 sm:mt-16" aria-labelledby="episodes-heading">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div>
          <Eyebrow>Series guide</Eyebrow>
          <h2
            id="episodes-heading"
            className="text-2xl font-semibold tracking-tight sm:text-3xl"
          >
            Episodes
          </h2>
        </div>
        <SeasonSelector
          series={series}
          seasonNumber={seasonNumber}
          loading={loading}
          onSelect={onSelectSeason}
        />
      </div>

      {error ? (
        <div className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}>
          {error}
        </div>
      ) : loading ? (
        <div
          className="mt-6 grid gap-3"
          role="status"
          aria-label="Loading episodes"
          aria-busy="true"
        >
          {[0, 1, 2].map((item) => (
            <div
              key={item}
              className="h-28 rounded-xl bg-white/5 motion-safe:animate-pulse"
            />
          ))}
        </div>
      ) : season?.episodes.length ? (
        <div className="mt-6 grid gap-3">
          {season.issues.some((issue) => issue.source === "jellyfin") ? (
            <p className="text-xs leading-5 text-muted">
              Local-library availability could not be checked. Episode metadata
              is still available.
            </p>
          ) : null}
          {season.episodes.map((episode) => {
            return (
              <article
                key={episode.id}
                className="group relative grid min-w-0 gap-4 rounded-xl border border-line bg-panel/70 p-3 text-left transition-colors hover:border-white/40 hover:bg-white/5 sm:grid-cols-[180px_minmax(0,1fr)_auto] sm:p-4"
              >
                <button
                  type="button"
                  aria-label={`Open episode ${episode.episodeNumber}: ${episode.title}`}
                  onClick={() => onOpenEpisode(episode)}
                  className="absolute inset-0 rounded-xl"
                />
                <span className="pointer-events-none relative block aspect-video overflow-hidden rounded-lg bg-panel">
                  <Artwork src={episode.still} sizes="180px" />
                  {episode.localCopy ? (
                    <span className="absolute top-2 left-2 rounded-md bg-emerald-200 px-2 py-1 text-[10px] font-bold tracking-wide text-emerald-950 uppercase">
                      In library
                    </span>
                  ) : null}
                </span>
                <span className="pointer-events-none grid min-w-0 content-center gap-2 transition-transform group-hover:translate-x-0.5">
                  <span className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                    <span className="font-mono text-xs text-accent">
                      E{episode.episodeNumber}
                    </span>
                    <strong className="font-semibold [overflow-wrap:anywhere]">
                      {episode.title}
                    </strong>
                  </span>
                  <EpisodeMetadata episode={episode} />
                  {episode.overview ? (
                    <span className="line-clamp-2 text-sm leading-6 text-ink/75">
                      {episode.overview}
                    </span>
                  ) : null}
                </span>
                {!episode.localCopy ? (
                  <button
                    type="button"
                    className="relative z-10 inline-flex min-h-11 items-center justify-center gap-2 self-center rounded-full border border-line px-4 text-xs font-semibold text-muted hover:border-white/40 hover:bg-white/10 hover:text-ink"
                    onClick={() => onDownloadEpisode(episode)}
                    aria-label={`Download ${episode.title}`}
                  >
                    <DownloadIcon /> <span className="sm:hidden lg:inline">Download</span>
                  </button>
                ) : null}
              </article>
            );
          })}
        </div>
      ) : (
        <div className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}>
          No episodes are available for this season yet.
        </div>
      )}
    </section>
  );
}

function SeasonSelector({
  series,
  seasonNumber,
  loading,
  onSelect,
}: {
  series: SeriesDetails | null;
  seasonNumber: number | null;
  loading: boolean;
  onSelect: (seasonNumber: number) => void;
}) {
  if (!series?.seasons.length) return null;
  return (
    <label className="grid gap-1.5 text-xs text-muted">
      Season
      <select
        className="min-h-11 rounded-full border border-line bg-panel px-4 text-sm font-semibold text-ink"
        value={seasonNumber ?? ""}
        disabled={loading}
        onChange={(event) => onSelect(Number(event.target.value))}
      >
        {series.seasons.map((item) => (
          <option key={item.id} value={item.seasonNumber}>
            {item.seasonNumber === 0 ? "Specials" : item.title}
            {item.episodeCount != null ? ` · ${item.episodeCount}` : ""}
          </option>
        ))}
      </select>
    </label>
  );
}

function EpisodeMetadata({ episode }: { episode: Episode }) {
  return (
    <span className="flex flex-wrap items-center gap-2 text-xs text-muted">
      <span>{episode.airDate ?? "Air date unavailable"}</span>
      {episode.runtimeMinutes != null ? (
        <><span aria-hidden="true">·</span><span>{episode.runtimeMinutes} min</span></>
      ) : null}
      {episode.rating != null ? (
        <><span aria-hidden="true">·</span><RatingBadge rating={episode.rating} variant="chip" /></>
      ) : null}
    </span>
  );
}

function EpisodeRail({
  episodes,
  currentEpisodeId,
  onOpenEpisode,
}: {
  episodes: Episode[];
  currentEpisodeId: string;
  onOpenEpisode: (episode: Episode) => void;
}) {
  return (
    <div className="mt-6 flex snap-x snap-proximity gap-4 overflow-x-auto pb-5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden">
      {episodes.map((item) => {
        const current = item.id === currentEpisodeId;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={current ? "true" : undefined}
            onClick={() => onOpenEpisode(item)}
            className={`group grid w-[78vw] max-w-xs shrink-0 snap-start content-start gap-3 rounded-2xl border p-3 text-left transition-[transform,border-color,background-color] motion-safe:hover:-translate-y-1 ${current ? "border-accent bg-accent/7" : "border-line bg-panel/70 hover:border-white/40 hover:bg-white/5"}`}
          >
            <span className="relative block aspect-video overflow-hidden rounded-xl bg-panel">
              <Artwork src={item.still} sizes="320px" />
              <span className="absolute inset-x-2 top-2 flex items-start justify-between gap-2">
                {current ? <span className="rounded-full bg-accent px-2.5 py-1 text-[10px] font-bold text-background uppercase">Now viewing</span> : <span />}
                {item.localCopy ? <span className="rounded-full bg-emerald-200 px-2.5 py-1 text-[10px] font-bold text-emerald-950 uppercase">In library</span> : null}
              </span>
            </span>
            <span className="grid min-w-0 gap-2 px-1 pb-1">
              <span className="flex items-baseline gap-2">
                <span className="font-mono text-xs text-accent">E{item.episodeNumber}</span>
                <strong className="truncate text-sm font-semibold group-hover:text-accent">{item.title}</strong>
              </span>
              <EpisodeMetadata episode={item} />
            </span>
          </button>
        );
      })}
    </div>
  );
}

function NextUp({
  episode,
  progress,
}: {
  episode: Episode;
  progress: PlaybackProgress | null;
}) {
  return (
    <div className={`mt-7 flex max-w-2xl items-center gap-4 rounded-2xl p-3 ${glassClass}`}>
      <div className="relative hidden aspect-video w-32 shrink-0 overflow-hidden rounded-xl bg-panel sm:block">
        <Artwork src={episode.still} sizes="128px" />
        {progress ? <ProgressBar progress={progress} className="absolute inset-x-0 bottom-0" /> : null}
      </div>
      <div className="min-w-0 flex-1 py-1">
        <p className="font-mono text-[10px] font-semibold tracking-[0.14em] text-accent uppercase">
          {progress ? "Continue watching" : "Next up"}
        </p>
        <p className="mt-1 truncate text-sm font-semibold">
          S{episode.seasonNumber} E{episode.episodeNumber} · {episode.title}
        </p>
        <p className="mt-1 text-xs text-muted">
          {progress
            ? `${formatRemaining(progress)} · resumes near ${formatTime(progress.positionSeconds)}`
            : episode.runtimeMinutes != null
              ? `${episode.runtimeMinutes} min`
              : "Ready to play"}
        </p>
      </div>
    </div>
  );
}

function PlayerView({
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
          <span className="mx-auto grid size-20 place-items-center rounded-full bg-accent text-background shadow-[0_0_0_14px_#f4bc521c]"><PlayIcon /></span>
          <div className="mt-8"><Eyebrow>Now playing</Eyebrow></div>
          <h1 className="mt-[-0.5rem] text-3xl font-semibold tracking-tight sm:text-5xl">
            {playback.episode ? playback.episode.title : media.title}
          </h1>
          {playback.episode ? <p className="mt-3 text-sm text-muted">{media.title} · S{playback.episode.seasonNumber} E{playback.episode.episodeNumber}</p> : null}
          <p className="mt-6 font-semibold">{playback.resumeSeconds ? `Resuming near ${formatTime(playback.resumeSeconds)}` : "Starting from the beginning"}</p>
          <p className="mt-2 text-sm text-muted">{playback.sourceLabel} · {playback.sourceDetail}</p>
          <p className="mt-8 text-xs text-muted">Playback preview only. The player endpoint is not connected yet.</p>
        </div>
      </main>
    </FullScreenShell>
  );
}

function FullScreenShell({
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

function NavigationHeader({
  label,
  href,
  onBack,
}: {
  label: string;
  href?: string;
  onBack?: () => void;
}) {
  const backClass =
    "inline-flex min-h-12 items-center gap-3 rounded-full border border-white/15 bg-white/8 pr-5 pl-3 text-sm font-semibold text-ink shadow-lg transition-colors hover:border-accent/60 hover:bg-white/12";
  const content = (
    <>
      <span className="grid size-7 place-items-center rounded-full bg-white/10"><BackIcon /></span>
      {label}
    </>
  );
  return (
    <header className={`sticky top-0 z-40 flex min-h-20 items-center justify-between border-b border-white/10 bg-background/85 py-3 backdrop-blur-2xl ${pageGutter}`}>
      {href ? (
        <Link className={backClass} href={href}>{content}</Link>
      ) : (
        <button type="button" className={backClass} onClick={onBack}>{content}</button>
      )}
      <Link className="text-sm font-extrabold tracking-[0.28em] text-accent" href="/" aria-label="Reel home">REEL</Link>
    </header>
  );
}

function ProgressBar({ progress, className = "" }: { progress: PlaybackProgress; className?: string }) {
  const percent = Math.min(100, Math.max(0, (progress.positionSeconds / progress.durationSeconds) * 100));
  return <span className={`block h-1 bg-white/25 ${className}`} aria-hidden="true"><span className="block h-full bg-accent" style={{ width: `${percent}%` }} /></span>;
}

function PlaybackControl({
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
            {resumeSeconds ? `Resume near ${formatTime(resumeSeconds)} with` : "Play with"}
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
              <span className="grid size-7 shrink-0 place-items-center rounded-full border border-line font-mono text-[10px] text-accent">{source.id === "local" ? "L" : source.provider.match(/\d+$/)?.[0] ?? index + 1}</span>
              <span className="grid min-w-0 flex-1 gap-0.5">
                <strong className="text-sm">{source.provider}</strong>
                <span className="truncate text-xs text-muted">{source.quality}</span>
              </span>
              <span className="text-xs text-muted">{source.detail}</span>
            </button>
          ))}
        </div>
      </details>
    </div>
  );
}

function PlaybackHint({
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
        Resume uses {progress.lastSourceLabel}. Choose another stream source to resume at approximately the same point.
      </p>
    );
  }
  if (progress) {
    return (
      <p className="mt-4 text-xs leading-5 text-amber-200/80">
        {progress.lastSourceLabel} is no longer available. Resume will use {resumeSourceLabel} at approximately the same point.
      </p>
    );
  }
  return null;
}

function EpisodeDetailView({
  media,
  episode,
  series,
  season,
  seasonNumber,
  seasonLoading,
  seasonError,
  selectedSource,
  progress,
  onBack,
  onSelectSeason,
  onOpenEpisode,
  onPlayLocal,
  onStream,
  onDownload,
}: {
  media: MediaCard;
  episode: Episode;
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  seasonNumber: number | null;
  seasonLoading: boolean;
  seasonError: string | null;
  selectedSource: Source;
  progress: PlaybackProgress | null;
  onBack: () => void;
  onSelectSeason: (seasonNumber: number) => void;
  onOpenEpisode: (episode: Episode) => void;
  onPlayLocal: (resumeSeconds?: number) => void;
  onStream: (source: Source, resumeSeconds?: number) => void;
  onDownload: () => void;
}) {
  const localCopy = Boolean(episode.localCopy);
  const lastSourceAvailable = isLastSourceAvailable(progress, localCopy);
  const resumeSourceLabel = progress
    ? lastSourceAvailable
      ? progress.lastSourceLabel
      : localCopy
        ? "Jellyfin"
        : selectedSource.provider
    : null;

  const sources = playbackSources(localCopy);
  const preferredSource = preferredPlaybackSource(
    progress,
    localCopy,
    selectedSource,
  );

  return (
    <FullScreenShell onBack={onBack} backLabel="Back to episodes">
      <main className={`mx-auto max-w-[1400px] py-10 sm:py-16 ${pageGutter}`}>
        <section className="grid items-center gap-8 lg:grid-cols-[minmax(0,1.3fr)_minmax(360px,0.7fr)] lg:gap-14">
          <div className="relative aspect-video overflow-hidden rounded-2xl border border-line bg-panel shadow-2xl">
            <Artwork src={episode.still} sizes="(max-width: 1024px) 100vw, 65vw" priority />
            {progress ? <ProgressBar progress={progress} /> : null}
          </div>
          <div className="min-w-0">
            <Eyebrow>Season {episode.seasonNumber} · Episode {episode.episodeNumber}</Eyebrow>
            <p className="mb-3 text-sm font-semibold text-muted">{media.title}</p>
            <h1 className="text-4xl leading-tight font-semibold tracking-[-0.04em] sm:text-5xl">{episode.title}</h1>
            <div className="mt-5"><EpisodeMetadata episode={episode} /></div>
            <p className="mt-5 text-base leading-7 text-ink/75">{episode.overview || `An episode of ${media.title}.`}</p>
            <div className="mt-7 flex flex-wrap items-stretch gap-3">
              <PlaybackControl
                sources={sources}
                selected={preferredSource}
                progress={progress}
                disabled={false}
                onPlay={(source, resumeSeconds) =>
                  source.id === "local"
                    ? onPlayLocal(resumeSeconds)
                    : onStream(source, resumeSeconds)
                }
              />
              {!episode.localCopy ? (
                <button type="button" className={buttonClass} onClick={onDownload}><DownloadIcon /> Download</button>
              ) : null}
            </div>
            <PlaybackHint progress={progress} lastSourceAvailable={lastSourceAvailable} resumeSourceLabel={resumeSourceLabel} />
          </div>
        </section>
        <section className="mt-14 border-t border-white/10 pt-10 sm:mt-20 sm:pt-12" aria-labelledby="season-episodes-heading">
          <div className="flex flex-wrap items-end justify-between gap-4">
            <div>
              <Eyebrow>Keep watching</Eyebrow>
              <h2 id="season-episodes-heading" className="text-2xl font-semibold tracking-tight sm:text-3xl">Episodes</h2>
            </div>
            <SeasonSelector series={series} seasonNumber={seasonNumber} loading={seasonLoading} onSelect={onSelectSeason} />
          </div>
          {seasonError ? (
            <div className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}>{seasonError}</div>
          ) : seasonLoading ? (
            <div className="mt-6 flex gap-4 overflow-hidden" role="status" aria-label="Loading episodes">
              {[0, 1, 2].map((item) => <div key={item} className="aspect-video w-[78vw] max-w-xs shrink-0 rounded-xl bg-white/5 motion-safe:animate-pulse" />)}
            </div>
          ) : season?.episodes.length ? (
            <EpisodeRail episodes={season.episodes} currentEpisodeId={episode.id} onOpenEpisode={onOpenEpisode} />
          ) : (
            <div className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}>No episodes are available for this season yet.</div>
          )}
        </section>
      </main>
    </FullScreenShell>
  );
}

function DownloadView({
  media,
  series,
  season,
  scope,
  onChange,
  onBack,
}: {
  media: MediaCard;
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  scope: DownloadScope;
  onChange: (scope: DownloadScope) => void;
  onBack: () => void;
}) {
  const regularSeasons = useMemo(
    () => series?.seasons.filter((item) => item.seasonNumber > 0) ?? [],
    [series],
  );
  const selectedSeasons = scope.kind === "series" ? scope.seasonNumbers : [];
  const selectedEpisodeCount = selectedSeasons.reduce((total, number) => {
    const summary = regularSeasons.find((item) => item.seasonNumber === number);
    if (season?.seasonNumber === number) {
      return total + season.episodes.filter((episode) => !episode.localCopy).length;
    }
    return total + (summary?.episodeCount ?? 0);
  }, 0);

  function toggleSeason(number: number) {
    const next = selectedSeasons.includes(number)
      ? selectedSeasons.filter((item) => item !== number)
      : [...selectedSeasons, number].sort((a, b) => a - b);
    onChange({ kind: "series", seasonNumbers: next });
  }

  const title =
    scope.kind === "episode"
      ? "Download episode"
      : media.kind === "series"
        ? "Download series"
        : "Download movie";

  return (
    <FullScreenShell onBack={onBack} backLabel="Back to title">
      <main className={`mx-auto max-w-3xl py-10 sm:py-16 ${pageGutter}`}>
      <Eyebrow>Save for later</Eyebrow>
      <h1 className="text-4xl leading-tight font-semibold tracking-[-0.04em] sm:text-5xl">{title}</h1>

      {scope.kind === "series" ? (
        <>
          <div className="mt-5 flex items-center justify-between gap-4">
            <p className="text-sm leading-6 text-muted">Choose one season or several. Episodes already in your library are skipped.</p>
            <button
              type="button"
              className="shrink-0 text-xs font-semibold text-accent hover:text-amber-200"
              onClick={() =>
                onChange({
                  kind: "series",
                  seasonNumbers:
                    selectedSeasons.length === regularSeasons.length
                      ? []
                      : regularSeasons.map((item) => item.seasonNumber),
                })
              }
            >
              {selectedSeasons.length === regularSeasons.length ? "Clear all" : "All seasons"}
            </button>
          </div>
          <fieldset className="mt-5 grid max-h-[42vh] gap-2 overflow-y-auto pr-1">
            <legend className="sr-only">Seasons to download</legend>
            {regularSeasons.map((item) => {
              const selected = selectedSeasons.includes(item.seasonNumber);
              const missing = season?.seasonNumber === item.seasonNumber
                ? season.episodes.filter((episode) => !episode.localCopy).length
                : item.episodeCount;
              return (
                <label key={item.id} className={`flex min-h-16 items-center gap-3 rounded-xl border px-4 transition-colors ${selected ? "border-accent bg-accent/7" : "border-line bg-white/3 hover:bg-white/6"}`}>
                  <input type="checkbox" checked={selected} onChange={() => toggleSeason(item.seasonNumber)} className="size-4 accent-accent" />
                  <span className="grid min-w-0 flex-1 gap-1">
                    <strong className="text-sm">{item.title}</strong>
                    <span className="text-xs text-muted">{missing ?? "Unknown number of"} episode{missing === 1 ? "" : "s"} to download</span>
                  </span>
                </label>
              );
            })}
          </fieldset>
          <button type="button" disabled={!selectedSeasons.length} className={`${primaryButtonClass} mt-6 w-full`} onClick={onBack}>
            <DownloadIcon /> Download {selectedSeasons.length} season{selectedSeasons.length === 1 ? "" : "s"} · {selectedEpisodeCount} episode{selectedEpisodeCount === 1 ? "" : "s"}
          </button>
        </>
      ) : (
        <>
          <div className={`mt-6 rounded-2xl p-5 ${glassClass}`}>
            <p className="text-sm font-semibold">
              {scope.kind === "episode"
                ? `${media.title} · S${scope.episode.seasonNumber} E${scope.episode.episodeNumber}`
                : media.title}
            </p>
            {scope.kind === "episode" ? <p className="mt-1 text-sm text-muted">{scope.episode.title}</p> : null}
          </div>
          <p className="mt-5 text-sm leading-6 text-muted">Reel will request it and add it to your library when ready.</p>
          <button type="button" className={`${primaryButtonClass} mt-6 w-full`} onClick={onBack}><DownloadIcon /> Download</button>
        </>
      )}
      <p className="mt-4 text-center text-xs text-muted">Request preview only. Nothing will be downloaded yet.</p>
      </main>
    </FullScreenShell>
  );
}

function progressForSelection(progress: PlaybackProgress | null | undefined, episode: Episode | null) {
  if (!progress || progress.positionSeconds <= 0 || progress.positionSeconds >= progress.durationSeconds) return null;
  if (progress.episodeId && progress.episodeId !== episode?.id) return null;
  return progress;
}

function playbackSources(hasLocalCopy: boolean) {
  return hasLocalCopy ? [jellyfinSource, ...exampleSources] : exampleSources;
}

function preferredPlaybackSource(
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

function isLastSourceAvailable(
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

function formatRemaining(progress: PlaybackProgress) {
  const minutes = Math.max(1, Math.ceil((progress.durationSeconds - progress.positionSeconds) / 60));
  return `${minutes} min left`;
}

function formatTime(seconds: number) {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  return hours ? `${hours}h ${minutes}m` : `${minutes}m`;
}

function PlayIcon() {
  return <svg aria-hidden="true" className="size-4 fill-current" viewBox="0 0 16 16"><path d="M3.5 2.2a1 1 0 0 1 1.5-.86l9 5.8a1 1 0 0 1 0 1.72l-9 5.8a1 1 0 0 1-1.5-.86V2.2Z" /></svg>;
}

function BackIcon() {
  return <svg aria-hidden="true" className="size-4 fill-none stroke-current" viewBox="0 0 16 16" strokeWidth="1.8"><path d="m9.5 3.5-4.5 4.5 4.5 4.5M5.5 8H13" /></svg>;
}

function DownloadIcon() {
  return <svg aria-hidden="true" className="size-4 fill-none stroke-current" viewBox="0 0 16 16" strokeWidth="1.5"><path d="M8 2v8m-3-3 3 3 3-3M3 13.5h10" /></svg>;
}

function ChevronIcon() {
  return <svg aria-hidden="true" className="size-3.5 fill-none stroke-current transition-transform group-open:rotate-180" viewBox="0 0 16 16" strokeWidth="2"><path d="m4 6 4 4 4-4" /></svg>;
}
