import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  canonicalTmdbId,
  collectionHref,
  catalogueRailProxyHref,
  collectionProxyHref,
  reelProxyPathAllowed,
  titleHref,
} from "../app/catalogue.ts";

test("category artwork keeps an explicit non-zero sizing chain", async () => {
  const source = await readFile(
    new URL("../app/media-card.tsx", import.meta.url),
    "utf8",
  );
  const categoryBranch = source.slice(
    source.indexOf('if (item.kind === "category")'),
    source.indexOf('const backdrop = layout === "backdrop"'),
  );

  assert.match(categoryBranch, /relative block aspect-\[4\/3\] w-full/);
  assert.doesNotMatch(categoryBranch, /self-start/);
  assert.match(
    categoryBranch,
    /absolute inset-x-\[12%\] top-\[9%\] bottom-\[27%\] opacity-35/,
  );
});

test("media facts and rating occupy separate artwork zones", async () => {
  const source = await readFile(
    new URL("../app/media-card.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    source,
    /absolute top-2\.5 left-2\.5 text-white/,
  );
  assert.match(source, /absolute inset-x-0 bottom-0 flex bg-linear-to-t/);
  assert.match(source, /variant="overlay"/);
});

test("library availability uses an accessible download icon", async () => {
  const source = await readFile(
    new URL("../app/media-card.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /aria-label="In library"/);
  assert.match(source, /<DownloadIcon \/>/);
  assert.match(source, /inline-flex text-accent/);
  assert.doesNotMatch(source, /absolute top-2\.5 right-2\.5/);
  assert.doesNotMatch(source, /text-emerald-100|bg-emerald-300/);
});

test("episode rows keep content stationary and promote the air date", async () => {
  const source = await readFile(
    new URL("../app/title/[kind]/[id]/episodes.tsx", import.meta.url),
    "utf8",
  );
  const episodeList = source.slice(
    source.indexOf("export function SeriesHierarchy"),
    source.indexOf("export function SeasonSelector"),
  );

  assert.doesNotMatch(episodeList, /group-hover:translate-x/);
  assert.match(source, /<CalendarIcon \/>/);
  assert.match(source, /formatAirDate\(episode\.airDate\)/);
});

test("episode rail constrains long titles to the card width", async () => {
  const source = await readFile(
    new URL("../app/title/[kind]/[id]/episodes.tsx", import.meta.url),
    "utf8",
  );

  assert.match(source, /className="min-w-0 flex items-baseline gap-2"/);
  assert.match(
    source,
    /className="min-w-0 flex-1 truncate text-sm font-semibold/,
  );
});

test("opening an episode does not force the page scroll position", async () => {
  const source = await readFile(
    new URL("../app/title/[kind]/[id]/title-detail.tsx", import.meta.url),
    "utf8",
  );

  assert.doesNotMatch(source, /window\.scrollTo|seriesScrollPosition/);
});

test("all secondary pages use the shared navigation header", async () => {
  const [ui, collection, title] = await Promise.all([
    readFile(new URL("../app/ui.tsx", import.meta.url), "utf8"),
    readFile(
      new URL("../app/collection/collection-browser.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/title-detail.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(ui, /export function NavigationHeader/);
  assert.match(
    collection,
    /<NavigationHeader href="\/" label="Back to catalogue" \/>/,
  );
  assert.match(title, /<NavigationHeader href="\/" label="Back to catalogue" \/>/);
  assert.doesNotMatch(title, /function NavigationHeader/);
});

test("collection links preserve opaque cursors and language", () => {
  const href =
    "/v1/catalogue/collections/studios/2?language=fr&cursor=eyJwYWdlIjoyfQ%3D%3D";
  assert.equal(
    new URL(collectionHref(href), "http://reel.local").searchParams.get("href"),
    href,
  );
  assert.equal(collectionProxyHref(href), `/api/reel${href}`);
});

test("rail proxy accepts only relative catalogue rail URLs", () => {
  assert.equal(
    catalogueRailProxyHref(
      "/v1/catalogue/discover/rails/trending?language=fr-FR",
    ),
    "/api/reel/v1/catalogue/discover/rails/trending?language=fr-FR",
  );
  for (const href of [
    "https://example.com/v1/catalogue/discover/rails/trending",
    "/v1/catalogue/discover/rails/../../admin",
    "/v1/catalogue/unknown/rails/trending",
    "/v1/catalogue/discover/rails/trending#fragment",
  ]) {
    assert.equal(catalogueRailProxyHref(href), null, href);
  }
});

test("collection proxy rejects external and escaping paths", () => {
  for (const href of [
    "",
    "https://example.com/v1/catalogue/collections/movies",
    "//example.com",
    "/v1/catalogue/collections/../../admin",
    "/v1/catalogue/collections/%2e%2e/%2e%2e/admin",
    "/v1/catalogue/collections/movies#fragment",
  ]) {
    assert.equal(collectionProxyHref(href), null, href);
  }
});

test("title links carry only canonical identity, not duplicated media facts", () => {
  const item = {
    kind: "movie",
    id: "tmdb:movie:123",
    tmdbId: 123,
    title: "A & B / The Return?",
    overview: "A story with #symbols & details.",
    year: 2026,
    rating: 0,
    images: { poster: null, backdrop: null },
    localCopy: null,
  };
  const url = new URL(titleHref(item), "http://reel.local");
  assert.equal(decodeURIComponent(url.pathname), "/title/movie/tmdb:movie:123");
  assert.equal(url.search, "");
});

test("title routes accept only matching canonical TMDB identities", () => {
  assert.equal(canonicalTmdbId("movie", "tmdb:movie:123"), 123);
  assert.equal(canonicalTmdbId("movie", "tmdb%3Amovie%3A123"), 123);
  assert.equal(canonicalTmdbId("series", "tmdb:series:123"), 123);
  assert.equal(canonicalTmdbId("movie", "tmdb:series:123"), null);
  assert.equal(canonicalTmdbId("movie", "movie-123"), null);
  assert.equal(canonicalTmdbId("movie", "tmdb:movie:0"), null);
  assert.equal(canonicalTmdbId("movie", "%E0%A4%A"), null);
});

test("title links carry progress and remember the last source as a preference", () => {
  const url = new URL(
    titleHref({
      kind: "movie",
      id: "tmdb:movie:1",
      tmdbId: 1,
      title: "Movie",
      overview: null,
      year: 2026,
      rating: null,
      images: { poster: null, backdrop: null },
      localCopy: null,
      progress: {
        positionSeconds: 3720,
        durationSeconds: 9960,
        lastSourceId: "stremio-2",
        lastSourceLabel: "Stremio · 2",
        lastSourceKind: "stream",
      },
    }),
    "http://reel.local",
  );

  assert.equal(url.searchParams.get("progress"), "3720");
  assert.equal(url.searchParams.get("duration"), "9960");
  assert.equal(url.searchParams.get("lastSource"), "stremio-2");
  assert.equal(url.searchParams.get("lastSourceLabel"), "Stremio · 2");
  assert.equal(url.searchParams.get("lastSourceKind"), "stream");
});

test("proxy permits only supported title detail routes", () => {
  assert.equal(reelProxyPathAllowed("v1/titles/movie/123"), true);
  assert.equal(reelProxyPathAllowed("v1/titles/series/123"), true);
  assert.equal(
    reelProxyPathAllowed("v1/titles/series/123/seasons/2"),
    true,
  );
  assert.equal(reelProxyPathAllowed("v1/titles/series/abc"), false);
  assert.equal(reelProxyPathAllowed("v1/titles/movie/abc"), false);
  assert.equal(reelProxyPathAllowed("v1/titles/series/123/episodes"), false);
});


test("media cards render rail facts without additional data requests", async () => {
  const source = await readFile(new URL("../app/media-card.tsx", import.meta.url), "utf8");
  assert.match(source, /formatRuntime\(item.runtimeMinutes\)/);
  assert.match(source, /formatSeasons\(item.numberOfSeasons\)/);
  assert.doesNotMatch(source, /useQuery|useIntersectionObserver|titleSummaryQuery|fetch\(/);
});
