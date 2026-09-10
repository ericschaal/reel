import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  collectionHref,
  catalogueRailProxyHref,
  collectionProxyHref,
  firstRegularSeason,
  mapWithConcurrency,
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

test("media metadata grid packs its rows instead of stretching", async () => {
  const source = await readFile(
    new URL("../app/media-card.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    source,
    /<span className="grid min-w-0 content-start gap-2">/,
  );
  assert.doesNotMatch(
    source,
    /backdrop \? "line-clamp-1" : "line-clamp-2 min-h-10"/,
  );
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

test("progressive rail work respects its concurrency limit", async () => {
  let active = 0;
  let peak = 0;
  const completed = [];
  await mapWithConcurrency([0, 1, 2, 3, 4, 5], 3, async (item) => {
    active += 1;
    peak = Math.max(peak, active);
    await new Promise((resolve) => setTimeout(resolve, 2));
    completed.push(item);
    active -= 1;
  });
  assert.equal(peak, 3);
  assert.deepEqual(completed.toSorted(), [0, 1, 2, 3, 4, 5]);
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

test("title links round-trip reserved characters and a zero rating", () => {
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
  assert.equal(url.searchParams.get("title"), item.title);
  assert.equal(url.searchParams.get("overview"), item.overview);
  assert.equal(url.searchParams.get("rating"), "0");
  assert.equal(url.searchParams.has("local"), false);
});

test("series selects the first populated regular season before specials", () => {
  const series = {
    seasons: [
      { id: "specials", seasonNumber: 0, episodeCount: 3 },
      { id: "empty", seasonNumber: 1, episodeCount: 0 },
      { id: "season-two", seasonNumber: 2, episodeCount: 8 },
    ],
  };
  assert.equal(firstRegularSeason(series)?.seasonNumber, 2);
});

test("proxy permits only supported series hierarchy routes", () => {
  assert.equal(reelProxyPathAllowed("v1/titles/series/123"), true);
  assert.equal(
    reelProxyPathAllowed("v1/titles/series/123/seasons/2"),
    true,
  );
  assert.equal(reelProxyPathAllowed("v1/titles/series/abc"), false);
  assert.equal(reelProxyPathAllowed("v1/titles/series/123/episodes"), false);
});
