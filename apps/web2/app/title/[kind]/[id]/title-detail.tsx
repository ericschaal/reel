"use client";

import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";
import {
  playbackHref,
  type Episode,
  type TitleMedia,
  type MovieDetails,
  type PlaybackProgress,
  type SeriesDetails,
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
  DownloadIcon,
  DownloadedStatus,
  PlaybackHint,
  playbackSourcesQuery,
  SourcePickerView,
  type PlaybackCandidate,
  type SourceSelection,
  WatchNowControl,
} from "./playback";

export function TitleDetail({
  title,
  progress: initialProgress,
}: {
  title: MovieDetails | SeriesDetails;
  progress: PlaybackProgress | null;
}) {
  const router = useRouter();
  const queryClient = useQueryClient();
  const media: TitleMedia = { ...title, progress: initialProgress };
  const series = title.kind === "series" ? title : null;
  const initialSeason = series?.initialSeason ?? null;
  const seriesError =
    series && !initialSeason && series.issues.length
      ? "Season and episode details are unavailable right now."
      : null;
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
  const discoveryEpisode = episodeDialog ?? nextEpisode ?? undefined;
  const canDiscover = media.kind === "movie" || discoveryEpisode != null;
  const sourceDiscoveryQuery = useQuery({
    ...playbackSourcesQuery(media, discoveryEpisode),
    enabled: canDiscover,
  });
  const [sourcePicker, setSourcePicker] = useState<{
    resumeSeconds?: number;
    episode?: Episode;
  } | null>(null);
  const [downloadScope, setDownloadScope] = useState<DownloadScope | null>(
    null,
  );
  const progress = progressForSelection(media.progress, nextEpisode);

  function selectSeason(nextSeasonNumber: number) {
    if (!series || nextSeasonNumber === seasonNumber) return;
    setSeasonNumber(nextSeasonNumber);
  }

  async function play(
    resumeSeconds?: number,
    episode = nextEpisode ?? undefined,
    sourceSelection: SourceSelection = { kind: "auto" },
  ) {
    let resolvedSelection = sourceSelection;
    if (sourceSelection.kind === "auto") {
      try {
        const discovery = await queryClient.fetchQuery(
          playbackSourcesQuery(media, episode),
        );
        resolvedSelection = {
          kind: "auto",
          discoveryId: discovery.discoveryId,
        };
      } catch {
        // Direct play still has a fresh-search fallback if discovery failed.
      }
    }
    const href = new URL(
      playbackHref(media, episode, resumeSeconds),
      "http://reel.local",
    );
    if (resolvedSelection.kind === "jellyfin") {
      href.searchParams.set("source", "jellyfin");
    } else if (resolvedSelection.kind === "aioStreams") {
      href.searchParams.set("source", "aioStreams");
      href.searchParams.set("discovery", resolvedSelection.discoveryId);
      href.searchParams.set("candidate", resolvedSelection.candidateId);
    } else if (resolvedSelection.discoveryId) {
      href.searchParams.set("source", "auto");
      href.searchParams.set("discovery", resolvedSelection.discoveryId);
    }
    router.push(`${href.pathname}${href.search}`, { scroll: false });
  }

  function openSources(
    resumeSeconds?: number,
    episode = nextEpisode ?? undefined,
  ) {
    setSourcePicker({ resumeSeconds, episode });
  }

  function closeSources() {
    setSourcePicker(null);
    window.requestAnimationFrame(() => {
      document
        .querySelector<HTMLButtonElement>("[data-source-picker-trigger]")
        ?.focus();
    });
  }

  function chooseSource(source: PlaybackCandidate) {
    const discovery = sourceDiscoveryQuery.data;
    if (!sourcePicker || !discovery) return;
    const selection: SourceSelection =
      source.source === "jellyfin"
        ? { kind: "jellyfin" }
        : {
            kind: "aioStreams",
            discoveryId: discovery.discoveryId,
            candidateId: source.id,
          };
    const { resumeSeconds, episode } = sourcePicker;
    setSourcePicker(null);
    void play(resumeSeconds, episode, selection);
  }

  function openDownload() {
    if (media.kind === "movie") {
      if (media.availability !== "local") {
        setDownloadScope({ kind: "movie" });
      }
    } else if (seasonNumber != null) {
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

  const sourcePickerState = sourcePicker
    ? sourceDiscoveryQuery.data
      ? {
          status: "ready" as const,
          discovery: sourceDiscoveryQuery.data,
          refreshing: sourceDiscoveryQuery.isFetching,
        }
      : sourceDiscoveryQuery.isError
        ? {
            status: "error" as const,
            message:
              sourceDiscoveryQuery.error instanceof Error
                ? sourceDiscoveryQuery.error.message
                : "Playback sources could not be loaded.",
          }
        : { status: "loading" as const }
    : null;

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

  if (sourcePickerState && sourcePicker) {
    return (
      <SourcePickerView
        state={sourcePickerState}
        media={media}
        episode={sourcePicker.episode}
        onClose={closeSources}
        onRetry={() => void sourceDiscoveryQuery.refetch()}
        onChoose={chooseSource}
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
          progress={progressForSelection(media.progress, episodeDialog)}
          onBack={closeEpisode}
          onSelectSeason={selectSeason}
          onOpenEpisode={openEpisode}
          onPlay={(resumeSeconds) => void play(resumeSeconds, episodeDialog)}
          onOpenSources={(resumeSeconds) =>
            openSources(resumeSeconds, episodeDialog)
          }
          sourcesOpen={sourcePicker != null}
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
              {media.kind === "movie" && media.availability === "local" ? (
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
            {media.availability === "unknown" ? (
              <p className="mt-3 text-sm text-muted">
                Local-library availability could not be checked.
              </p>
            ) : null}
            {media.kind === "series" && nextEpisode ? (
              <NextUp episode={nextEpisode} progress={progress} />
            ) : null}
            <div className="mt-5 flex flex-wrap items-stretch gap-3">
              <WatchNowControl
                progress={progress}
                onPlay={(resumeSeconds) => void play(resumeSeconds)}
                onOpenSources={(resumeSeconds) => openSources(resumeSeconds)}
                sourcesOpen={sourcePicker != null}
                disabled={!canDiscover}
              />
              {media.kind === "movie" && media.availability === "local" ? (
                <DownloadedStatus />
              ) : (
                <button
                  className={buttonClass}
                  type="button"
                  onClick={openDownload}
                >
                  <DownloadIcon /> Download
                </button>
              )}
            </div>
            <PlaybackHint progress={progress} />
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
