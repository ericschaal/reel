import { CatalogueBrowser } from "./catalogue-browser";
import type { Surface } from "./catalogue";

export default async function Home({
  searchParams,
}: {
  searchParams: Promise<{ surface?: string }>;
}) {
  const { surface: query } = await searchParams;
  const surface: Surface =
    query === "movies" || query === "series" ? query : "discover";
  return <CatalogueBrowser key={surface} surface={surface} />;
}
