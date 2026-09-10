"use client";

import { useInfiniteQuery } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef } from "react";
import {
  type CatalogueItem,
  collectionProxyHref,
} from "../catalogue";
import { CatalogueCard } from "../media-card";
import { collectionQuery } from "../reel-query";
import {
  buttonClass,
  CardSkeletons,
  EmptyState,
  Eyebrow,
  Header,
  pageGutter,
} from "../ui";

export function CollectionBrowser({ initialHref }: { initialHref: string }) {
  const valid = Boolean(collectionProxyHref(initialHref));
  const sentinelRef = useRef<HTMLDivElement>(null);
  const {
    data,
    isPending,
    isFetchingNextPage,
    isError,
    hasNextPage,
    fetchNextPage,
    refetch,
  } = useInfiniteQuery({
    ...collectionQuery(initialHref),
    enabled: valid,
  });
  const pages = useMemo(() => data?.pages ?? [], [data?.pages]);
  const title = pages[0]?.title ?? "Collection";
  const total = pages[0]?.totalResults ?? 0;
  const next = pages.at(-1)?.next ?? null;
  const loading = isPending || isFetchingNextPage;
  const error = !valid
    ? "This collection link is invalid."
    : isError
      ? "This collection could not be loaded. Please try again."
      : null;
  const items = useMemo(() => {
    const unique = new Map<string, CatalogueItem>();
    for (const page of pages) {
      for (const item of page.items)
        unique.set(`${item.kind}-${item.id}`, item);
    }
    return [...unique.values()];
  }, [pages]);

  const loadMore = useCallback(() => {
    if (hasNextPage && !isFetchingNextPage) void fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !next || loading || error) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) loadMore();
      },
      { rootMargin: "300px" },
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [loadMore, next, loading, error]);

  return (
    <div className="min-h-dvh bg-[radial-gradient(ellipse_at_15%_0%,#233336_0%,transparent_40%)]">
      <Header />
      <main
        id="main-content"
        className={`mx-auto max-w-[1600px] pb-16 [overflow-anchor:none] ${pageGutter}`}
      >
        <section className="pt-12 pb-10 sm:pt-16 sm:pb-12">
          <Eyebrow>Browse collection</Eyebrow>
          <h1 className="max-w-4xl text-4xl leading-tight font-semibold tracking-[-0.045em] [overflow-wrap:anywhere] sm:text-6xl">
            {title}
          </h1>
          {total > 0 ? (
            <p className="mt-4 text-sm text-muted">
              {total.toLocaleString()} titles
            </p>
          ) : null}
        </section>
        <div
          className="grid grid-cols-2 gap-x-4 gap-y-7 sm:grid-cols-3 sm:gap-x-5 lg:grid-cols-5 xl:grid-cols-6"
          aria-label="Collection titles"
          aria-busy={loading}
        >
          {items.map((item, index) => (
            <CatalogueCard
              key={`${item.kind}-${item.id}`}
              item={item}
              priority={index < 4}
            />
          ))}
          {loading && items.length === 0 ? <CardSkeletons count={12} /> : null}
        </div>
        <div ref={sentinelRef} aria-hidden="true" className="h-px" />
        {error ? (
          <EmptyState
            title="Unable to load collection"
            action={
              valid ? (
                <button
                  className={buttonClass}
                  type="button"
                  onClick={() =>
                    void (items.length ? fetchNextPage() : refetch())
                  }
                >
                  Try again
                </button>
              ) : null
            }
          >
            {error}
          </EmptyState>
        ) : null}
        <div
          className="mt-10 flex min-h-12 justify-center"
          role="status"
          aria-live="polite"
        >
          {loading ? (
            <p className="self-center text-sm text-muted">
              {items.length ? "Loading more titles…" : "Loading collection…"}
            </p>
          ) : !error && next ? (
            <button
              className={buttonClass}
              type="button"
              onClick={loadMore}
            >
              Load more
            </button>
          ) : !error && items.length > 0 ? (
            <p className="text-sm text-muted">
              You’ve reached the end · {items.length.toLocaleString()} titles
            </p>
          ) : null}
        </div>
        {!loading && !error && items.length === 0 ? (
          <EmptyState title="No titles found">
            Try another collection to find something to watch.
          </EmptyState>
        ) : null}
      </main>
    </div>
  );
}
