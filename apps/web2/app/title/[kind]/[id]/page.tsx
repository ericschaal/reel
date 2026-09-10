import { notFound } from "next/navigation";
import {
  firstRegularSeason,
  type MediaCard,
  type SeasonDetails,
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

function imageValue(query: Query, key: string) {
  const raw = value(query, key);
  if (!raw) return null;
  try {
    const url = new URL(raw);
    return ["http:", "https:"].includes(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
}

export default async function TitlePage({
  params,
  searchParams,
}: {
  params: Promise<{ kind: string; id: string }>;
  searchParams: Promise<Query>;
}) {
  const [{ kind, id }, query] = await Promise.all([params, searchParams]);
  const tmdbId = numericValue(query, "tmdbId") ?? 0;
  if (
    (kind !== "movie" && kind !== "series") ||
    (kind === "movie" && !value(query, "title")) ||
    (kind === "series" && tmdbId <= 0)
  )
    notFound();

  let series: SeriesDetails | null = null;
  let initialSeason: SeasonDetails | null = null;
  let seriesError: string | null = null;
  if (kind === "series") {
    try {
      series = await reelGet<SeriesDetails>(
        `/v1/titles/series/${tmdbId}?language=en`,
      );
      const firstSeason = firstRegularSeason(series);
      if (firstSeason) {
        initialSeason = await reelGet<SeasonDetails>(
          `/v1/titles/series/${tmdbId}/seasons/${firstSeason.seasonNumber}?language=en`,
        );
      }
    } catch (reason) {
      if (reason instanceof ReelResponseError && reason.status === 404) notFound();
      seriesError = "Season and episode details are unavailable right now.";
    }
  }

  const fallbackMedia: MediaCard = {
    kind,
    id,
    tmdbId,
    title: value(query, "title") ?? "Series",
    overview: value(query, "overview") ?? null,
    year: numericValue(query, "year"),
    rating: numericValue(query, "rating"),
    images: {
      poster: imageValue(query, "poster"),
      backdrop: imageValue(query, "backdrop"),
    },
    localCopy:
      value(query, "local") === "true" ? { jellyfinItemId: "available" } : null,
  };
  const media: MediaCard = series
    ? {
        kind: "series",
        id: series.id,
        tmdbId: series.tmdbId,
        title: series.title,
        overview: series.overview,
        year: series.year,
        rating: series.rating,
        images: series.images,
        localCopy: null,
      }
    : fallbackMedia;

  return (
    <TitleDetail
      key={`${kind}-${id}`}
      media={media}
      series={series}
      initialSeason={initialSeason}
      seriesError={seriesError}
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
