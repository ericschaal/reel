import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { JSDOM } from "jsdom";
import ts from "typescript";
import { act, createElement, useState } from "react";

const viewUrl = new URL("../app/title/[kind]/[id]/download-view.tsx", import.meta.url);
const indicatorUrl = new URL("../app/title/[kind]/[id]/request-status-indicator.tsx", import.meta.url);
const fixtureUrl = new URL("./fixtures/request-view-dependencies.mjs", import.meta.url);
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "../../../ui" || specifier === "./playback" || specifier === "../../../media-card") {
      return { url: fixtureUrl.href, shortCircuit: true };
    }
    if (specifier === "../../../requests") {
      return { url: new URL("../app/requests.ts", import.meta.url).href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url.startsWith("file:") && [viewUrl, indicatorUrl].some((file) => fileURLToPath(url) === fileURLToPath(file))) {
      return {
        format: "module",
        shortCircuit: true,
        source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
          compilerOptions: { jsx: ts.JsxEmit.ReactJSX, module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
        }).outputText,
      };
    }
    return nextLoad(url, context);
  },
});

const dom = new JSDOM('<!doctype html><div id="root"></div>', { url: "http://localhost" });
Object.assign(globalThis, {
  window: dom.window,
  document: dom.window.document,
  HTMLElement: dom.window.HTMLElement,
  IS_REACT_ACT_ENVIRONMENT: true,
});
const { createRoot } = await import("react-dom/client");
const { DownloadView } = await import(viewUrl.href);
const { RequestStatusIndicator } = await import(indicatorUrl.href);
const {
  createMediaRequest,
  requestIsBlocked,
  requestLabel,
  requestNeedsAvailabilityRefresh,
  movieAvailabilityQuery,
  requestProfilesQuery,
} = await import("../app/requests.ts");
const { reelProxyPathAllowed, reelRequestCreatePathAllowed } = await import("../app/catalogue.ts");

const movie = { kind: "movie", title: "Movie", tmdbId: 1, images: { poster: null, backdrop: null } };
const series = {
  kind: "series", title: "Series", tmdbId: 2, images: { poster: null, backdrop: null },
  seasons: [
    { id: "one", seasonNumber: 1, title: "Season 1", episodeCount: 8 },
    { id: "two", seasonNumber: 2, title: "Season 2", episodeCount: 10 },
    { id: "three", seasonNumber: 3, title: "Season 3", episodeCount: 12 },
  ],
};

async function mount(props) {
  const root = createRoot(document.getElementById("root"));
  const submitted = [];
  function Harness() {
    const [scope, setScope] = useState(props.scope);
    const [selectedProfileId, setSelectedProfileId] = useState(11);
    return createElement(DownloadView, {
      media: props.media,
      series: props.series ?? null,
      scope,
      status: props.status,
      statusLoading: false,
      statusError: null,
      profiles: { serverId: 0, serverName: "Default", defaultProfileId: 11, profiles: [{ id: 11, name: "HD" }, { id: 12, name: "UHD" }] },
      profilesLoading: false,
      profilesError: null,
      selectedProfileId,
      submitting: false,
      submitError: props.submitError ?? null,
      submitted: props.submitted ?? false,
      onChange: setScope,
      onProfileChange: setSelectedProfileId,
      onSubmit: (value) => submitted.push(value),
      onBack: () => {},
    });
  }
  await act(async () => root.render(createElement(Harness)));
  return { submitted, root };
}

test("movie request submits once and shows a Seerr failure without claiming a download", async () => {
  const status = { kind: "movie", tmdbId: 1, requestStatus: "none", acquisitionStatus: "unknown", seasons: [] };
  const { submitted, root } = await mount({ media: movie, scope: { kind: "movie" }, status, submitError: "Seerr denied the request" });
  assert.equal(document.querySelector("h1")?.textContent, "Movie");
  assert.match(document.querySelector('aside[aria-label="What happens next"]')?.textContent ?? "", /Jellyfin makes it playable/);
  const button = [...document.querySelectorAll("button")].find((item) => item.textContent.includes("Request movie"));
  assert.equal(button.disabled, false);
  const profile = document.querySelector("select");
  assert.equal(profile.value, "11");
  assert.deepEqual([...profile.options].map((option) => option.textContent), ["HD", "UHD"]);
  await act(async () => {
    profile.value = "12";
    profile.dispatchEvent(new dom.window.Event("change", { bubbles: true }));
  });
  assert.equal(profile.value, "12");
  await act(async () => button.click());
  assert.deepEqual(submitted, [{ kind: "movie" }]);
  assert.match(document.body.textContent, /Seerr denied the request/);
  assert.doesNotMatch(document.body.textContent, /Downloaded/);
  await act(async () => root.unmount());
});

test("series request excludes already requested seasons and submits the remaining selection", async () => {
  const status = {
    kind: "series", tmdbId: 2, requestStatus: "approved", acquisitionStatus: "partiallyAvailable",
    seasons: [
      { seasonNumber: 1, requestStatus: "completed", acquisitionStatus: "available" },
      { seasonNumber: 2, requestStatus: "approved", acquisitionStatus: "processing" },
      { seasonNumber: 3, requestStatus: "none", acquisitionStatus: "unknown" },
    ],
  };
  const { submitted, root } = await mount({ media: series, series, scope: { kind: "series", seasonNumbers: [1, 3] }, status });
  assert.equal(document.querySelector("h1")?.textContent, "Series");
  const boxes = [...document.querySelectorAll('input[type="checkbox"]')];
  assert.equal(boxes.length, 3);
  assert.equal(boxes[0].disabled, true);
  assert.equal(boxes[1].disabled, true);
  assert.equal(boxes[2].checked, true);
  const button = [...document.querySelectorAll("button")].find((item) => item.textContent.includes("Request 1 season"));
  await act(async () => button.click());
  assert.deepEqual(submitted, [{ kind: "series", seasonNumbers: [3] }]);
  await act(async () => root.unmount());
});

test("request helpers and proxy allow only the intended actions", async () => {
  assert.equal(reelProxyPathAllowed("v1/requests/movie/1"), true);
  assert.equal(reelProxyPathAllowed("v1/requests/profiles/movie"), true);
  assert.equal(reelProxyPathAllowed("v1/requests/series/2"), true);
  assert.equal(reelProxyPathAllowed("v1/requests/movie/0"), false);
  assert.equal(reelProxyPathAllowed("v1/requests"), false);
  assert.equal(reelProxyPathAllowed("v1/requests/profiles/anything"), false);
  assert.equal(reelRequestCreatePathAllowed("v1/requests"), true);
  assert.equal(reelRequestCreatePathAllowed("v1/requests/movie/1"), false);
  assert.equal(requestIsBlocked({ requestStatus: "failed", acquisitionStatus: "unknown" }), true);
  assert.equal(requestLabel({ requestStatus: "completed", acquisitionStatus: "unknown" }), "Waiting for Jellyfin");
  assert.equal(requestLabel({ requestStatus: "approved", acquisitionStatus: "processing", transferStatus: "waiting" }), "Waiting for a copy");
  assert.equal(requestLabel({ requestStatus: "approved", acquisitionStatus: "processing", transferStatus: "downloading" }), "Downloading");
  assert.equal(requestNeedsAvailabilityRefresh({ requestStatus: "approved", acquisitionStatus: "processing" }), true);
  assert.equal(requestNeedsAvailabilityRefresh({ requestStatus: "failed", acquisitionStatus: "processing" }), false);
  assert.equal(requestLabel({ requestStatus: "failed", acquisitionStatus: "processing" }), "Request failed in Seerr");
  assert.equal(requestLabel({ requestStatus: "declined", acquisitionStatus: "processing" }), "Request declined in Seerr");

  const originalFetch = globalThis.fetch;
  let posted;
  globalThis.fetch = async (url, init) => {
    posted = { url, init };
    return new Response(JSON.stringify({ id: 7, requestStatus: "pending" }), { status: 201 });
  };
  try {
    const created = await createMediaRequest({ kind: "series", tmdbId: 2, seasonNumbers: [3], profileId: 12 });
    assert.deepEqual(created, { id: 7, requestStatus: "pending" });
    assert.equal(posted.url, "/api/reel/v1/requests");
    assert.equal(posted.init.method, "POST");
    assert.deepEqual(JSON.parse(posted.init.body), { kind: "series", tmdbId: 2, seasonNumbers: [3], profileId: 12 });
    await requestProfilesQuery("movie").queryFn({ signal: new AbortController().signal });
    assert.equal(posted.url, "/api/reel/v1/requests/profiles/movie");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("movie availability rechecks Jellyfin through the title endpoint", async () => {
  const originalFetch = globalThis.fetch;
  let requestedUrl;
  globalThis.fetch = async (url) => {
    requestedUrl = url;
    return Response.json({ kind: "movie", availability: "local" });
  };
  try {
    const availability = await movieAvailabilityQuery(44).queryFn({ signal: new AbortController().signal });
    assert.equal(requestedUrl, "/api/reel/v1/titles/movie/44?language=en");
    assert.equal(availability, "local");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("status indicator distinguishes waiting from downloading and offers refresh", async () => {
  const root = createRoot(document.getElementById("root"));
  let refreshed = 0;
  const waiting = { requestStatus: "approved", acquisitionStatus: "processing", transferStatus: "waiting", requestedAt: "2026-09-01T14:00:12.000Z" };
  await act(async () => root.render(createElement(RequestStatusIndicator, { status: waiting, onRefresh: () => refreshed++ })));
  assert.match(document.querySelector('[role="status"]')?.textContent ?? "", /Waiting for a copy/);
  assert.match(document.querySelector('[role="status"]')?.textContent ?? "", /Since Sep 1/);
  await act(async () => document.querySelector("button").click());
  assert.equal(refreshed, 1);
  await act(async () => root.render(createElement(RequestStatusIndicator, { status: { ...waiting, transferStatus: "downloading" } })));
  assert.match(document.querySelector('[role="status"]')?.textContent ?? "", /Downloading/);
  await act(async () => root.unmount());
});
