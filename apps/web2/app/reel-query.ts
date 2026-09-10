import { infiniteQueryOptions, queryOptions } from "@tanstack/react-query";
import {
  catalogueRailProxyHref,
  collectionProxyHref,
  type CatalogueManifest,
  type CatalogueRailResponse,
  type CollectionResponse,
  type SeasonDetails,
  type Surface,
} from "./catalogue";

const catalogueStaleTime = 5 * 60 * 1000;
const availabilityStaleTime = 30 * 1000;
async function reelJson<T>(url: string, signal: AbortSignal): Promise<T> {
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Reel request failed (${response.status})`);
  return response.json() as Promise<T>;
}

export function catalogueManifestQuery(surface: Surface) {
  return queryOptions({
    queryKey: ["reel", "catalogue", surface, "manifest", "en"] as const,
    queryFn: ({ signal }) =>
      reelJson<CatalogueManifest>(
        `/api/reel/v1/catalogue/${surface}/manifest?language=en`,
        signal,
      ),
    staleTime: catalogueStaleTime,
  });
}

export function catalogueRailQuery(itemsHref: string) {
  return queryOptions({
    queryKey: ["reel", "catalogue", "rail", itemsHref] as const,
    queryFn: ({ signal }) => {
      const url = catalogueRailProxyHref(itemsHref);
      if (!url) throw new Error("This catalogue rail has an invalid URL.");
      return reelJson<CatalogueRailResponse>(url, signal);
    },
    staleTime: availabilityStaleTime,
  });
}

export function seasonQuery(
  seriesTmdbId: number,
  seasonNumber: number,
) {
  return queryOptions({
    queryKey: [
      "reel",
      "series",
      seriesTmdbId,
      "season",
      seasonNumber,
      "en",
    ] as const,
    queryFn: ({ signal }) =>
      reelJson<SeasonDetails>(
        `/api/reel/v1/titles/series/${seriesTmdbId}/seasons/${seasonNumber}?language=en`,
        signal,
      ),
    staleTime: availabilityStaleTime,
  });
}

export function collectionQuery(initialHref: string) {
  return infiniteQueryOptions({
    queryKey: ["reel", "collection", initialHref] as const,
    queryFn: ({ pageParam, signal }) => {
      const url = collectionProxyHref(pageParam);
      if (!url) throw new Error("This collection link is invalid.");
      return reelJson<CollectionResponse>(url, signal);
    },
    initialPageParam: initialHref,
    getNextPageParam: (lastPage) => lastPage.next ?? undefined,
    staleTime: availabilityStaleTime,
  });
}
