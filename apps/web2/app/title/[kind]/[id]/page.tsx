import { notFound } from "next/navigation";
import {
  canonicalTmdbId,
  type MediaCard,
  type MovieDetails,
  type PlaybackProgress,
  type SeriesDetails,
} from "../../../catalogue";
import { TitleDetail } from "./title-detail";

type Query = Record<string, string | string[] | undefined>;

function value(query: Query, key: string) {
  const result = query[key];
  return typeof result === "string" ? result : undefined;
}

function numericValue(query: Query, key: string) {
  const raw = value(query, key);
  if (!raw?.trim()) return null;
  const result = Number(raw);
  return Number.isFinite(result) && result >= 0 ? result : null;
}

function progressValue(query: Query): PlaybackProgress | null {
  const positionSeconds = numericValue(query, "progress");
  const durationSeconds = numericValue(query, "duration");
  const lastSourceKind = value(query, "lastSourceKind");
  const lastSourceId = value(query, "lastSource");
  const lastSourceLabel = value(query, "lastSourceLabel");
  if (
    positionSeconds == null ||
    durationSeconds == null ||
    positionSeconds <= 0 ||
    positionSeconds >= durationSeconds ||
    (lastSourceKind !== "local" && lastSourceKind !== "stream") ||
    !lastSourceId ||
    !lastSourceLabel
  )
    return null;
  return {
    positionSeconds,
    durationSeconds,
    lastSourceId,
    lastSourceLabel,
    lastSourceKind,
    episodeId: value(query, "episodeId"),
  };
}

export default async function TitlePage({
  params,
  searchParams,
}: {
  params: Promise<{ kind: string; id: string }>;
  searchParams: Promise<Query>;
}) {
  const [{ kind, id }, query] = await Promise.all([params, searchParams]);
  if (kind !== "movie" && kind !== "series") notFound();
  const tmdbId = canonicalTmdbId(kind, id);
  if (tmdbId == null) notFound();

  let series: SeriesDetails | null = null;
  let media: MediaCard;
  try {
    if (kind === "series") {
      series = await reelGet<SeriesDetails>(
        `/v1/titles/series/${tmdbId}?language=en`,
      );
      media = {
        kind: "series",
        id: series.id,
        tmdbId: series.tmdbId,
        title: series.title,
        overview: series.overview,
        year: series.year,
        rating: series.rating,
        images: series.images,
        localCopy: null,
        progress: progressValue(query),
      };
    } else {
      const movie = await reelGet<MovieDetails>(
        `/v1/titles/movie/${tmdbId}?language=en`,
      );
      media = { ...movie, kind: "movie", progress: progressValue(query) };
    }
  } catch (reason) {
    if (reason instanceof ReelResponseError && reason.status === 404) notFound();
    throw reason;
  }

  return (
    <TitleDetail
      key={`${kind}-${id}`}
      media={media}
      series={series}
      initialSeason={series?.initialSeason ?? null}
      seriesError={
        series && !series.initialSeason && series.issues.length
          ? "Season and episode details are unavailable right now."
          : null
      }
    />
  );
}

class ReelResponseError extends Error {
  constructor(readonly status: number) {
    super(`Reel API returned ${status}`);
  }
}

async function reelGet<T>(path: string): Promise<T> {
  const apiBaseUrl = process.env.REEL_API_URL ?? "http://localhost:3000";
  const response = await fetch(
    new URL(path.replace(/^\//, ""), `${apiBaseUrl.replace(/\/$/, "")}/`),
    {
      cache: "no-store",
      headers: { accept: "application/json" },
      signal: AbortSignal.timeout(15_000),
    },
  );
  if (!response.ok) throw new ReelResponseError(response.status);
  return response.json() as Promise<T>;
}
