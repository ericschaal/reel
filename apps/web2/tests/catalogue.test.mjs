import assert from "node:assert/strict";
import test from "node:test";
import {
  collectionHref,
  collectionProxyHref,
  titleHref,
} from "../app/catalogue.ts";

test("collection links preserve opaque cursors and language", () => {
  const href =
    "/v1/catalogue/collections/studios/2?language=fr&cursor=eyJwYWdlIjoyfQ%3D%3D";
  assert.equal(
    new URL(collectionHref(href), "http://reel.local").searchParams.get("href"),
    href,
  );
  assert.equal(collectionProxyHref(href), `/api/reel${href}`);
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
