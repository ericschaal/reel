import { notFound } from "next/navigation";
import {
  canonicalTmdbId,
  titleHref,
  type Episode,
  type SeasonDetails,
} from "../../../../catalogue";
import {
  loadTitle,
  numericQueryValue,
  progressValue,
  queryValue,
  ReelResponseError,
  reelGet,
  type TitleQuery,
} from "../title-route";
import type { SourceSelection } from "../playback";
import { PlaybackRoute } from "./playback-route";

function positiveInteger(query: TitleQuery, key: string) {
  const value = numericQueryValue(query, key);
  return value != null && Number.isSafeInteger(value) && value > 0 ? value : null;
}

function playbackSelection(query: TitleQuery): SourceSelection {
  const source = queryValue(query, "source");
  if (!source) return { kind: "auto" };
  if (source === "jellyfin") return { kind: "jellyfin" };
  const discoveryId = queryValue(query, "discovery");
  const candidateId = queryValue(query, "candidate");
  const opaqueId = /^[A-Za-z0-9_-]{32}$/;
  if (
    source !== "aioStreams" ||
    !discoveryId ||
    !candidateId ||
    !opaqueId.test(discoveryId) ||
    !opaqueId.test(candidateId)
  ) {
    notFound();
  }
  return { kind: "aioStreams", discoveryId, candidateId };
}

export default async function PlayPage({
  params,
  searchParams,
}: {
  params: Promise<{ kind: string; id: string }>;
  searchParams: Promise<TitleQuery>;
}) {
  const [{ kind, id }, query] = await Promise.all([params, searchParams]);
  if (kind !== "movie" && kind !== "series") notFound();
  const tmdbId = canonicalTmdbId(kind, id);
  if (tmdbId == null) notFound();

  let title;
  let episode: Episode | undefined;
  try {
    title = await loadTitle(kind, tmdbId);
    if (title.kind === "series") {
      const seasonNumber = positiveInteger(query, "season");
      const episodeNumber = positiveInteger(query, "episode");
      if (seasonNumber == null || episodeNumber == null) notFound();
      const season =
        title.initialSeason?.seasonNumber === seasonNumber
          ? title.initialSeason
          : await reelGet<SeasonDetails>(
              `/v1/titles/series/${tmdbId}/seasons/${seasonNumber}?language=en`,
            );
      episode = season.episodes.find(
        (candidate) => candidate.episodeNumber === episodeNumber,
      );
      if (!episode) notFound();
    }
  } catch (reason) {
    if (reason instanceof ReelResponseError && reason.status === 404) notFound();
    throw reason;
  }

  const progress = progressValue(query);
  const resumeSeconds = numericQueryValue(query, "start") ?? undefined;
  return (
    <PlaybackRoute
      media={{ ...title, progress }}
      episode={episode}
      resumeSeconds={resumeSeconds}
      sourceSelection={playbackSelection(query)}
      backHref={titleHref({ ...title, progress })}
    />
  );
}
