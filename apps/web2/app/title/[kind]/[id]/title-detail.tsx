"use client";

import { useState } from "react";
import type {
  Episode,
  MediaCard,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import { Dialog } from "../../../dialog";
import { Artwork } from "../../../media-card";
import {
  buttonClass,
  primaryButtonClass,
  Eyebrow,
  glassClass,
  Header,
  liquidGlassGroupClass,
  liquidGlassItemClass,
  pageGutter,
} from "../../../ui";

type Source = {
  id: string;
  provider: string;
  quality: string;
  detail: string;
  available: boolean;
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
  const [selectedEpisodeId, setSelectedEpisodeId] = useState(
    initialSeason?.episodes[0]?.id ?? null,
  );
  const selectedEpisode =
    season?.episodes.find((episode) => episode.id === selectedEpisodeId) ??
    season?.episodes[0] ??
    null;
  const localCopy =
    media.kind === "series" ? selectedEpisode?.localCopy : media.localCopy;
  const sources: Source[] = [
    {
      id: "local",
      provider: "Jellyfin",
      quality: "Local library",
      detail: localCopy ? "In library" : "Unavailable",
      available: Boolean(localCopy),
    },
    ...exampleSources,
  ];
  const [selectedSource, setSelectedSource] = useState(
    localCopy ? "local" : "stremio-1",
  );
  const [dialog, setDialog] = useState<"sources" | "player" | "request" | null>(
    null,
  );
  const selected =
    sources.find((source) => source.id === selectedSource) ?? sources[1];

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
      const firstEpisode = nextSeason.episodes[0] ?? null;
      setSeason(nextSeason);
      setSelectedEpisodeId(firstEpisode?.id ?? null);
      setSelectedSource(firstEpisode?.localCopy ? "local" : "stremio-1");
    } catch {
      setSeason(null);
      setSelectedEpisodeId(null);
      setSelectedSource("stremio-1");
      setSeasonError("This season could not be loaded. Please try again.");
    } finally {
      setSeasonLoading(false);
    }
  }

  function selectEpisode(episode: Episode) {
    setSelectedEpisodeId(episode.id);
    setSelectedSource(episode.localCopy ? "local" : "stremio-1");
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
      <Header preview />
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
            <p className="mt-5 text-sm leading-6 text-muted">
              {media.year ?? "Year unavailable"}
              {media.rating != null ? ` · ★ ${media.rating.toFixed(1)}` : ""}
              {media.kind === "movie" && media.localCopy
                ? " · In your library"
                : ""}
              {series?.numberOfSeasons != null
                ? ` · ${series.numberOfSeasons} season${series.numberOfSeasons === 1 ? "" : "s"}`
                : ""}
            </p>
            <p className="mt-6 max-w-2xl text-base leading-7 text-ink/80">
              {media.overview || "No synopsis is available for this title yet."}
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <button
                className={`${primaryButtonClass} w-full sm:w-auto`}
                type="button"
                disabled={media.kind === "series" && !selectedEpisode}
                onClick={() => setDialog("player")}
              >
                <span aria-hidden="true">▶</span>{" "}
                {media.kind === "series" ? "Preview episode" : "Preview player"}
              </button>
              <button
                className={buttonClass}
                type="button"
                disabled={media.kind === "series" && !selectedEpisode}
                onClick={() => setDialog("sources")}
              >
                Other sources
              </button>
              <button
                className={buttonClass}
                type="button"
                onClick={() => setDialog("request")}
              >
                Request download
              </button>
            </div>
            <p className="mt-4 text-xs leading-5 text-muted">
              Preview only. Playback and download requests are not available
              yet.
            </p>
          </div>
        </section>
        {media.kind === "series" ? (
          <SeriesHierarchy
            series={series}
            season={season}
            seasonNumber={seasonNumber}
            selectedEpisode={selectedEpisode}
            loading={seasonLoading}
            error={seasonError}
            onSelectSeason={(number) => void selectSeason(number)}
            onSelectEpisode={selectEpisode}
          />
        ) : null}
        <section
          className={`mt-10 flex flex-wrap items-center justify-between gap-5 rounded-2xl p-5 sm:mt-14 sm:p-6 ${glassClass}`}
          aria-label="Preview playback source"
        >
          <div className="grid gap-2">
            <span className="text-xs text-muted">Preview source</span>
            <strong className="text-sm font-semibold">
              {selected.provider}{" "}
              <span className="font-normal text-muted">
                · {selected.quality}
              </span>
            </strong>
          </div>
          <button
            className="inline-flex min-h-11 items-center text-sm font-semibold text-accent hover:text-amber-200"
            type="button"
            disabled={media.kind === "series" && !selectedEpisode}
            onClick={() => setDialog("sources")}
          >
            Change source{" "}
            <span className="ml-2" aria-hidden="true">
              →
            </span>
          </button>
        </section>
      </main>
      {dialog ? (
        <Dialog labelledBy="dialog-title" onClose={() => setDialog(null)}>
          <div className="flex items-start justify-between gap-4">
            <div>
              <Eyebrow>
                {dialog === "sources" ? "Playback options" : "UI preview"}
              </Eyebrow>
              <h2
                id="dialog-title"
                className="text-2xl leading-tight font-semibold tracking-tight sm:text-3xl"
              >
                {dialog === "sources"
                  ? "Choose a source"
                  : dialog === "player"
                    ? "Player preview"
                    : "Download requests"}
              </h2>
            </div>
            <button
              type="button"
              className="inline-flex size-11 shrink-0 items-center justify-center rounded-full border border-line text-2xl text-muted hover:bg-white/5 hover:text-ink"
              aria-label="Close dialog"
              onClick={() => setDialog(null)}
            >
              ×
            </button>
          </div>
          {dialog === "sources" ? (
            <>
              <p className="mt-4 text-sm leading-6 text-muted">
                These are example sources, shown in add-on order. Selecting one
                updates this preview only.
              </p>
              <fieldset
                className={`mt-6 grid rounded-xl ${liquidGlassGroupClass}`}
              >
                <legend className="sr-only">Playback source</legend>
                {sources.map((source) => (
                  <label
                    key={source.id}
                    className={`flex min-h-20 items-center gap-3 border-b border-white/50 p-4 last:border-b-0 focus-within:z-10 focus-within:ring-2 focus-within:ring-accent focus-within:ring-inset ${liquidGlassItemClass} ${!source.available ? "cursor-not-allowed opacity-50" : selectedSource === source.id ? "cursor-pointer bg-white/30" : "cursor-pointer hover:bg-white/30"}`}
                  >
                    <input
                      type="radio"
                      name="source"
                      value={source.id}
                      checked={selectedSource === source.id}
                      disabled={!source.available}
                      onChange={() => setSelectedSource(source.id)}
                      className="size-4 shrink-0 accent-accent focus-visible:outline-none"
                    />
                    <span className="grid min-w-0 flex-1 gap-1">
                      <strong className="text-sm font-semibold">
                        {source.provider}
                      </strong>
                      <span className="text-xs leading-5 text-muted">
                        {source.quality}
                      </span>
                    </span>
                    <span className="hidden text-xs text-muted sm:block">
                      {source.detail}
                    </span>
                  </label>
                ))}
              </fieldset>
              <button
                className={`${primaryButtonClass} mt-6 w-full`}
                type="button"
                onClick={() => setDialog("player")}
              >
                Preview with {selected.provider}
              </button>
            </>
          ) : (
            <>
              <p className="mt-6 text-lg font-semibold [overflow-wrap:anywhere]">
                {selectedEpisode
                  ? `${media.title} · S${selectedEpisode.seasonNumber} E${selectedEpisode.episodeNumber} · ${selectedEpisode.title}`
                  : media.title}
              </p>
              <p className="mt-3 text-sm leading-6 text-muted">
                {dialog === "player"
                  ? `${selected.provider} · ${selected.quality}. Playback is not connected yet.`
                  : "Download requests are not connected yet. No request has been submitted."}
              </p>
              <button
                className={`${buttonClass} mt-6`}
                type="button"
                onClick={() => setDialog(null)}
              >
                Back to title
              </button>
            </>
          )}
        </Dialog>
      ) : null}
    </div>
  );
}

function SeriesHierarchy({
  series,
  season,
  seasonNumber,
  selectedEpisode,
  loading,
  error,
  onSelectSeason,
  onSelectEpisode,
}: {
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  seasonNumber: number | null;
  selectedEpisode: Episode | null;
  loading: boolean;
  error: string | null;
  onSelectSeason: (seasonNumber: number) => void;
  onSelectEpisode: (episode: Episode) => void;
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
        {series?.seasons.length ? (
          <label className="grid gap-1.5 text-xs text-muted">
            Season
            <select
              className="min-h-11 rounded-full border border-line bg-panel px-4 text-sm font-semibold text-ink"
              value={seasonNumber ?? ""}
              disabled={loading}
              onChange={(event) => onSelectSeason(Number(event.target.value))}
            >
              {series.seasons.map((item) => (
                <option key={item.id} value={item.seasonNumber}>
                  {item.seasonNumber === 0 ? "Specials" : item.title}
                  {item.episodeCount != null ? ` · ${item.episodeCount}` : ""}
                </option>
              ))}
            </select>
          </label>
        ) : null}
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
            const active = selectedEpisode?.id === episode.id;
            return (
              <button
                key={episode.id}
                type="button"
                aria-pressed={active}
                onClick={() => onSelectEpisode(episode)}
                className={`grid min-w-0 gap-4 rounded-xl border p-3 text-left transition-colors sm:grid-cols-[180px_minmax(0,1fr)] sm:p-4 ${
                  active
                    ? "border-accent bg-accent/5"
                    : "border-line bg-panel/70 hover:border-white/40 hover:bg-white/5"
                }`}
              >
                <span className="relative block aspect-video overflow-hidden rounded-lg bg-panel">
                  <Artwork src={episode.still} sizes="180px" />
                  {episode.localCopy ? (
                    <span className="absolute top-2 left-2 rounded-md bg-emerald-200 px-2 py-1 text-[10px] font-bold tracking-wide text-emerald-950 uppercase">
                      In library
                    </span>
                  ) : null}
                </span>
                <span className="grid min-w-0 content-center gap-2">
                  <span className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                    <span className="font-mono text-xs text-accent">
                      E{episode.episodeNumber}
                    </span>
                    <strong className="font-semibold [overflow-wrap:anywhere]">
                      {episode.title}
                    </strong>
                  </span>
                  <span className="text-xs text-muted">
                    {episode.airDate ?? "Air date unavailable"}
                    {episode.runtimeMinutes != null
                      ? ` · ${episode.runtimeMinutes} min`
                      : ""}
                    {episode.rating != null
                      ? ` · ★ ${episode.rating.toFixed(1)}`
                      : ""}
                  </span>
                  {episode.overview ? (
                    <span className="line-clamp-2 text-sm leading-6 text-ink/75">
                      {episode.overview}
                    </span>
                  ) : null}
                </span>
              </button>
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
