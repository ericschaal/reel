"use client";

import Image from "next/image";
import Link from "next/link";
import { useRef, useState } from "react";
import { useAmbientBackdrop } from "./ambient-backdrop";
import {
  type CatalogueItem,
  type MediaCard,
  collectionHref,
  titleHref,
} from "./catalogue";

const mediaOverlayPillClass =
  "inline-flex min-h-7 items-center rounded-full border border-white/12 bg-black/55 px-2.5 py-1 text-xs leading-none font-semibold shadow-[0_2px_8px_#0005] backdrop-blur-md";

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
  return src ? (
    <ArtworkImage key={src} src={src} sizes={sizes} priority={priority} fit={fit} />
  ) : (
    <ArtworkFallback />
  );
}

function ArtworkImage({
  src,
  sizes,
  priority,
  fit,
}: {
  src: string;
  sizes: string;
  priority: boolean;
  fit: "cover" | "contain";
}) {
  const [loaded, setLoaded] = useState(false);
  const [failed, setFailed] = useState(false);

  return !failed ? (
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
  ) : <ArtworkFallback />;
}

function ArtworkFallback() {
  return (
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
  variant?: "inline" | "chip" | "overlay";
}) {
  const rottenTomatoes = source === "rottenTomatoes";
  const value = rottenTomatoes
    ? `${Math.round(rating)}%`
    : rating.toFixed(1);
  const chipClass = rottenTomatoes
    ? "border-[#fa320a]/25 bg-[#fa320a]/10"
    : "border-[#01b4e4]/20 bg-[#01b4e4]/8";
  const badgeClass =
    variant === "overlay"
      ? "min-h-6 shrink-0 rounded-full border border-white/15 bg-black/65 px-1.5 py-0.5 text-white shadow-sm backdrop-blur-md"
      : variant === "chip"
        ? `min-h-6 rounded-full border px-2 py-0.5 ${chipClass}`
        : "";

  return (
    <span
      className={`inline-flex items-center gap-1.5 leading-none font-medium tabular-nums text-ink/90 ${badgeClass}`}
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
          className="relative top-px bg-linear-to-r from-[#90cea1] to-[#01b4e4] bg-clip-text font-mono text-[9px] leading-none font-black tracking-[-0.08em] text-transparent"
        >
          TMDB
        </span>
      )}
      <span className="leading-none">{value}</span>
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
  const ambientBackdrop = useAmbientBackdrop();
  const interactions = useRef({ focused: false, hovered: false });

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
  const ambientArtwork = item.images.poster ?? item.images.backdrop;
  const showAmbientArtwork = () => {
    if (ambientArtwork) ambientBackdrop.show(ambientArtwork);
  };
  const clearAmbientArtwork = () => {
    if (ambientArtwork) ambientBackdrop.clear(ambientArtwork);
  };

  return (
    <Link
      className="group grid min-w-0 snap-start content-start gap-3 rounded-xl transition-transform duration-300 ease-out hover:z-10 motion-safe:hover:-translate-y-1 motion-safe:hover:scale-[1.035]"
      href={titleHref(item)}
      onMouseEnter={() => {
        interactions.current.hovered = true;
        showAmbientArtwork();
      }}
      onMouseLeave={() => {
        interactions.current.hovered = false;
        if (!interactions.current.focused) clearAmbientArtwork();
      }}
      onFocus={() => {
        interactions.current.focused = true;
        showAmbientArtwork();
      }}
      onBlur={() => {
        interactions.current.focused = false;
        if (!interactions.current.hovered) clearAmbientArtwork();
      }}
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
        <span className="absolute top-2.5 left-2.5 text-white/90">
          <MediaSummaryPill item={item} />
        </span>
        {item.rating != null ? (
          <span className="absolute inset-x-0 bottom-0 flex bg-linear-to-t from-black/85 via-black/40 to-transparent px-2 pt-10 pb-2 text-[11px] text-white">
            <RatingBadge rating={item.rating} variant="overlay" />
          </span>
        ) : null}
      </span>
      <span className="grid min-w-0 content-start">
        <strong
          className={`${backdrop ? "line-clamp-1" : "line-clamp-2"} text-sm leading-5 font-semibold tracking-[-0.01em] group-hover:text-accent`}
        >
          {item.title}
        </strong>
      </span>
    </Link>
  );
}

function MediaSummaryPill({
  item,
}: {
  item: MediaCard;
}) {
  const label =
    item.kind === "movie"
      ? formatRuntime(item.runtimeMinutes)
      : formatSeasons(item.numberOfSeasons);
  return (
    <span
      className={`min-w-0 gap-1.5 whitespace-nowrap tabular-nums ${mediaOverlayPillClass}`}
    >
      <span>{item.year ?? "Year unavailable"}</span>
      {label ? (
        <>
          <span className="h-3 w-px bg-white/20" aria-hidden="true" />
          {item.kind === "movie" ? <ClockIcon /> : <SeasonsIcon />}
          <span>{label}</span>
        </>
      ) : null}
      {item.availability === "local" ? (
        <>
          <span className="h-3 w-px bg-white/20" aria-hidden="true" />
          <span
            className="inline-flex text-accent"
            role="img"
            aria-label="In library"
          >
            <DownloadIcon />
          </span>
        </>
      ) : null}
    </span>
  );
}

function formatRuntime(minutes: number | null | undefined) {
  if (!minutes || minutes <= 0) return null;
  const hours = Math.floor(minutes / 60);
  const remainder = minutes % 60;
  return hours ? `${hours}h ${remainder ? `${remainder}m` : ""}`.trim() : `${minutes}m`;
}

function formatSeasons(count: number | null | undefined) {
  if (!count || count <= 0) return null;
  return `${count} season${count === 1 ? "" : "s"}`;
}

function ClockIcon() {
  return <svg aria-hidden="true" className="size-3.5 fill-none stroke-current" viewBox="0 0 16 16" strokeWidth="1.6"><circle cx="8" cy="8" r="5.5" /><path d="M8 4.5V8l2.4 1.4" /></svg>;
}

function SeasonsIcon() {
  return <svg aria-hidden="true" className="size-3.5 fill-none stroke-current" viewBox="0 0 16 16" strokeWidth="1.6"><rect x="3" y="3" width="9" height="9" rx="1.5" /><path d="M5 1.5h7.5a2 2 0 0 1 2 2V11" /></svg>;
}

function DownloadIcon() {
  return (
    <svg
      aria-hidden="true"
      className="size-4 fill-none stroke-current"
      viewBox="0 0 16 16"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M8 2v7m-2.75-2.5L8 9.25l2.75-2.75M3 12.5h10" />
    </svg>
  );
}
