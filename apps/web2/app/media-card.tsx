"use client";

import Image from "next/image";
import Link from "next/link";
import { useState } from "react";
import { type CatalogueItem, collectionHref, titleHref } from "./catalogue";

// Artwork comes from connected services; keep their URLs intact without routing
// private media hosts through the Next image optimizer.
export function Artwork({
  src,
  sizes,
  priority = false,
  fit = "cover",
}: {
  src: string | null;
  sizes: string;
  priority?: boolean;
  fit?: "cover" | "contain";
}) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);

  return src && !failed ? (
    <>
      <span
        aria-hidden="true"
        className={`absolute inset-0 bg-linear-to-br from-slate-700 to-panel transition-opacity duration-300 ${loaded ? "opacity-0" : "opacity-100"}`}
      />
      <Image
        src={src}
        alt=""
        fill
        sizes={sizes}
        unoptimized
        decoding="async"
        preload={priority}
        loading={priority ? undefined : "lazy"}
        onLoad={() => setLoaded(true)}
        onError={() => setFailed(true)}
        className={`${fit === "contain" ? "object-contain" : "object-cover"} transition-opacity duration-300 ${loaded ? "opacity-100" : "opacity-0"}`}
      />
    </>
  ) : (
    <span
      aria-hidden="true"
      className="absolute inset-0 grid place-items-center bg-linear-to-br from-slate-700 to-panel text-6xl font-bold text-accent/40"
    >
      R
    </span>
  );
}

type RatingSource = "tmdb" | "rottenTomatoes";

export function RatingBadge({
  rating,
  source = "tmdb",
  variant = "inline",
}: {
  rating: number;
  source?: RatingSource;
  variant?: "inline" | "chip";
}) {
  const rottenTomatoes = source === "rottenTomatoes";
  const value = rottenTomatoes
    ? `${Math.round(rating)}%`
    : rating.toFixed(1);
  const chipClass = rottenTomatoes
    ? "border-[#fa320a]/25 bg-[#fa320a]/10"
    : "border-[#01b4e4]/20 bg-[#01b4e4]/8";

  return (
    <span
      className={`inline-flex items-center gap-1.5 font-medium tabular-nums text-ink/90 ${variant === "chip" ? `min-h-6 rounded-md border px-2 py-0.5 ${chipClass}` : ""}`}
      aria-label={`${rottenTomatoes ? "Rotten Tomatoes" : "TMDB"} rating ${value}`}
    >
      {rottenTomatoes ? (
        <svg
          aria-hidden="true"
          className="size-3.5 shrink-0"
          viewBox="0 0 20 20"
        >
          <circle cx="10" cy="11" r="7.5" fill="#FA320A" />
          <path
            d="M8.2 3.8c.8-1.7 2.1-2.2 3.8-1.6-.5.6-.8 1.2-.9 1.9 1-.5 2-.5 3 .1-1.5 1.2-3.4 1.5-5.9-.4Z"
            fill="#72B844"
          />
          <path d="M6 9.5h8M7 13h6" stroke="white" strokeWidth="1.4" />
        </svg>
      ) : (
        <span
          aria-hidden="true"
          className="bg-linear-to-r from-[#90cea1] to-[#01b4e4] bg-clip-text font-mono text-[9px] leading-none font-black tracking-[-0.08em] text-transparent"
        >
          TMDB
        </span>
      )}
      <span>{value}</span>
    </span>
  );
}

export function CatalogueCard({
  item,
  layout = "poster",
  priority = false,
}: {
  item: CatalogueItem;
  layout?: "poster" | "backdrop";
  priority?: boolean;
}) {
  if (item.kind === "category") {
    const isLogo = item.categoryKind !== "genre";
    return (
      <Link
        className="group relative block aspect-[4/3] w-full snap-start overflow-hidden rounded-xl border border-line bg-panel transition-[transform,border-color,box-shadow] duration-300 ease-out hover:z-10 hover:border-accent/50 hover:shadow-[0_18px_44px_#00000066] motion-safe:hover:-translate-y-1 motion-safe:hover:scale-[1.025]"
        href={collectionHref(item.href)}
      >
        {isLogo ? (
          <span
            className="absolute inset-x-[12%] top-[9%] bottom-[27%] opacity-35"
            aria-hidden="true"
          >
            <Artwork
              src={item.images[0] ?? null}
              sizes="(max-width: 640px) 82vw, 420px"
              priority={priority}
              fit="contain"
            />
          </span>
        ) : (
          <span
            className="absolute inset-0 grid grid-cols-3 opacity-65"
            aria-hidden="true"
          >
            {item.images.slice(0, 3).map((image, index) => (
              <span className="relative block min-h-0" key={`${image}-${index}`}>
                <Artwork src={image} sizes="100px" priority={priority} />
              </span>
            ))}
          </span>
        )}
        <span className="absolute inset-0 bg-linear-to-b from-transparent via-background/35 to-background" />
        <span className="absolute inset-x-4 bottom-4 grid gap-1.5 sm:inset-x-5">
          <span className="font-mono text-[10px] tracking-widest text-accent uppercase">
            {item.categoryKind}
          </span>
          <strong className="text-base leading-tight tracking-tight [overflow-wrap:anywhere] sm:text-lg">
            {item.title}
          </strong>
        </span>
      </Link>
    );
  }

  const backdrop = layout === "backdrop";
  return (
    <Link
      className="group grid min-w-0 snap-start content-start gap-3 rounded-xl transition-transform duration-300 ease-out hover:z-10 motion-safe:hover:-translate-y-1 motion-safe:hover:scale-[1.035]"
      href={titleHref(item)}
    >
      <span
        className={`relative block overflow-hidden rounded-xl border border-white/10 bg-panel transition-colors group-hover:border-accent/60 ${backdrop ? "aspect-video" : "aspect-[2/3]"}`}
      >
        <Artwork
          src={backdrop ? item.images.backdrop : item.images.poster}
          sizes={
            backdrop
              ? "(max-width: 640px) 80vw, 420px"
              : "(max-width: 640px) 45vw, 240px"
          }
          priority={priority}
        />
        {item.localCopy ? (
          <span className="absolute top-2.5 left-2.5 rounded-md bg-emerald-200 px-2 py-1 text-[10px] font-bold tracking-wide text-emerald-950 uppercase">
            In library
          </span>
        ) : null}
      </span>
      <span className="grid min-w-0 content-start gap-2">
        <strong
          className={`${backdrop ? "line-clamp-1" : "line-clamp-2"} text-sm leading-5 font-semibold tracking-[-0.01em] group-hover:text-accent`}
        >
          {item.title}
        </strong>
        <span className="flex min-h-6 flex-wrap items-center gap-1.5 text-xs text-muted">
          <span className="inline-flex min-h-6 items-center rounded-md border border-white/8 bg-white/4 px-2 py-0.5 tabular-nums">
            {item.year ?? "Year unavailable"}
          </span>
          {item.rating != null ? (
            <RatingBadge rating={item.rating} variant="chip" />
          ) : null}
        </span>
      </span>
    </Link>
  );
}
