import type {
  MovieDetails,
  PlaybackProgress,
  SeriesDetails,
} from "../../../catalogue";

export type TitleQuery = Record<string, string | string[] | undefined>;

export function queryValue(query: TitleQuery, key: string) {
  const result = query[key];
  return typeof result === "string" ? result : undefined;
}

export function numericQueryValue(query: TitleQuery, key: string) {
  const raw = queryValue(query, key);
  if (!raw?.trim()) return null;
  const result = Number(raw);
  return Number.isFinite(result) && result >= 0 ? result : null;
}

export function progressValue(query: TitleQuery): PlaybackProgress | null {
  const positionSeconds = numericQueryValue(query, "progress");
  const durationSeconds = numericQueryValue(query, "duration");
  const lastSourceKind = queryValue(query, "lastSourceKind");
  const lastSourceId = queryValue(query, "lastSource");
  const lastSourceLabel = queryValue(query, "lastSourceLabel");
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
    episodeId: queryValue(query, "episodeId"),
  };
}

export class ReelResponseError extends Error {
  constructor(readonly status: number) {
    super(`Reel API returned ${status}`);
  }
}

export async function reelGet<T>(path: string): Promise<T> {
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

export async function loadTitle(kind: "movie" | "series", tmdbId: number) {
  if (kind === "series") {
    return reelGet<SeriesDetails>(
      `/v1/titles/series/${tmdbId}?language=en&include=initialSeason`,
    );
  }
  return reelGet<MovieDetails>(`/v1/titles/movie/${tmdbId}?language=en`);
}
