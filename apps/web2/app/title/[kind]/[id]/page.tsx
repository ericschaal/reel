import { notFound } from "next/navigation";
import {
  canonicalTmdbId,
} from "../../../catalogue";
import { TitleDetail } from "./title-detail";
import {
  loadTitle,
  progressValue,
  ReelResponseError,
  type TitleQuery,
} from "./title-route";

export default async function TitlePage({
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
  try {
    title = await loadTitle(kind, tmdbId);
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
