"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import {
  type CatalogueItem,
  type CollectionResponse,
  collectionProxyHref,
} from "../catalogue";
import { CatalogueCard } from "../media-card";
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
  const [title, setTitle] = useState("Collection");
  const [items, setItems] = useState<CatalogueItem[]>([]);
  const [next, setNext] = useState<string | null>(null);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(valid);
  const [error, setError] = useState<string | null>(
    valid ? null : "This collection link is invalid.",
  );
  const [request, setRequest] = useState({ href: initialHref, attempt: 0 });
  const sentinelRef = useRef<HTMLDivElement>(null);
  const inFlight = useRef(valid);

  useEffect(() => {
    const url = collectionProxyHref(request.href);
    if (!url) return;
    const controller = new AbortController();
    async function load() {
      try {
        const response = await fetch(url!, { signal: controller.signal });
        if (!response.ok)
          throw new Error(
            "This collection could not be loaded. Please try again.",
          );
        const page: CollectionResponse = await response.json();
        if (controller.signal.aborted) return;
        setTitle(page.title);
        setTotal(page.totalResults);
        setNext(page.next);
        setItems((current) => {
          const unique = new Map(
            current.map((item) => [`${item.kind}-${item.id}`, item]),
          );
          for (const item of page.items)
            unique.set(`${item.kind}-${item.id}`, item);
          return [...unique.values()];
        });
      } catch (reason) {
        if (!controller.signal.aborted)
          setError(
            reason instanceof Error
              ? reason.message
              : "Unable to load this collection.",
          );
      } finally {
        if (!controller.signal.aborted) {
          inFlight.current = false;
          setLoading(false);
        }
      }
    }
    void load();
    return () => controller.abort();
  }, [request]);

  const loadMore = useCallback((href: string) => {
    if (inFlight.current) return;
    if (!collectionProxyHref(href)) {
      setError(
        "The next page link is invalid. Return to the catalogue to browse another collection.",
      );
      return;
    }
    inFlight.current = true;
    setLoading(true);
    setError(null);
    setRequest((current) => ({ href, attempt: current.attempt + 1 }));
  }, []);

  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel || !next || loading || error) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) loadMore(next);
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
                  onClick={() => loadMore(next ?? request.href)}
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
              onClick={() => loadMore(next)}
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
