import { type ItemRequestStatus, requestLabel } from "../../../requests";

export function RequestStatusIndicator({
  status,
  seasonNumber,
  error = false,
  refreshing = false,
  onRefresh,
}: {
  status: ItemRequestStatus | undefined;
  seasonNumber?: number;
  error?: boolean;
  refreshing?: boolean;
  onRefresh?: () => void;
}) {
  const label = error ? "Status unavailable" : requestLabel(status);
  if (!label) return null;

  const attention = error
    || status?.requestStatus === "failed"
    || status?.requestStatus === "declined"
    || status?.transferStatus === "attention"
    || status?.acquisitionStatus === "blocklisted";
  const spinning = !attention && (status?.transferStatus === "downloading" || status?.transferStatus === "importing");
  const date = status?.requestedAt ? new Date(status.requestedAt) : null;
  const dateLabel = date && Number.isFinite(date.getTime())
    ? new Intl.DateTimeFormat("en-US", { month: "short", day: "numeric", timeZone: "UTC" }).format(date)
    : null;
  const fullDate = date && Number.isFinite(date.getTime())
    ? new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeZone: "UTC" }).format(date)
    : null;

  return (
    <div
      className={`inline-flex min-h-12 max-w-full items-center gap-2.5 rounded-full border py-1.5 pr-1.5 pl-4 text-sm shadow-sm ${attention ? "border-red-300/25 bg-red-300/8 text-red-200" : "border-line bg-white/5 text-ink"}`}
      role="status"
      title={status?.transferStatus === "waiting" ? "No active download is queued yet." : undefined}
    >
      <span
        className={spinning
          ? "size-3.5 shrink-0 rounded-full border-2 border-accent/30 border-t-accent motion-safe:animate-spin"
          : `size-2.5 shrink-0 rounded-full ${attention ? "bg-red-300" : "bg-accent motion-safe:animate-pulse"}`}
        aria-hidden="true"
      />
      <strong className="min-w-0 font-medium">{seasonNumber != null ? `Season ${seasonNumber} · ` : ""}{label}</strong>
      {dateLabel ? <time className="sr-only whitespace-nowrap text-xs text-muted sm:not-sr-only sm:inline sm:border-l sm:border-line sm:pl-2.5" dateTime={status?.requestedAt} title={`Requested ${fullDate}`}>Since {dateLabel}</time> : null}
      {onRefresh ? (
        <button
          type="button"
          className="grid size-9 shrink-0 place-items-center rounded-full text-muted transition-colors hover:bg-white/10 hover:text-ink disabled:opacity-50"
          onClick={onRefresh}
          disabled={refreshing}
          aria-label={refreshing ? "Refreshing request status" : "Refresh request status"}
          aria-busy={refreshing}
        >
          <svg className={`size-5 ${refreshing ? "motion-safe:animate-spin" : ""}`} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
            <path d="M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.93 9h-2.02A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4z" />
          </svg>
        </button>
      ) : null}
    </div>
  );
}
