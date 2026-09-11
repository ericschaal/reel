import type {
  Episode,
  PlaybackProgress,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import { Artwork, RatingBadge } from "../../../media-card";
import { Eyebrow, glassClass } from "../../../ui";
import {
  DownloadIcon,
  formatRemaining,
  formatTime,
  ProgressBar,
} from "./playback";

const airDateFormatter = new Intl.DateTimeFormat("en", {
  month: "short",
  day: "numeric",
  year: "numeric",
  timeZone: "UTC",
});

export function SeriesHierarchy({
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
          {season.episodes.map((episode) => (
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
                {(episode.availability === "local") ? (
                  <span className="absolute top-2 left-2 rounded-md bg-emerald-200 px-2 py-1 text-[10px] font-bold tracking-wide text-emerald-950 uppercase">
                    In library
                  </span>
                ) : null}
              </span>
              <span className="pointer-events-none grid min-w-0 content-center gap-2.5">
                <span className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                  <span className="font-mono text-xs text-accent">
                    E{episode.episodeNumber}
                  </span>
                  <strong className="font-semibold [overflow-wrap:anywhere] transition-colors group-hover:text-accent">
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
              {!(episode.availability === "local") ? (
                <button
                  type="button"
                  className="relative z-10 inline-flex min-h-11 items-center justify-center gap-2 self-center rounded-full border border-line px-4 text-xs font-semibold text-muted hover:border-white/40 hover:bg-white/10 hover:text-ink"
                  onClick={() => onDownloadEpisode(episode)}
                  aria-label={`Download ${episode.title}`}
                >
                  <DownloadIcon />
                  <span className="sm:hidden lg:inline">Download</span>
                </button>
              ) : null}
            </article>
          ))}
        </div>
      ) : (
        <div className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}>
          No episodes are available for this season yet.
        </div>
      )}
    </section>
  );
}

export function SeasonSelector({
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

export function EpisodeMetadata({ episode }: { episode: Episode }) {
  const airDate = episode.airDate ? formatAirDate(episode.airDate) : null;
  return (
    <span className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
      <span
        className="inline-flex min-h-6 items-center gap-1.5 rounded-full border border-white/10 bg-white/6 px-2.5 py-1 font-medium text-ink/85"
        aria-label={airDate ? `Aired ${airDate}` : "Air date unavailable"}
      >
        <CalendarIcon />
        <span>{airDate ?? "Air date unavailable"}</span>
      </span>
      {episode.runtimeMinutes != null ? (
        <span className="inline-flex min-h-6 items-center rounded-full px-2 py-1 font-medium text-ink/65">
          {episode.runtimeMinutes} min
        </span>
      ) : null}
      {episode.rating != null ? (
        <RatingBadge rating={episode.rating} variant="chip" />
      ) : null}
    </span>
  );
}

export function EpisodeRail({
  episodes,
  currentEpisodeId,
  onOpenEpisode,
}: {
  episodes: Episode[];
  currentEpisodeId: string;
  onOpenEpisode: (episode: Episode) => void;
}) {
  return (
    <div
      data-keyboard-rail="true"
      className="mt-6 flex snap-x snap-proximity gap-4 overflow-x-auto pb-5 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
    >
      {episodes.map((item) => {
        const current = item.id === currentEpisodeId;
        return (
          <button
            key={item.id}
            type="button"
            aria-current={current ? "true" : undefined}
            onClick={() => onOpenEpisode(item)}
            className={`interactive-card group grid min-w-0 w-[78vw] max-w-xs shrink-0 snap-start content-start gap-3 overflow-hidden rounded-2xl border p-3 text-left ${current ? "border-accent bg-accent/7" : "border-line bg-panel/70"}`}
          >
            <span className="relative block aspect-video overflow-hidden rounded-xl bg-panel">
              <Artwork src={item.still} sizes="320px" />
              <span className="absolute inset-x-2 top-2 flex items-start justify-between gap-2">
                {current ? (
                  <span className="rounded-full bg-accent px-2.5 py-1 text-[10px] font-bold text-background uppercase">
                    Now viewing
                  </span>
                ) : (
                  <span />
                )}
                {item.availability === "local" ? (
                  <span className="rounded-full bg-emerald-200 px-2.5 py-1 text-[10px] font-bold text-emerald-950 uppercase">
                    In library
                  </span>
                ) : null}
              </span>
            </span>
            <span className="grid min-w-0 gap-2 px-1 pb-1">
              <span className="min-w-0 flex items-baseline gap-2">
                <span className="shrink-0 font-mono text-xs text-accent">
                  E{item.episodeNumber}
                </span>
                <strong className="min-w-0 flex-1 truncate text-sm font-semibold interactive-card-title">
                  {item.title}
                </strong>
              </span>
              <EpisodeMetadata episode={item} />
            </span>
          </button>
        );
      })}
    </div>
  );
}

export function NextUp({
  episode,
  progress,
}: {
  episode: Episode;
  progress: PlaybackProgress | null;
}) {
  return (
    <div
      className={`mt-7 flex max-w-2xl items-center gap-4 rounded-2xl p-3 ${glassClass}`}
    >
      <div className="relative hidden aspect-video w-32 shrink-0 overflow-hidden rounded-xl bg-panel sm:block">
        <Artwork src={episode.still} sizes="128px" />
        {progress ? (
          <ProgressBar
            progress={progress}
            className="absolute inset-x-0 bottom-0"
          />
        ) : null}
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

function formatAirDate(value: string) {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  if (!match) return value;
  const [, year, month, day] = match;
  return airDateFormatter.format(
    new Date(Date.UTC(Number(year), Number(month) - 1, Number(day))),
  );
}

function CalendarIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-3.5 shrink-0 fill-none stroke-current"
      viewBox="0 0 16 16"
      strokeWidth="1.5"
    >
      <rect x="2.5" y="3.5" width="11" height="10" rx="2" />
      <path d="M5 2v3M11 2v3M2.5 7h11" />
    </svg>
  );
}
