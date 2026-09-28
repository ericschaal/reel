import type {
  TitleMedia,
  SeriesDetails,
} from "../../../catalogue";
import { Artwork } from "../../../media-card";
import {
  type MediaRequestStatus,
  type RequestProfiles,
  requestIsBlocked,
  requestLabel,
} from "../../../requests";
import { pageGutter, primaryButtonClass } from "../../../ui";
import { DownloadIcon, FullScreenShell } from "./playback";

export type DownloadScope =
  | { kind: "movie" }
  | { kind: "series"; seasonNumbers: number[] };

export function DownloadView({
  media,
  series,
  scope,
  status,
  statusLoading,
  statusError,
  profiles,
  profilesLoading,
  profilesError,
  selectedProfileId,
  submitting,
  submitError,
  submitted,
  onChange,
  onProfileChange,
  onSubmit,
  onBack,
}: {
  media: TitleMedia;
  series: SeriesDetails | null;
  scope: DownloadScope;
  status: MediaRequestStatus | null;
  statusLoading: boolean;
  statusError: string | null;
  profiles: RequestProfiles | null;
  profilesLoading: boolean;
  profilesError: string | null;
  selectedProfileId: number | null;
  submitting: boolean;
  submitError: string | null;
  submitted: boolean;
  onChange: (scope: DownloadScope) => void;
  onProfileChange: (id: number) => void;
  onSubmit: (scope: DownloadScope) => void;
  onBack: () => void;
}) {
  const regularSeasons = series?.seasons.filter((item) => item.seasonNumber > 0) ?? [];
  const availableSeasons = regularSeasons.filter((item) =>
    !requestIsBlocked(status?.seasons.find((season) => season.seasonNumber === item.seasonNumber)),
  );
  const selectedSeasons = scope.kind === "series"
    ? scope.seasonNumbers.filter((number) => availableSeasons.some((item) => item.seasonNumber === number))
    : [];

  function toggleSeason(number: number) {
    const next = selectedSeasons.includes(number)
      ? selectedSeasons.filter((item) => item !== number)
      : [...selectedSeasons, number].sort((a, b) => a - b);
    onChange({ kind: "series", seasonNumbers: next });
  }

  const title = media.kind === "series" ? "Request series" : "Request movie";
  const service = media.kind === "series" ? "Sonarr" : "Radarr";

  return (
    <FullScreenShell onBack={onBack} backLabel="Back to title">
      <main className={`relative isolate mx-auto max-w-6xl py-8 sm:py-12 ${pageGutter}`}>
        {media.images.backdrop ? (
          <div className="pointer-events-none absolute inset-x-0 top-0 -z-10 h-[32rem] overflow-hidden opacity-25" aria-hidden="true">
            <Artwork src={media.images.backdrop} sizes="100vw" />
            <div className="absolute inset-0 bg-linear-to-b from-background/30 via-background/70 to-background" />
          </div>
        ) : null}
        <header className="mb-8 flex items-start gap-5 sm:gap-7">
          <div className="relative aspect-[2/3] w-24 shrink-0 overflow-hidden rounded-xl border border-line bg-panel shadow-2xl sm:w-36">
            <Artwork src={media.images.poster} sizes="(max-width: 640px) 96px, 144px" priority />
          </div>
          <div className="min-w-0 pt-1 sm:pt-3">
            <p className="text-xs font-semibold tracking-[0.16em] text-accent uppercase">{title}</p>
            <h1 className="mt-3 text-3xl leading-tight font-semibold tracking-tight text-balance sm:text-4xl">{media.title}</h1>
            <p className="mt-2 text-sm text-muted">{media.kind === "movie" ? "Movie" : "Series"}{media.year ? ` · ${media.year}` : ""}</p>
            <p className="mt-4 max-w-xl text-sm leading-6 text-ink/75">
              Choose a quality profile{media.kind === "series" ? " and the seasons you want" : ""}. We’ll send the request to Seerr and keep its progress here.
            </p>
          </div>
        </header>
        <div className="grid items-start gap-5 lg:grid-cols-[minmax(0,1fr)_260px] lg:gap-7">
          <section className="rounded-2xl border border-white/15 bg-panel/90 p-5 shadow-xl sm:p-7" aria-label="Request options">

        {statusLoading ? <p className="text-sm text-muted" role="status">Checking request status…</p> : null}
        {statusError ? <p className="rounded-xl border border-red-300/25 bg-red-300/8 p-3 text-sm text-red-200" role="alert">{statusError}</p> : null}
        {profilesLoading ? <p className="mt-3 text-sm text-muted" role="status">Loading quality profiles…</p> : null}
        {profilesError ? <p className="mt-3 rounded-xl border border-red-300/25 bg-red-300/8 p-3 text-sm text-red-200" role="alert">{profilesError}</p> : null}
        {submitted ? (
          <div role="status">
            <span className="inline-flex size-10 items-center justify-center rounded-full bg-accent/15 text-xl text-accent" aria-hidden="true">✓</span>
            <h2 className="mt-4 text-xl font-semibold">Request sent</h2>
            <p className="mt-2 max-w-lg text-sm leading-6 text-muted">Seerr has your request for {media.title}. You can return here to see its status. Jellyfin will show it as available when a local copy is ready.</p>
            <button type="button" className={`${primaryButtonClass} mt-6`} onClick={onBack}>Back to title</button>
          </div>
        ) : (
          <>
            <h2 className="text-lg font-semibold">Your request</h2>
            <p className="mt-1 text-sm leading-6 text-muted">Choose the quality {service} should look for.</p>
            {profiles ? (
              <label className="mt-6 grid gap-2 text-sm font-semibold" htmlFor="quality-profile">
                Quality profile
                <select
                  id="quality-profile"
                  className="min-h-12 w-full rounded-xl border border-white/20 bg-background px-4 text-base font-medium text-ink transition-colors hover:border-white/40"
                  value={selectedProfileId ?? ""}
                  disabled={submitting}
                  onChange={(event) => onProfileChange(Number(event.target.value))}
                >
                  {profiles.profiles.map((profile) => (
                    <option key={profile.id} value={profile.id}>{profile.name}</option>
                  ))}
                </select>
                <span className="text-xs font-normal leading-5 text-muted">Profiles from {profiles.serverName} in Seerr</span>
              </label>
            ) : null}
            {scope.kind === "series" ? (
          <>
            <div className="mt-7 flex items-center justify-between gap-4 border-t border-line pt-6">
              <div>
                <h3 className="text-sm font-semibold">Seasons to request</h3>
                <p className="mt-1 text-xs leading-5 text-muted">Already requested or available seasons can’t be selected.</p>
              </div>
              <button
                type="button"
                className="shrink-0 text-xs font-semibold text-accent hover:text-amber-200"
                onClick={() =>
                  onChange({
                    kind: "series",
                    seasonNumbers:
                      selectedSeasons.length === availableSeasons.length
                        ? []
                        : availableSeasons.map((item) => item.seasonNumber),
                  })
                }
                disabled={!status || availableSeasons.length === 0}
              >
                {selectedSeasons.length === availableSeasons.length
                  ? "Clear all"
                  : "Select all"}
              </button>
            </div>
            <fieldset className="mt-4 grid max-h-[22rem] gap-2 overflow-y-auto pr-1">
              <legend className="sr-only">Seasons to download</legend>
              {regularSeasons.map((item) => {
                const selected = selectedSeasons.includes(item.seasonNumber);
                const itemStatus = status?.seasons.find(
                  (season) => season.seasonNumber === item.seasonNumber,
                );
                const blocked = requestIsBlocked(itemStatus);
                return (
                  <label
                    key={item.id}
                    className={`flex min-h-16 items-center gap-3 rounded-xl border px-4 transition-colors ${selected ? "border-accent/75 bg-accent/10" : "border-line bg-white/3"} ${blocked ? "opacity-60" : "hover:border-white/40"}`}
                  >
                    <input
                      type="checkbox"
                      checked={selected}
                      disabled={!status || blocked || submitting}
                      onChange={() => toggleSeason(item.seasonNumber)}
                      className="size-4 accent-accent"
                    />
                    <span className="grid min-w-0 flex-1 gap-1">
                      <strong className="text-sm">{item.title}</strong>
                      <span className="text-xs text-muted">
                        {requestLabel(itemStatus) ?? `${item.episodeCount ?? "Unknown number of"} episodes`}
                      </span>
                    </span>
                  </label>
                );
              })}
            </fieldset>
            <div className="mt-7 flex flex-col gap-4 border-t border-line pt-6 sm:flex-row sm:items-center sm:justify-between">
              <p className="max-w-sm text-xs leading-5 text-muted">Jellyfin will make the requested seasons playable once it finds a local copy.</p>
              <button
                type="button"
                disabled={!status || !profiles || selectedProfileId == null || !selectedSeasons.length || submitting}
                className={`${primaryButtonClass} w-full shrink-0 sm:w-auto`}
                onClick={() => onSubmit({ kind: "series", seasonNumbers: selectedSeasons })}
              >
                <DownloadIcon /> {submitting ? "Sending request…" : "Request"} {selectedSeasons.length} season
                {selectedSeasons.length === 1 ? "" : "s"}
              </button>
            </div>
          </>
        ) : (
          <>
            {requestLabel(status ?? undefined) ? (
              <p className="mt-6 rounded-xl border border-line bg-white/5 p-3 text-sm text-muted" role="status">{requestLabel(status ?? undefined)}</p>
            ) : null}
            <div className="mt-7 flex flex-col gap-4 border-t border-line pt-6 sm:flex-row sm:items-center sm:justify-between">
              <p className="max-w-sm text-xs leading-5 text-muted">Sending a request starts the process. Playback begins when Jellyfin has a local copy.</p>
              <button
                type="button"
                disabled={!status || !profiles || selectedProfileId == null || requestIsBlocked(status) || submitting}
                className={`${primaryButtonClass} w-full shrink-0 sm:w-auto`}
                onClick={() => onSubmit(scope)}
              >
                <DownloadIcon /> {submitting ? "Sending request…" : "Request movie"}
              </button>
            </div>
          </>
        )}
          </>
        )}
        {submitError ? <p className="mt-4 rounded-xl border border-red-300/25 bg-red-300/8 p-3 text-sm text-red-200" role="alert">{submitError}</p> : null}
          </section>
          <aside className="rounded-2xl border border-line bg-background/70 p-5 text-sm" aria-label="What happens next">
            <h2 className="font-semibold">What happens next</h2>
            <ol className="mt-5 grid gap-5 text-muted">
              <li className="flex gap-3"><span className="grid size-6 shrink-0 place-items-center rounded-full bg-accent/15 text-xs font-bold text-accent">1</span><span>Seerr receives your request and handles approval.</span></li>
              <li className="flex gap-3"><span className="grid size-6 shrink-0 place-items-center rounded-full bg-accent/15 text-xs font-bold text-accent">2</span><span>{service} finds a copy using your chosen profile.</span></li>
              <li className="flex gap-3"><span className="grid size-6 shrink-0 place-items-center rounded-full bg-accent/15 text-xs font-bold text-accent">3</span><span>Jellyfin makes it playable in Reel.</span></li>
            </ol>
          </aside>
        </div>
      </main>
    </FullScreenShell>
  );
}
