import { queryOptions } from "@tanstack/react-query";
import type { Availability, MovieDetails } from "./catalogue";

export type RequestStatus =
  | "none"
  | "pending"
  | "approved"
  | "declined"
  | "failed"
  | "completed"
  | "unknown";

export type AcquisitionStatus =
  | "unknown"
  | "pending"
  | "processing"
  | "partiallyAvailable"
  | "available"
  | "blocklisted"
  | "deleted";

export type ItemRequestStatus = {
  requestStatus: RequestStatus;
  acquisitionStatus: AcquisitionStatus;
  transferStatus?: "waiting" | "queued" | "downloading" | "importing" | "attention";
  requestedAt?: string;
};

export type MediaRequestStatus = ItemRequestStatus & {
  kind: "movie" | "series";
  tmdbId: number;
  seasons: Array<ItemRequestStatus & { seasonNumber: number }>;
};

export type CreateMediaRequest = {
  kind: "movie" | "series";
  tmdbId: number;
  seasonNumbers?: number[];
  profileId: number;
};

export type RequestProfiles = {
  serverId: number;
  serverName: string;
  defaultProfileId: number;
  profiles: Array<{ id: number; name: string }>;
};

export function requestProfilesQuery(kind: "movie" | "series") {
  return queryOptions({
    queryKey: ["reel", "requests", "profiles", kind] as const,
    queryFn: async ({ signal }): Promise<RequestProfiles> => {
      const response = await fetch(`/api/reel/v1/requests/profiles/${kind}`, { signal });
      if (!response.ok) throw new Error(await responseMessage(response));
      return response.json() as Promise<RequestProfiles>;
    },
    staleTime: 60_000,
    refetchOnWindowFocus: true,
  });
}

export function requestStatusQuery(kind: "movie" | "series", tmdbId: number) {
  return queryOptions({
    queryKey: ["reel", "requests", kind, tmdbId] as const,
    queryFn: async ({ signal }): Promise<MediaRequestStatus> => {
      const response = await fetch(`/api/reel/v1/requests/${kind}/${tmdbId}`, { signal });
      if (!response.ok) throw new Error(await responseMessage(response));
      return response.json() as Promise<MediaRequestStatus>;
    },
    staleTime: 15_000,
    refetchOnWindowFocus: true,
    refetchInterval: (query) => {
      const status = query.state.data;
      if (!status) return false;
      const active = [status, ...status.seasons].some(requestNeedsAvailabilityRefresh);
      return active ? 30_000 : false;
    },
  });
}

export function movieAvailabilityQuery(tmdbId: number) {
  return queryOptions({
    queryKey: ["reel", "movie-availability", tmdbId] as const,
    queryFn: async ({ signal }): Promise<Availability> => {
      const response = await fetch(`/api/reel/v1/titles/movie/${tmdbId}?language=en`, { signal });
      if (!response.ok) throw new Error(await responseMessage(response));
      const movie = (await response.json()) as MovieDetails;
      return movie.availability;
    },
    staleTime: 0,
    refetchOnWindowFocus: true,
  });
}

export async function createMediaRequest(input: CreateMediaRequest) {
  const response = await fetch("/api/reel/v1/requests", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(input),
  });
  if (!response.ok) throw new Error(await responseMessage(response));
  return response.json() as Promise<{ id: number; requestStatus: RequestStatus }>;
}

async function responseMessage(response: Response) {
  try {
    const body = (await response.json()) as { error?: { message?: string } };
    if (body.error?.message) return body.error.message;
  } catch {
    // A malformed error response still gets a useful fallback.
  }
  return `Reel request failed (${response.status})`;
}

export function requestIsBlocked(item: ItemRequestStatus | undefined) {
  if (!item) return false;
  return (
    item.requestStatus === "pending" ||
    item.requestStatus === "approved" ||
    item.requestStatus === "failed" ||
    item.requestStatus === "completed" ||
    item.acquisitionStatus === "pending" ||
    item.acquisitionStatus === "processing" ||
    item.acquisitionStatus === "partiallyAvailable" ||
    item.acquisitionStatus === "available" ||
    item.acquisitionStatus === "blocklisted"
  );
}

export function requestNeedsAvailabilityRefresh(item: ItemRequestStatus | undefined) {
  if (!item || item.requestStatus === "failed" || item.requestStatus === "declined") return false;
  return (
    item.requestStatus === "pending" ||
    item.requestStatus === "approved" ||
    item.requestStatus === "completed" ||
    item.acquisitionStatus === "pending" ||
    item.acquisitionStatus === "processing" ||
    item.acquisitionStatus === "partiallyAvailable" ||
    item.acquisitionStatus === "available"
  );
}

export function requestLabel(item: ItemRequestStatus | undefined) {
  if (!item) return null;
  if (item.requestStatus === "failed") return "Request failed in Seerr";
  if (item.requestStatus === "declined") return "Request declined in Seerr";
  if (item.acquisitionStatus === "blocklisted") return "Blocked";
  if (item.transferStatus === "attention") return "Download needs attention";
  if (item.transferStatus === "importing") return "Importing download";
  if (item.transferStatus === "downloading") return "Downloading";
  if (item.transferStatus === "queued") return "Queued for download";
  if (item.acquisitionStatus === "available") return "Waiting for Jellyfin";
  if (item.acquisitionStatus === "partiallyAvailable") return "Partially available";
  if (item.acquisitionStatus === "processing") return item.transferStatus === "waiting" ? "Waiting for a copy" : "Request in progress";
  if (item.requestStatus === "pending") return "Awaiting approval";
  if (item.requestStatus === "approved" || item.acquisitionStatus === "pending")
    return "Request received";
  if (item.requestStatus === "completed") return "Waiting for Jellyfin";
  return null;
}
