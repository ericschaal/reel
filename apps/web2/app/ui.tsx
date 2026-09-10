import Link from "next/link";
import type { ReactNode } from "react";

const buttonBase =
  "inline-flex min-h-11 items-center justify-center gap-2 rounded-full border px-5 py-2.5 text-sm font-semibold transition-colors disabled:cursor-not-allowed disabled:opacity-50";
export const buttonClass = `${buttonBase} border-line bg-white/5 hover:border-white/40 hover:bg-white/10`;
export const primaryButtonClass = `${buttonBase} border-accent bg-accent text-background hover:border-amber-300 hover:bg-amber-300`;
// Shared glass treatment: dark fallback, translucent gradient, frosted backdrop,
// and inset highlights. Text sits above the effect and stays crisp.
export const glassClass =
  "border border-white/20 bg-panel/90 bg-linear-to-br from-white/12 via-white/3 to-white/6 shadow-[0_8px_32px_#00000030,inset_0_1px_0_#ffffff30,inset_0_-1px_0_#ffffff08] supports-backdrop-filter:bg-panel/60 backdrop-blur-2xl backdrop-saturate-150";
export const liquidGlassGroupClass =
  "overflow-hidden border border-white/50 bg-white/2.5 backdrop-blur-sm";
export const liquidGlassItemClass =
  "relative isolate bg-white/2.5 backdrop-blur-sm shadow-[inset_0_1px_0_rgba(255,255,255,0.75),0_0_9px_rgba(0,0,0,0.2),0_3px_8px_rgba(0,0,0,0.15)] transition-all duration-300 before:pointer-events-none before:absolute before:inset-0 before:-z-10 before:bg-linear-to-br before:from-white/60 before:via-transparent before:to-transparent before:opacity-70 after:pointer-events-none after:absolute after:inset-0 after:-z-10 after:bg-linear-to-tl after:from-white/30 after:via-transparent after:to-transparent after:opacity-50";
export const catalogueNavGroupClass =
  "overflow-hidden border border-white/12 bg-[#081012]/70 shadow-[0_12px_34px_#0000004d,inset_0_1px_0_#ffffff1f] supports-backdrop-filter:bg-[#0a1718]/50 backdrop-blur-xl backdrop-saturate-150";
export const catalogueNavItemClass =
  "relative transition-[color,background-color,box-shadow] duration-200 hover:bg-white/7";
export const pageGutter = "px-5 sm:px-8 lg:px-12";

export function Header({
  children,
  preview = false,
}: {
  children?: ReactNode;
  preview?: boolean;
}) {
  return (
    <header
      className={`relative z-10 mx-auto flex min-h-20 max-w-[1600px] flex-wrap items-center justify-between gap-x-4 gap-y-3 py-4 ${pageGutter}`}
    >
      <Link
        className="inline-flex min-h-11 items-center text-lg font-extrabold tracking-[0.28em] text-accent"
        href="/"
        aria-label="Reel home"
      >
        REEL
      </Link>
      {children ?? (
        <Link
          className="inline-flex min-h-11 items-center gap-2 text-sm text-muted hover:text-ink"
          href="/"
        >
          ← Catalogue
        </Link>
      )}
      {preview ? (
        <span className="rounded-full border border-line px-3 py-1.5 text-xs text-muted">
          UI preview
        </span>
      ) : null}
    </header>
  );
}

export function Eyebrow({ children }: { children: ReactNode }) {
  return (
    <p className="mb-4 font-mono text-xs font-semibold tracking-[0.16em] text-accent uppercase">
      {children}
    </p>
  );
}

export function EmptyState({
  title,
  children,
  action,
}: {
  title: string;
  children: ReactNode;
  action?: ReactNode;
}) {
  return (
    <section
      className="mx-auto my-8 w-[calc(100%-2.5rem)] max-w-lg rounded-2xl border border-line bg-panel/80 px-6 py-10 text-center"
      role="status"
    >
      <h2 className="text-xl font-semibold tracking-tight">{title}</h2>
      <p className="mt-3 text-sm leading-6 text-muted">{children}</p>
      {action ? <div className="mt-6">{action}</div> : null}
    </section>
  );
}

export function CardSkeletons({
  count = 6,
  layout = "poster",
}: {
  count?: number;
  layout?: "poster" | "backdrop";
}) {
  return Array.from({ length: count }, (_, index) => (
    <div
      key={index}
      className="min-w-0 motion-safe:animate-pulse"
      aria-hidden="true"
    >
      <div
        className={`${layout === "backdrop" ? "aspect-video" : "aspect-[2/3]"} rounded-xl bg-white/5`}
      />
      <div className="mt-3 h-4 w-3/4 rounded bg-white/5" />
      <div className="mt-2 h-3 w-1/2 rounded bg-white/5" />
    </div>
  ));
}
