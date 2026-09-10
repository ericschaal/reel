"use client";

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type {
  Episode,
  MediaCard,
  PlaybackProgress,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import { Artwork, RatingBadge } from "../../../media-card";
import { seasonQuery } from "../../../reel-query";
import {
  buttonClass,
  Eyebrow,
  NavigationHeader,
  pageGutter,
} from "../../../ui";
import { DownloadView, type DownloadScope } from "./download-view";
import { EpisodeDetailView } from "./episode-detail-view";
import { NextUp, SeriesHierarchy } from "./episodes";
import {
  type ActivePlayback,
  DownloadIcon,
  exampleSources,
  isLastSourceAvailable,
  PlaybackControl,
  PlaybackHint,
  playbackSources,
  PlayerView,
  preferredPlaybackSource,
  type Source,
} from "./playback";

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
  const [seasonNumber, setSeasonNumber] = useState(
    initialSeason?.seasonNumber ?? null,
  );
  const selectedSeasonQuery = useQuery({
    ...seasonQuery(series?.tmdbId ?? 0, seasonNumber ?? 0),
    enabled: Boolean(series && seasonNumber != null),
    initialData:
      initialSeason?.seasonNumber === seasonNumber ? initialSeason : undefined,
  });
  const season = selectedSeasonQuery.data ?? null;
  const seasonLoading = selectedSeasonQuery.isPending && seasonNumber != null;
  const seasonError =
    seriesError ??
    (selectedSeasonQuery.isError
      ? "This season could not be loaded. Please try again."
      : null);
  const [nextEpisode] = useState(
    () =>
      initialSeason?.episodes.find(
        (episode) => episode.id === media.progress?.episodeId,
      ) ??
      initialSeason?.episodes[0] ??
      null,
  );
  const [episodeDialog, setEpisodeDialog] = useState<Episode | null>(null);
  const localCopy =
    media.kind === "series" ? nextEpisode?.localCopy : media.localCopy;
  const [selectedSource, setSelectedSource] = useState("stremio-1");
  const [activePlayback, setActivePlayback] =
    useState<ActivePlayback | null>(null);
  const [downloadScope, setDownloadScope] = useState<DownloadScope | null>(
    null,
  );
  const selected =
    exampleSources.find((source) => source.id === selectedSource) ??
    exampleSources[0];
  const progress = progressForSelection(media.progress, nextEpisode);
  const sources = playbackSources(Boolean(localCopy));
  const preferredSource = preferredPlaybackSource(
    progress,
    Boolean(localCopy),
    selected,
  );
  const lastSourceAvailable = isLastSourceAvailable(
    progress,
    Boolean(localCopy),
  );
  const resumeSourceLabel = progress
    ? lastSourceAvailable
      ? progress.lastSourceLabel
      : localCopy
        ? "Jellyfin"
        : selected.provider
    : null;

  function selectSeason(nextSeasonNumber: number) {
    if (!series || nextSeasonNumber === seasonNumber) return;
    setSeasonNumber(nextSeasonNumber);
    setSelectedSource("stremio-1");
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
    source: Source = selected,
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
    setEpisodeDialog(episode);
  }

  function closeEpisode() {
    setEpisodeDialog(null);
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
        onSelectSeason={selectSeason}
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
              <button
                className={buttonClass}
                type="button"
                onClick={openDownload}
              >
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
            onSelectSeason={selectSeason}
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

function progressForSelection(
  progress: PlaybackProgress | null | undefined,
  episode: Episode | null,
) {
  if (
    !progress ||
    progress.positionSeconds <= 0 ||
    progress.positionSeconds >= progress.durationSeconds
  )
    return null;
  if (progress.episodeId && progress.episodeId !== episode?.id) return null;
  return progress;
}
