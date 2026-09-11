import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { registerHooks } from "node:module";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { JSDOM } from "jsdom";
import ts from "typescript";
import { act, createElement } from "react";

const routeUrl = new URL(
  "../app/title/[kind]/[id]/play/playback-route.tsx",
  import.meta.url,
);
const dependenciesUrl = new URL(
  "./fixtures/playback-route-dependencies.mjs",
  import.meta.url,
);
registerHooks({
  resolve(specifier, context, nextResolve) {
    if (specifier === "next/navigation" || specifier === "../playback") {
      return { url: dependenciesUrl.href, shortCircuit: true };
    }
    return nextResolve(specifier, context);
  },
  load(url, context, nextLoad) {
    if (url.startsWith("file:") && fileURLToPath(url) === fileURLToPath(routeUrl)) {
      return {
        format: "module",
        shortCircuit: true,
        source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
          compilerOptions: {
            jsx: ts.JsxEmit.ReactJSX,
            module: ts.ModuleKind.ESNext,
            target: ts.ScriptTarget.ES2022,
          },
        }).outputText,
      };
    }
    return nextLoad(url, context);
  },
});

const dom = new JSDOM('<!doctype html><div id="root"></div>');
Object.assign(globalThis, {
  window: dom.window,
  document: dom.window.document,
  IS_REACT_ACT_ENVIRONMENT: true,
  __activationCalls: 0,
});
const { createRoot } = await import("react-dom/client");
const { PlaybackRoute } = await import(routeUrl.href);
const media = {
  kind: "movie",
  id: "tmdb:movie:1",
  tmdbId: 1,
  title: "Movie",
  overview: null,
  year: 2026,
  rating: null,
  images: { poster: null, backdrop: null },
  availability: "local",
};

async function mountPlayer(container, sourceSelection = { kind: "auto" }) {
  const root = createRoot(container);
  await act(async () =>
    root.render(
      createElement(PlaybackRoute, {
        media,
        resumeSeconds: 12,
        sourceSelection,
        backHref: "/title/movie/tmdb%3Amovie%3A1",
      }),
    ),
  );
  assert.equal(container.firstElementChild?.dataset.status, "ready");
  return root;
}

test("a fresh player-route mount reactivates playback after reload", async () => {
  const container = document.getElementById("root");
  let root = await mountPlayer(container);
  assert.equal(globalThis.__activationCalls, 1);

  await act(async () => root.unmount());
  root = await mountPlayer(container);

  assert.equal(globalThis.__activationCalls, 2);
  await act(async () => root.unmount());
});

test("the URL-backed player route activates an explicit opaque source", async () => {
  const container = document.getElementById("root");
  const sourceSelection = {
    kind: "aioStreams",
    discoveryId: "a".repeat(32),
    candidateId: "b".repeat(32),
  };
  const root = await mountPlayer(container, sourceSelection);

  assert.deepEqual(globalThis.__lastSourceSelection, sourceSelection);
  await act(async () => root.unmount());
});
