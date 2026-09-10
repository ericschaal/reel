import { notFound } from "next/navigation";
import type { MediaCard } from "../../../catalogue";
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
  if ((kind !== "movie" && kind !== "series") || !value(query, "title"))
    notFound();

  const media: MediaCard = {
    kind,
    id,
    tmdbId: numericValue(query, "tmdbId") ?? 0,
    title: value(query, "title")!,
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

  return <TitleDetail key={`${kind}-${id}`} media={media} />;
}
