export type Surface = "discover" | "movies" | "series";

export type MediaCard = {
  kind: "movie" | "series";
  id: string;
  tmdbId: number;
  title: string;
  overview: string | null;
  year: number | null;
  rating: number | null;
  images: { poster: string | null; backdrop: string | null };
  localCopy: { jellyfinItemId: string } | null;
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
  sections: Array<{
    id: string;
    title: string;
    layout: "poster" | "backdrop";
    href: string | null;
    items: CatalogueItem[];
  }>;
};

export type CollectionResponse = {
  id: string;
  title: string;
  items: CatalogueItem[];
  totalResults: number;
  next: string | null;
};

export type SeriesDetails = {
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
  issues: Array<{
    source: "jellyfin" | "seerr";
    sectionId: string | null;
    code: "upstreamUnavailable";
  }>;
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
  localCopy: { jellyfinItemId: string } | null;
};

export function firstRegularSeason(series: SeriesDetails) {
  return (
    series.seasons.find(
      (season) => season.seasonNumber > 0 && (season.episodeCount ?? 0) > 0,
    ) ??
    series.seasons.find((season) => (season.episodeCount ?? 0) > 0) ??
    series.seasons[0] ??
    null
  );
}

export function reelProxyPathAllowed(path: string) {
  return (
    /^v1\/catalogue\/(?:(?:discover|movies|series)|collections\/[a-z0-9-]+(?:\/[0-9]+)?)$/.test(
      path,
    ) || /^v1\/titles\/series\/[0-9]+(?:\/seasons\/[0-9]+)?$/.test(path)
  );
}

export function collectionHref(apiHref: string) {
  return `/collection?href=${encodeURIComponent(apiHref)}`;
}

export function titleHref(item: MediaCard) {
  const query = new URLSearchParams({
    title: item.title,
    tmdbId: String(item.tmdbId),
  });
  if (item.overview) query.set("overview", item.overview);
  if (item.year != null) query.set("year", String(item.year));
  if (item.rating != null) query.set("rating", String(item.rating));
  if (item.images.poster) query.set("poster", item.images.poster);
  if (item.images.backdrop) query.set("backdrop", item.images.backdrop);
  if (item.localCopy) query.set("local", "true");
  return `/title/${item.kind}/${encodeURIComponent(item.id)}?${query.toString()}`;
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
