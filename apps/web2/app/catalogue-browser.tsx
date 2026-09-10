"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import {
  type CatalogueManifest,
  type CatalogueRailResponse,
  type CatalogueSection,
  type Surface,
  catalogueRailProxyHref,
  collectionHref,
  mapWithConcurrency,
} from "./catalogue";
import { CatalogueCard } from "./media-card";
import {
  buttonClass,
  CardSkeletons,
  catalogueNavGroupClass,
  catalogueNavItemClass,
  EmptyState,
  Eyebrow,
  Header,
  pageGutter,
} from "./ui";

const surfaces: { id: Surface; label: string }[] = [
  { id: "discover", label: "Discover" },
  { id: "movies", label: "Movies" },
  { id: "series", label: "Series" },
];
const rowClass = `grid grid-flow-col gap-4 overflow-x-auto overscroll-x-contain scroll-px-5 sm:scroll-px-8 lg:scroll-px-12 [scrollbar-width:none] [&::-webkit-scrollbar]:hidden snap-x snap-proximity pt-3 pb-7 sm:gap-5 ${pageGutter}`;

export function CatalogueBrowser({ surface }: { surface: Surface }) {
  const [manifest, setManifest] = useState<CatalogueManifest | null>(null);
  const [rails, setRails] = useState<
    Record<string, { section?: CatalogueSection; error?: string }>
  >({});
  const [error, setError] = useState<string | null>(null);
  const [requestKey, setRequestKey] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    async function load() {
      try {
        setManifest(null);
        setRails({});
        setError(null);
        const response = await fetch(
          `/api/reel/v1/catalogue/${surface}/manifest?language=en`,
          { signal: controller.signal },
        );
        if (!response.ok)
          throw new Error(
            "The catalogue is unavailable right now. Please try again.",
          );
        const data: CatalogueManifest = await response.json();
        if (controller.signal.aborted) return;
        setManifest(data);

        await mapWithConcurrency(data.rails, 3, async (rail) => {
          const url = catalogueRailProxyHref(rail.itemsHref);
          if (!url) {
            setRails((current) => ({
              ...current,
              [rail.id]: { error: "This catalogue rail has an invalid URL." },
            }));
            return;
          }
          try {
            const railResponse = await fetch(url, {
              signal: controller.signal,
            });
            if (!railResponse.ok) throw new Error("This rail is unavailable.");
            const railData: CatalogueRailResponse = await railResponse.json();
            if (!controller.signal.aborted)
              setRails((current) => ({
                ...current,
                [rail.id]: { section: railData.section },
              }));
          } catch (reason) {
            if (!controller.signal.aborted)
              setRails((current) => ({
                ...current,
                [rail.id]: {
                  error:
                    reason instanceof Error
                      ? reason.message
                      : "This rail is unavailable.",
                },
              }));
          }
        });
      } catch (reason) {
        if (!controller.signal.aborted)
          setError(
            reason instanceof Error
              ? reason.message
              : "Unable to load the catalogue.",
          );
      }
    }
    void load();
    return () => controller.abort();
  }, [surface, requestKey]);

  return (
    <div className="min-h-dvh bg-[radial-gradient(ellipse_at_40%_0%,#233336_0%,transparent_45%)]">
      <Header>
        <nav
          className={`flex rounded-full ${catalogueNavGroupClass}`}
          aria-label="Catalogue"
        >
          {surfaces.map((item) => (
            <Link
              key={item.id}
              href={item.id === "discover" ? "/" : `/?surface=${item.id}`}
              aria-current={surface === item.id ? "page" : undefined}
              className={`inline-flex min-h-11 items-center border-r border-white/10 px-3 text-sm last:border-r-0 sm:px-5 ${catalogueNavItemClass} ${surface === item.id ? "bg-accent/12 font-semibold text-accent shadow-[inset_0_0_20px_#f4bc5212]" : "text-muted hover:text-ink"}`}
            >
              {item.label}
            </Link>
          ))}
        </nav>
      </Header>
      <main id="main-content" className="mx-auto max-w-[1600px] pb-16 sm:pb-24">
        <section className={`pt-12 pb-10 sm:pt-16 sm:pb-14 ${pageGutter}`}>
          <Eyebrow>Your unified library</Eyebrow>
          <h1 className="max-w-4xl text-4xl leading-[1.08] font-semibold tracking-[-0.045em] text-balance sm:text-6xl lg:text-7xl">
            {surface === "discover"
              ? "Find something remarkable."
              : surface === "movies"
                ? "Movies"
                : "Series"}
          </h1>
        </section>
        {manifest ? (
          <div className="grid gap-10 sm:gap-14">
            {manifest.rails.map((rail, sectionIndex) => {
              const state = rails[rail.id];
              const section = state?.section;
              return (
                <section
                  className="min-w-0"
                  key={rail.id}
                  aria-labelledby={`section-${rail.id}`}
                  aria-busy={!section && !state?.error}
                >
                  <div
                    className={`mb-3 flex items-center justify-between gap-4 ${pageGutter}`}
                  >
                    <h2
                      id={`section-${rail.id}`}
                      className="text-xl font-semibold tracking-tight sm:text-2xl"
                    >
                      {rail.title}
                    </h2>
                    {section?.href ? (
                      <Link
                        className="inline-flex min-h-11 shrink-0 items-center gap-2 text-sm text-muted hover:text-accent"
                        href={collectionHref(section.href)}
                        aria-label={`View all ${section.title}`}
                      >
                        View all <span aria-hidden="true">→</span>
                      </Link>
                    ) : null}
                  </div>
                  <div
                    className={
                      rail.layout === "backdrop"
                        ? `${rowClass} auto-cols-[82%] sm:auto-cols-[340px] lg:auto-cols-[420px]`
                        : `${rowClass} auto-cols-[44%] sm:auto-cols-[180px] lg:auto-cols-[210px]`
                    }
                  >
                    {section ? (
                      section.items.map((item, itemIndex) => (
                        <CatalogueCard
                          key={`${item.kind}-${item.id}`}
                          item={item}
                          layout={section.layout}
                          priority={sectionIndex === 0 && itemIndex < 2}
                        />
                      ))
                    ) : state?.error ? (
                      <div className="col-span-2 grid min-h-40 content-center gap-3 rounded-xl border border-line bg-panel/60 p-5 text-sm text-muted">
                        <p>{state.error}</p>
                        <button
                          className={`${buttonClass} w-fit`}
                          type="button"
                          onClick={() => setRequestKey((key) => key + 1)}
                        >
                          Try again
                        </button>
                      </div>
                    ) : (
                      <CardSkeletons
                        count={Math.min(rail.itemCountHint, 6)}
                        layout={rail.layout}
                      />
                    )}
                  </div>
                </section>
              );
            })}
            {manifest.rails.length === 0 ? (
              <EmptyState title="Nothing to show yet">
                Check back soon for movies and series.
              </EmptyState>
            ) : null}
          </div>
        ) : error ? (
          <EmptyState
            title="Catalogue unavailable"
            action={
              <button
                className={buttonClass}
                type="button"
                onClick={() => {
                  setError(null);
                  setRequestKey((key) => key + 1);
                }}
              >
                Try again
              </button>
            }
          >
            {error}
          </EmptyState>
        ) : (
          <div
            className="grid gap-12"
            role="status"
            aria-label="Loading catalogue"
            aria-busy="true"
          >
            {[0, 1].map((row) => (
              <div key={row}>
                <div className={`mb-5 ${pageGutter}`}>
                  <div className="h-6 w-44 rounded bg-white/5 motion-safe:animate-pulse" />
                </div>
                <div
                  className={`${rowClass} auto-cols-[44%] sm:auto-cols-[180px] lg:auto-cols-[210px]`}
                >
                  <CardSkeletons />
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
