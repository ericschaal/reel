import { CollectionBrowser } from "./collection-browser";

export default async function CollectionPage({
  searchParams,
}: {
  searchParams: Promise<{ href?: string | string[] }>;
}) {
  const query = await searchParams;
  const href = typeof query.href === "string" ? query.href : "";
  return <CollectionBrowser key={href} initialHref={href} />;
}
