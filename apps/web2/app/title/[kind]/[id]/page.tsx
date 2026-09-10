import { notFound } from "next/navigation";
import {
  canonicalTmdbId,
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

  let title: MovieDetails | SeriesDetails;
  try {
    if (kind === "series") {
      title = await reelGet<SeriesDetails>(
        `/v1/titles/series/${tmdbId}?language=en&include=initialSeason`,
      );
    } else {
      title = await reelGet<MovieDetails>(
        `/v1/titles/movie/${tmdbId}?language=en`,
      );
    }
  } catch (reason) {
    if (reason instanceof ReelResponseError && reason.status === 404) notFound();
    throw reason;
  }

  return (
    <TitleDetail
      key={`${kind}-${id}`}
      title={title}
      progress={progressValue(query)}
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
