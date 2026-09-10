"use client";

import Link from "next/link";
import { useQueries, useQuery } from "@tanstack/react-query";
import {
  type Surface,
  collectionHref,
} from "./catalogue";
import { CatalogueCard } from "./media-card";
import { catalogueManifestQuery, catalogueRailQuery } from "./reel-query";
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
  const manifestQuery = useQuery(catalogueManifestQuery(surface));
  const manifest = manifestQuery.data;
  const railQueries = useQueries({
    queries: (manifest?.rails ?? []).map((rail) =>
      catalogueRailQuery(rail.itemsHref),
    ),
  });

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
              const state = railQueries[sectionIndex];
              const section = state.data?.section;
              return (
                <section
                  className="min-w-0"
                  key={rail.id}
                  aria-labelledby={`section-${rail.id}`}
                  aria-busy={!section && state.isPending}
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
                    ) : state.isError ? (
                      <div className="col-span-2 grid min-h-40 content-center gap-3 rounded-xl border border-line bg-panel/60 p-5 text-sm text-muted">
                        <p>This rail is unavailable.</p>
                        <button
                          className={`${buttonClass} w-fit`}
                          type="button"
                          onClick={() => void state.refetch()}
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
        ) : manifestQuery.isError ? (
          <EmptyState
            title="Catalogue unavailable"
            action={
              <button
                className={buttonClass}
                type="button"
                onClick={() => void manifestQuery.refetch()}
              >
                Try again
              </button>
            }
          >
            The catalogue is unavailable right now. Please try again.
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
