export type Surface = "discover" | "movies" | "series";

export type MediaCard = {
  runtimeMinutes: number | null;
  numberOfSeasons: number | null;
  kind: "movie" | "series";
  id: string;
  tmdbId: number;
  title: string;
  year: number | null;
  rating: number | null;
  images: { poster: string | null; backdrop: string | null };
  availability: Availability;
  // Existing client prototype state; not returned by the catalogue API.
  progress?: PlaybackProgress | null;
};

export type TitleMedia = Omit<MediaCard, "runtimeMinutes" | "numberOfSeasons"> & {
  overview: string | null;
};

export type Availability = "local" | "notLocal" | "unknown" | "episodeBased";
export type CatalogueIssue = {
  source: "jellyfin" | "seerr";
  sectionId: string | null;
  code: "upstreamUnavailable";
};

export type TitleSummary =
  | { kind: "movie"; id: string; runtimeMinutes: number | null }
  | { kind: "series"; id: string; numberOfSeasons: number | null };

export type TitleSummariesResponse = {
  items: TitleSummary[];
  issues: Array<{ id: string; code: "media_not_found" | "catalogue_unavailable" }>;
};

export type PlaybackProgress = {
  positionSeconds: number;
  durationSeconds: number;
  lastSourceId: string;
  lastSourceLabel: string;
  lastSourceKind: "local" | "stream";
  episodeId?: string;
};

export type CategoryCard = {
  kind: "category";
  id: number;
  title: string;
  mediaKind: "movie" | "series";
  categoryKind: "genre" | "studio" | "network";
  href: string;
  images: string[];
};

export type CatalogueItem = MediaCard | CategoryCard;

export type CatalogueResponse = {
  surface: Surface;
  issues: CatalogueIssue[];
  sections: Array<{
    id: string;
    title: string;
    layout: "poster" | "backdrop";
    href: string | null;
    items: CatalogueItem[];
  }>;
};

export type CatalogueSection = CatalogueResponse["sections"][number];

export type CatalogueManifest = {
  surface: Surface;
  rails: Array<{
    id: string;
    title: string;
    layout: "poster" | "backdrop";
    itemsHref: string;
    itemCountHint: number;
  }>;
};

export type CatalogueRailResponse = {
  section: CatalogueSection;
  issues: CatalogueIssue[];
};

export type CollectionResponse = {
  issues: CatalogueIssue[];
  id: string;
  title: string;
  items: CatalogueItem[];
  totalResults: number;
  next: string | null;
};

export type SeriesDetails = {
  kind: "series";
  availability: "episodeBased";
  id: string;
  tmdbId: number;
  title: string;
  overview: string | null;
  year: number | null;
  rating: number | null;
  numberOfSeasons: number | null;
  numberOfEpisodes: number | null;
  images: { poster: string | null; backdrop: string | null };
  seasons: SeasonSummary[];
  issues: CatalogueIssue[];
  initialSeason: SeasonDetails | null;
};

export type MovieDetails = Omit<TitleMedia, "kind" | "progress"> & {
  kind: "movie";
  issues: CatalogueIssue[];
  runtimeMinutes: number | null;
};

export type SeasonSummary = {
  id: string;
  seasonNumber: number;
  title: string;
  overview: string | null;
  airDate: string | null;
  episodeCount: number | null;
  poster: string | null;
};

export type SeasonDetails = {
  id: string;
  seriesTmdbId: number;
  seasonNumber: number;
  title: string;
  overview: string | null;
  airDate: string | null;
  poster: string | null;
  episodes: Episode[];
  issues: CatalogueIssue[];
};

export type Episode = {
  id: string;
  tmdbId: number;
  seasonNumber: number;
  episodeNumber: number;
  title: string;
  overview: string | null;
  airDate: string | null;
  rating: number | null;
  still: string | null;
  runtimeMinutes: number | null;
  availability: Availability;
};

export function reelProxyPathAllowed(path: string) {
  return (
    path === "v1/titles/summaries" ||
    /^v1\/catalogue\/(?:(?:discover|movies|series)(?:\/manifest|\/rails\/[a-z0-9-]+)?|collections\/[a-z0-9-]+(?:\/[0-9]+)?)$/.test(
      path,
    ) || /^v1\/titles\/(?:movie\/[0-9]+|series\/[0-9]+(?:\/seasons\/[0-9]+)?)$/.test(path)
  );
}

export function collectionHref(apiHref: string) {
  return `/collection?href=${encodeURIComponent(apiHref)}`;
}

export function titleHref(item: Pick<MediaCard, "kind" | "id" | "progress">) {
  const query = new URLSearchParams();
  if (item.progress) {
    query.set("progress", String(item.progress.positionSeconds));
    query.set("duration", String(item.progress.durationSeconds));
    query.set("lastSource", item.progress.lastSourceId);
    query.set("lastSourceLabel", item.progress.lastSourceLabel);
    query.set("lastSourceKind", item.progress.lastSourceKind);
    if (item.progress.episodeId) query.set("episodeId", item.progress.episodeId);
  }
  const suffix = query.size ? `?${query.toString()}` : "";
  return `/title/${item.kind}/${encodeURIComponent(item.id)}${suffix}`;
}

export function playbackHref(
  media: TitleMedia,
  episode?: Episode,
  resumeSeconds?: number,
) {
  const url = new URL(titleHref(media), "http://reel.local");
  url.pathname = `${url.pathname}/play`;
  if (resumeSeconds != null && Number.isFinite(resumeSeconds) && resumeSeconds > 0) {
    url.searchParams.set("start", String(resumeSeconds));
  }
  if (episode) {
    url.searchParams.set("season", String(episode.seasonNumber));
    url.searchParams.set("episode", String(episode.episodeNumber));
  }
  return `${url.pathname}${url.search}`;
}

export function canonicalTmdbId(kind: MediaCard["kind"], id: string) {
  let canonicalId: string;
  try {
    canonicalId = decodeURIComponent(id);
  } catch {
    return null;
  }
  const match = new RegExp(`^tmdb:${kind}:([1-9][0-9]*)$`).exec(canonicalId);
  if (!match) return null;
  const value = Number(match[1]);
  return Number.isSafeInteger(value) ? value : null;
}

// Accept only relative collection routes, including opaque pagination queries.
export function collectionProxyHref(href: string) {
  if (!href.startsWith("/v1/catalogue/collections/")) return null;
  const url = new URL(href, "http://reel.local");
  if (
    url.origin !== "http://reel.local" ||
    !url.pathname.startsWith("/v1/catalogue/collections/") ||
    url.hash
  )
    return null;
  return `/api/reel${url.pathname}${url.search}`;
}

export function catalogueRailProxyHref(href: string) {
  if (!href.startsWith("/v1/catalogue/")) return null;
  const url = new URL(href, "http://reel.local");
  if (
    url.origin !== "http://reel.local" ||
    !/^\/v1\/catalogue\/(?:discover|movies|series)\/rails\/[a-z0-9-]+$/.test(
      url.pathname,
    ) ||
    url.hash
  )
    return null;
  return `/api/reel${url.pathname}${url.search}`;
}
