"use client";

import { useState } from "react";
import type { MediaCard } from "../../../catalogue";
import { Dialog } from "../../../dialog";
import { Artwork } from "../../../media-card";
import {
  buttonClass,
  primaryButtonClass,
  Eyebrow,
  glassClass,
  Header,
  pageGutter,
} from "../../../ui";

type Source = {
  id: string;
  provider: string;
  quality: string;
  detail: string;
  available: boolean;
};
const exampleSources: Source[] = [
  {
    id: "stremio-1",
    provider: "Stremio · 1",
    quality: "4K · HDR · 5.1",
    detail: "12.4 GB",
    available: true,
  },
  {
    id: "stremio-2",
    provider: "Stremio · 2",
    quality: "1080p · H.264 · 5.1",
    detail: "3.8 GB",
    available: true,
  },
  {
    id: "stremio-3",
    provider: "Stremio · 3",
    quality: "720p · H.264 · Stereo",
    detail: "1.2 GB",
    available: true,
  },
];

export function TitleDetail({ media }: { media: MediaCard }) {
  const sources: Source[] = [
    {
      id: "local",
      provider: "Jellyfin",
      quality: "Local library",
      detail: media.localCopy ? "In library" : "Unavailable",
      available: Boolean(media.localCopy),
    },
    ...exampleSources,
  ];
  const [selectedSource, setSelectedSource] = useState(
    media.localCopy ? "local" : "stremio-1",
  );
  const [dialog, setDialog] = useState<"sources" | "player" | "request" | null>(
    null,
  );
  const selected =
    sources.find((source) => source.id === selectedSource) ?? sources[1];

  return (
    <div className="relative isolate min-h-dvh">
      <div
        className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[75vh] overflow-hidden opacity-35"
        aria-hidden="true"
      >
        <Artwork src={media.images.backdrop} sizes="100vw" priority />
        <div className="absolute inset-0 bg-linear-to-r from-background via-background/50 to-transparent" />
        <div className="absolute inset-0 bg-linear-to-t from-background via-background/30 to-transparent" />
      </div>
      <Header preview />
      <main
        id="main-content"
        className={`mx-auto max-w-[1400px] pt-10 pb-16 sm:pt-16 lg:pt-20 ${pageGutter}`}
      >
        <section className="grid items-start gap-8 md:grid-cols-[220px_minmax(0,1fr)] md:gap-10 lg:grid-cols-[280px_minmax(0,1fr)] lg:gap-16">
          <div className="relative aspect-[2/3] w-32 overflow-hidden rounded-xl border border-line bg-panel shadow-2xl sm:w-40 md:w-full">
            <Artwork
              src={media.images.poster}
              sizes="(max-width: 768px) 160px, 280px"
              priority
            />
          </div>
          <div className="min-w-0 md:py-3">
            <Eyebrow>{media.kind === "movie" ? "Movie" : "Series"}</Eyebrow>
            <h1 className="text-4xl leading-[1.08] font-semibold tracking-[-0.045em] text-balance [overflow-wrap:anywhere] sm:text-5xl lg:text-6xl">
              {media.title}
            </h1>
            <p className="mt-5 text-sm leading-6 text-muted">
              {media.year ?? "Year unavailable"}
              {media.rating != null ? ` · ★ ${media.rating.toFixed(1)}` : ""}
              {media.localCopy ? " · In your library" : ""}
            </p>
            <p className="mt-6 max-w-2xl text-base leading-7 text-ink/80">
              {media.overview || "No synopsis is available for this title yet."}
            </p>
            <div className="mt-8 flex flex-wrap gap-3">
              <button
                className={`${primaryButtonClass} w-full sm:w-auto`}
                type="button"
                onClick={() => setDialog("player")}
              >
                <span aria-hidden="true">▶</span> Preview player
              </button>
              <button
                className={buttonClass}
                type="button"
                onClick={() => setDialog("sources")}
              >
                Other sources
              </button>
              <button
                className={buttonClass}
                type="button"
                onClick={() => setDialog("request")}
              >
                Request download
              </button>
            </div>
            <p className="mt-4 text-xs leading-5 text-muted">
              Preview only. Playback and download requests are not available
              yet.
            </p>
          </div>
        </section>
        <section
          className={`mt-10 flex flex-wrap items-center justify-between gap-5 rounded-2xl p-5 sm:mt-14 sm:p-6 ${glassClass}`}
          aria-label="Preview playback source"
        >
          <div className="grid gap-2">
            <span className="text-xs text-muted">Preview source</span>
            <strong className="text-sm font-semibold">
              {selected.provider}{" "}
              <span className="font-normal text-muted">
                · {selected.quality}
              </span>
            </strong>
          </div>
          <button
            className="inline-flex min-h-11 items-center text-sm font-semibold text-accent hover:text-amber-200"
            type="button"
            onClick={() => setDialog("sources")}
          >
            Change source{" "}
            <span className="ml-2" aria-hidden="true">
              →
            </span>
          </button>
        </section>
      </main>
      {dialog ? (
        <Dialog labelledBy="dialog-title" onClose={() => setDialog(null)}>
          <div className="flex items-start justify-between gap-4">
            <div>
              <Eyebrow>
                {dialog === "sources" ? "Playback options" : "UI preview"}
              </Eyebrow>
              <h2
                id="dialog-title"
                className="text-2xl leading-tight font-semibold tracking-tight sm:text-3xl"
              >
                {dialog === "sources"
                  ? "Choose a source"
                  : dialog === "player"
                    ? "Player preview"
                    : "Download requests"}
              </h2>
            </div>
            <button
              type="button"
              className="inline-flex size-11 shrink-0 items-center justify-center rounded-full border border-line text-2xl text-muted hover:bg-white/5 hover:text-ink"
              aria-label="Close dialog"
              onClick={() => setDialog(null)}
            >
              ×
            </button>
          </div>
          {dialog === "sources" ? (
            <>
              <p className="mt-4 text-sm leading-6 text-muted">
                These are example sources, shown in add-on order. Selecting one
                updates this preview only.
              </p>
              <fieldset className="mt-6 grid gap-3">
                <legend className="sr-only">Playback source</legend>
                {sources.map((source) => (
                  <label
                    key={source.id}
                    className={`flex min-h-20 items-center gap-3 rounded-xl border p-4 ${!source.available ? "cursor-not-allowed border-line opacity-50" : selectedSource === source.id ? "cursor-pointer border-accent bg-accent/5" : "cursor-pointer border-line bg-white/2 hover:bg-white/5"}`}
                  >
                    <input
                      type="radio"
                      name="source"
                      value={source.id}
                      checked={selectedSource === source.id}
                      disabled={!source.available}
                      onChange={() => setSelectedSource(source.id)}
                      className="size-4 shrink-0 accent-accent"
                    />
                    <span className="grid min-w-0 flex-1 gap-1">
                      <strong className="text-sm font-semibold">
                        {source.provider}
                      </strong>
                      <span className="text-xs leading-5 text-muted">
                        {source.quality}
                      </span>
                    </span>
                    <span className="hidden text-xs text-muted sm:block">
                      {source.detail}
                    </span>
                  </label>
                ))}
              </fieldset>
              <button
                className={`${primaryButtonClass} mt-6 w-full`}
                type="button"
                onClick={() => setDialog("player")}
              >
                Preview with {selected.provider}
              </button>
            </>
          ) : (
            <>
              <p className="mt-6 text-lg font-semibold [overflow-wrap:anywhere]">
                {media.title}
              </p>
              <p className="mt-3 text-sm leading-6 text-muted">
                {dialog === "player"
                  ? `${selected.provider} · ${selected.quality}. Playback is not connected yet.`
                  : "Download requests are not connected yet. No request has been submitted."}
              </p>
              <button
                className={`${buttonClass} mt-6`}
                type="button"
                onClick={() => setDialog(null)}
              >
                Back to title
              </button>
            </>
          )}
        </Dialog>
      ) : null}
    </div>
  );
}
