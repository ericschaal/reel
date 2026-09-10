import Image from "next/image";
import Link from "next/link";
import { type CatalogueItem, collectionHref, titleHref } from "./catalogue";

// Artwork comes from connected services; keep their URLs intact without routing
// private media hosts through the Next image optimizer.
export function Artwork({
  src,
  sizes,
  priority = false,
}: {
  src: string | null;
  sizes: string;
  priority?: boolean;
}) {
  return src ? (
    <Image
      src={src}
      alt=""
      fill
      sizes={sizes}
      unoptimized
      loading={priority ? "eager" : "lazy"}
      className="object-cover"
    />
  ) : (
    <span
      aria-hidden="true"
      className="absolute inset-0 grid place-items-center bg-linear-to-br from-slate-700 to-panel text-6xl font-bold text-accent/40"
    >
      R
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
    return (
      <Link
        className="group relative aspect-[2/3] snap-start overflow-hidden rounded-xl border border-line bg-panel transition-colors hover:border-accent/60"
        href={collectionHref(item.href)}
      >
        <span
          className="absolute inset-0 grid grid-cols-3 opacity-60"
          aria-hidden="true"
        >
          {item.images.slice(0, 3).map((image, index) => (
            <span className="relative" key={`${image}-${index}`}>
              <Artwork src={image} sizes="100px" priority={priority} />
            </span>
          ))}
        </span>
        <span className="absolute inset-0 bg-linear-to-b from-transparent via-background/30 to-background" />
        <span className="absolute inset-x-4 bottom-5 grid gap-2 sm:inset-x-5">
          <span className="font-mono text-[10px] tracking-widest text-accent uppercase">
            {item.categoryKind}
          </span>
          <strong className="text-xl leading-tight tracking-tight [overflow-wrap:anywhere] sm:text-2xl">
            {item.title}
          </strong>
          <span className="text-xs leading-5 text-muted group-hover:text-ink">
            Explore {item.mediaKind === "movie" ? "movies" : "series"} →
          </span>
        </span>
      </Link>
    );
  }

  const backdrop = layout === "backdrop";
  return (
    <Link
      className="group grid min-w-0 snap-start content-start gap-3 rounded-xl"
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
      <span className="grid min-w-0 gap-1">
        <strong className="line-clamp-2 text-sm leading-5 font-semibold group-hover:text-accent">
          {item.title}
        </strong>
        <span className="text-xs leading-5 text-muted">
          {item.year ?? "Year unavailable"}
          {item.rating != null ? ` · ★ ${item.rating.toFixed(1)}` : ""}
        </span>
      </span>
    </Link>
  );
}
