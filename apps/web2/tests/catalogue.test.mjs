import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import {
  canonicalTmdbId,
  collectionHref,
  catalogueRailProxyHref,
  collectionProxyHref,
  playbackHref,
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

test("catalogue cards share one hover and keyboard-focus treatment", async () => {
  const [cards, styles] = await Promise.all([
    readFile(new URL("../app/media-card.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/globals.css", import.meta.url), "utf8"),
  ]);

  assert.match(cards, /interactive-card/);
  assert.match(cards, /interactive-card-title/);
  assert.match(styles, /data-input-modality="pointer"/);
  assert.match(styles, /data-input-modality="keyboard"/);
  assert.match(
    styles,
    /:root:not\(\[data-input-modality\]\) \.interactive-card:focus-visible/,
  );
  assert.match(styles, /\.interactive-card:focus-visible\s*\{[^}]*outline: none/s);
  assert.doesNotMatch(
    styles,
    /\.interactive-card[^,{]*:is\(:hover, :focus-visible\)[^{]*\{[^}]*border/s,
  );
  assert.match(styles, /prefers-reduced-motion: no-preference/);
});

test("catalogue menu switches immediately with keyboard focus and preserves navigation state", async () => {
  const [browser, page, ui, styles] = await Promise.all([
    readFile(new URL("../app/catalogue-browser.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/page.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/ui.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/globals.css", import.meta.url), "utf8"),
  ]);

  assert.match(browser, /useRouter\(\)/);
  assert.match(browser, /useState<NavigationState>/);
  assert.match(browser, /pendingSurface: item\.id/);
  assert.match(browser, /from "motion\/react"/);
  assert.match(browser, /CATALOGUE_SWAP_DELAY_MS = 180/);
  assert.match(browser, /useStagedSurface\(visualSurface\)/);
  assert.match(browser, /window\.setTimeout\([\s\S]*?CATALOGUE_SWAP_DELAY_MS,\s*\)/);
  assert.match(
    browser,
    /<CatalogueContent key=\{contentSurface\} surface=\{contentSurface\} \/>/,
  );
  assert.match(browser, /layoutId="catalogue-active-indicator"/);
  assert.match(browser, /className="h-0\.5 w-7/);
  assert.match(browser, /pendingSurface \?\? surface/);
  assert.match(browser, /<MotionConfig\s+reducedMotion="user"/);
  assert.match(browser, /data-keyboard-menu="true"/);
  assert.match(browser, /tabIndex=\{surface === item\.id \? 0 : -1\}/);
  assert.match(browser, /dataset\.keyboardFocusDirection/);
  assert.match(browser, /direction === "left" \|\| direction === "right"/);
  assert.match(browser, /router\.replace\([^;]+scroll: false/s);
  assert.match(browser, /onNavigate=\{\(event\) =>/);
  assert.match(browser, /event\.preventDefault\(\)/);
  assert.match(browser, /router\.push\([^;]+scroll: false/s);
  assert.doesNotMatch(browser, /createDebouncedPublisher/);
  assert.doesNotMatch(browser, /MENU_FOCUS_DELAY_MS/);
  assert.doesNotMatch(browser, /useOptimistic|startTransition/);
  assert.match(browser, /\[content-visibility:auto\]/);
  assert.doesNotMatch(ui, /catalogueNavItemClass\s*=\s*[^;]+interactive-card/s);
  assert.doesNotMatch(browser, /border-r/);
  assert.doesNotMatch(page, /<CatalogueBrowser key=\{surface\}/);
  assert.doesNotMatch(ui, /catalogueNavItemClass[^;]+border/s);
  assert.match(ui, /catalogueNavItemClass[^;]+hover:text-white/s);
  assert.match(ui, /catalogueNavItemClass[^;]+focus-visible:text-nav-accent/s);
  assert.match(styles, /--color-nav-accent: #ffd166/);
  assert.doesNotMatch(ui, /catalogueNavGroupClass[^;]+(?:rounded|border|shadow|backdrop)/s);
  assert.doesNotMatch(ui, /catalogueNavItemClass[^;]+rounded/s);
  assert.doesNotMatch(styles, /\.catalogue-nav(?:-item)?\s*[{:]/);
  assert.doesNotMatch(browser, /::after|data-visual-current/);
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

test("download actions become downloaded status for local media", async () => {
  const [titleDetail, episodeDetail, playback] = await Promise.all([
    readFile(
      new URL("../app/title/[kind]/[id]/title-detail.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/episode-detail-view.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/playback.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(playback, /export function DownloadedStatus/);
  assert.match(playback, /<CheckIcon \/> Downloaded/);
  assert.match(
    titleDetail,
    /media\.kind === "movie" && media\.availability === "local"/,
  );
  assert.match(titleDetail, /<DownloadedStatus \/>/);
  assert.match(episodeDetail, /episode\.availability === "local"/);
  assert.match(episodeDetail, /<DownloadedStatus \/>/);
});

test("movies and exact episodes activate normalized playback through the existing player route", async () => {
  const [playback, playbackRoute, videoPlayer, titleDetail, episodeDetail, nextConfig] = await Promise.all([
    readFile(
      new URL("../app/title/[kind]/[id]/playback.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/play/playback-route.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/video-player.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/title-detail.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/episode-detail-view.tsx", import.meta.url),
      "utf8",
    ),
    readFile(new URL("../next.config.ts", import.meta.url), "utf8"),
  ]);

  assert.match(playback, /fetch\("\/v1\/playback\/activate"/);
  assert.match(playback, /fetch\("\/v1\/playback\/sources"/);
  assert.match(playback, /seriesTmdbId: media\.tmdbId/);
  assert.match(playback, /episodeNumber: episode\.episodeNumber/);
  assert.match(playbackRoute, /activatePlayback/);
  assert.match(titleDetail, /playbackHref\(media, episode, resumeSeconds\)/);
  assert.match(videoPlayer, /void import\("hls\.js"\)/);
  assert.match(videoPlayer, /<video/);
  assert.equal(videoPlayer.match(/<video/g)?.length, 1);
  assert.doesNotMatch(playback, /<video/);
  assert.doesNotMatch(`${playback}${videoPlayer}`, /exampleSources/);
  assert.doesNotMatch(titleDetail, /disabled=\{!localCopy\}/);
  assert.doesNotMatch(episodeDetail, /disabled=\{!localCopy\}/);
  assert.match(titleDetail, /<WatchNowControl/);
  assert.match(episodeDetail, /<WatchNowControl/);
  assert.match(playback, /role="group"/);
  assert.match(playback, /aria-label="Choose another playback source"/);
  assert.match(nextConfig, /source: "\/v1\/playback\/:path\*"/);
});

test("source discovery starts on detail open and reuses the query cache", async () => {
  const [playback, titleDetail] = await Promise.all([
    readFile(
      new URL("../app/title/[kind]/[id]/playback.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/title-detail.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(playback, /export function playbackSourcesQuery/);
  assert.match(playback, /queryFn: \(\{ signal \}\) => discoverPlaybackSources/);
  assert.match(playback, /staleTime: 5 \* 60 \* 1000/);
  assert.match(titleDetail, /\.\.\.playbackSourcesQuery\(media, discoveryEpisode\)/);
  assert.match(titleDetail, /enabled: canDiscover/);
  assert.match(titleDetail, /const discoveryEpisode = episodeDialog \?\? nextEpisode/);
  assert.match(titleDetail, /const discovery = sourceDiscoveryQuery\.data/);
});

test("custom player exposes complete playback and track controls", async () => {
  const [player, playback] = await Promise.all([
    readFile(
      new URL("../app/title/[kind]/[id]/video-player.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/playback.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(player, /aria-label="Seek through video"/);
  assert.match(player, /aria-label="Volume"/);
  assert.match(player, /aria-label="Playback settings"/);
  assert.match(player, /hls\.audioTrack = id/);
  assert.match(player, /hls\.subtitleTrack = id/);
  assert.match(player, /hls\.subtitleDisplay = id !== -1/);
  assert.match(player, /descriptorSubtitleTracks\.length/);
  assert.match(player, /onSelectTracks/);
  assert.match(playback, /subtitleStreamIndex/);
  assert.match(player, /requestFullscreen/);
  assert.match(player, /requestPictureInPicture/);
  assert.match(player, /PLAYBACK_RATES/);
});

test("track changes keep the mounted player and swap its descriptor in place", async () => {
  const [playbackRoute, player] = await Promise.all([
    readFile(
      new URL("../app/title/[kind]/[id]/play/playback-route.tsx", import.meta.url),
      "utf8",
    ),
    readFile(
      new URL("../app/title/[kind]/[id]/video-player.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(playbackRoute, /async function selectPlaybackTracks/);
  assert.match(playbackRoute, /return await activatePlayback/);
  const trackActivation = playbackRoute.slice(
    playbackRoute.indexOf("async function selectPlaybackTracks"),
    playbackRoute.indexOf("function closePlayback"),
  );
  assert.match(trackActivation, /media,\s*episode,\s*undefined,/);
  assert.doesNotMatch(trackActivation, /media,\s*episode,\s*resumeSeconds,/);
  assert.doesNotMatch(
    playbackRoute,
    /onSelectTracks=\{\(resumeSeconds, selection\) =>\s*playLocal/,
  );
  assert.match(player, /useState\(playback\.descriptor\)/);
  assert.match(player, /await onSelectTracks/);
  assert.match(player, /setDescriptor\(nextDescriptor\)/);
  assert.match(player, /canvasRef/);
  assert.match(player, /drawImage\(video/);
  assert.match(player, /requestVideoFrameCallback/);
  assert.match(player, /isBuffering \|\| isSwitchingTracks/);
  const trackSwap = player.slice(
    player.indexOf("const switchDescriptorTracks"),
    player.indexOf("const toggleSubtitles"),
  );
  assert.ok(
    trackSwap.indexOf("captureCurrentFrame()") <
      trackSwap.indexOf("await onSelectTracks"),
  );
  assert.doesNotMatch(player, /setHasFrozenFrame/);
});

test("playback links preserve progress and identify the exact episode", () => {
  const episode = {
    id: "tmdb:episode:99",
    tmdbId: 99,
    seasonNumber: 3,
    episodeNumber: 7,
    title: "Episode",
    overview: null,
    airDate: null,
    rating: null,
    still: null,
    runtimeMinutes: null,
    availability: "local",
  };
  const href = playbackHref(
    {
      kind: "series",
      id: "tmdb:series:12",
      tmdbId: 12,
      title: "Series",
      overview: null,
      year: 2026,
      rating: null,
      images: { poster: null, backdrop: null },
      availability: "episodeBased",
      progress: {
        positionSeconds: 20,
        durationSeconds: 100,
        lastSourceId: "local",
        lastSourceLabel: "Jellyfin",
        lastSourceKind: "local",
        episodeId: episode.id,
      },
    },
    episode,
    20,
  );
  const url = new URL(href, "http://reel.local");
  assert.equal(url.pathname, "/title/series/tmdb%3Aseries%3A12/play");
  assert.equal(url.searchParams.get("season"), "3");
  assert.equal(url.searchParams.get("episode"), "7");
  assert.equal(url.searchParams.get("start"), "20");
  assert.equal(url.searchParams.get("progress"), "20");
  assert.equal(url.searchParams.has("mediaUrl"), false);
  assert.equal(url.searchParams.has("sessionId"), false);
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

test("preselected titles cast a continuous poster-centered light field", async () => {
  const [backdrop, styles, card, catalogue, collection] = await Promise.all([
    readFile(new URL("../app/ambient-backdrop.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/globals.css", import.meta.url), "utf8"),
    readFile(new URL("../app/media-card.tsx", import.meta.url), "utf8"),
    readFile(new URL("../app/catalogue-browser.tsx", import.meta.url), "utf8"),
    readFile(
      new URL("../app/collection/collection-browser.tsx", import.meta.url),
      "utf8",
    ),
  ]);

  assert.match(backdrop, /ambient-light-surface/);
  assert.match(backdrop, /ambient-light-core/);
  assert.match(backdrop, /ambient-light-bloom/);
  assert.doesNotMatch(backdrop, /AmbientLavaPool|variant="primary"/);
  assert.match(styles, /ambient-light-surface/);
  assert.match(styles, /inset: -240px/);
  assert.match(styles, /var\(--ambient-light-x\)/);
  assert.match(styles, /blur\(110px\)/);
  assert.doesNotMatch(styles, /ambient-lava-pool|border-radius: 48%/);
  assert.match(card, /getBoundingClientRect\(\)/);
  assert.match(backdrop, /from "motion\/react"/);
  assert.match(backdrop, /<AnimatePresence/);
  assert.match(backdrop, /--ambient-light-x/);
  assert.match(backdrop, /--ambient-light-y/);
  assert.doesNotMatch(backdrop, /style=\{\{ left: layer\.x, top: layer\.y \}\}/);
  assert.match(backdrop, /lastOrigin\.current/);
  assert.match(backdrop, /duration: 0\.28/);
  assert.match(backdrop, /stiffness: 105/);
  assert.match(styles, /saturate\(2\.35\)/);
  assert.doesNotMatch(styles, /ambient-artwork-enter/);
  assert.match(card, /item\.images\.poster \?\? item\.images\.backdrop/);
  assert.match(card, /onMouseEnter/);
  assert.match(card, /onFocus/);
  assert.match(catalogue, /<AmbientBackdrop>/);
  assert.match(collection, /<AmbientBackdrop>/);
});
