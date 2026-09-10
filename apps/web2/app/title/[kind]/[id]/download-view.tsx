import { useMemo } from "react";
import type {
  Episode,
  TitleMedia,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import {
  Eyebrow,
  glassClass,
  pageGutter,
  primaryButtonClass,
} from "../../../ui";
import { DownloadIcon, FullScreenShell } from "./playback";

export type DownloadScope =
  | { kind: "movie" }
  | { kind: "episode"; episode: Episode }
  | { kind: "series"; seasonNumbers: number[] };

export function DownloadView({
  media,
  series,
  season,
  scope,
  onChange,
  onBack,
}: {
  media: TitleMedia;
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  scope: DownloadScope;
  onChange: (scope: DownloadScope) => void;
  onBack: () => void;
}) {
  const regularSeasons = useMemo(
    () => series?.seasons.filter((item) => item.seasonNumber > 0) ?? [],
    [series],
  );
  const selectedSeasons = scope.kind === "series" ? scope.seasonNumbers : [];
  const selectedEpisodeCount = selectedSeasons.reduce((total, number) => {
    const summary = regularSeasons.find((item) => item.seasonNumber === number);
    if (season?.seasonNumber === number) {
      return (
        total +
        season.episodes.filter((episode) => !(episode.availability === "local")).length
      );
    }
    return total + (summary?.episodeCount ?? 0);
  }, 0);

  function toggleSeason(number: number) {
    const next = selectedSeasons.includes(number)
      ? selectedSeasons.filter((item) => item !== number)
      : [...selectedSeasons, number].sort((a, b) => a - b);
    onChange({ kind: "series", seasonNumbers: next });
  }

  const title =
    scope.kind === "episode"
      ? "Download episode"
      : media.kind === "series"
        ? "Download series"
        : "Download movie";

  return (
    <FullScreenShell onBack={onBack} backLabel="Back to title">
      <main className={`mx-auto max-w-3xl py-10 sm:py-16 ${pageGutter}`}>
        <Eyebrow>Save for later</Eyebrow>
        <h1 className="text-4xl leading-tight font-semibold tracking-[-0.04em] sm:text-5xl">
          {title}
        </h1>

        {scope.kind === "series" ? (
          <>
            <div className="mt-5 flex items-center justify-between gap-4">
              <p className="text-sm leading-6 text-muted">
                Choose one season or several. Episodes already in your library
                are skipped.
              </p>
              <button
                type="button"
                className="shrink-0 text-xs font-semibold text-accent hover:text-amber-200"
                onClick={() =>
                  onChange({
                    kind: "series",
                    seasonNumbers:
                      selectedSeasons.length === regularSeasons.length
                        ? []
                        : regularSeasons.map((item) => item.seasonNumber),
                  })
                }
              >
                {selectedSeasons.length === regularSeasons.length
                  ? "Clear all"
                  : "All seasons"}
              </button>
            </div>
            <fieldset className="mt-5 grid max-h-[42vh] gap-2 overflow-y-auto pr-1">
              <legend className="sr-only">Seasons to download</legend>
              {regularSeasons.map((item) => {
                const selected = selectedSeasons.includes(item.seasonNumber);
                const missing =
                  season?.seasonNumber === item.seasonNumber
                    ? season.episodes.filter((episode) => !(episode.availability === "local"))
                        .length
                    : item.episodeCount;
                return (
                  <label
                    key={item.id}
                    className={`flex min-h-16 items-center gap-3 rounded-xl border px-4 transition-colors ${selected ? "border-accent bg-accent/7" : "border-line bg-white/3 hover:bg-white/6"}`}
                  >
                    <input
                      type="checkbox"
                      checked={selected}
                      onChange={() => toggleSeason(item.seasonNumber)}
                      className="size-4 accent-accent"
                    />
                    <span className="grid min-w-0 flex-1 gap-1">
                      <strong className="text-sm">{item.title}</strong>
                      <span className="text-xs text-muted">
                        {missing ?? "Unknown number of"} episode
                        {missing === 1 ? "" : "s"} to download
                      </span>
                    </span>
                  </label>
                );
              })}
            </fieldset>
            <button
              type="button"
              disabled={!selectedSeasons.length}
              className={`${primaryButtonClass} mt-6 w-full`}
              onClick={onBack}
            >
              <DownloadIcon /> Download {selectedSeasons.length} season
              {selectedSeasons.length === 1 ? "" : "s"} ·{" "}
              {selectedEpisodeCount} episode
              {selectedEpisodeCount === 1 ? "" : "s"}
            </button>
          </>
        ) : (
          <>
            <div className={`mt-6 rounded-2xl p-5 ${glassClass}`}>
              <p className="text-sm font-semibold">
                {scope.kind === "episode"
                  ? `${media.title} · S${scope.episode.seasonNumber} E${scope.episode.episodeNumber}`
                  : media.title}
              </p>
              {scope.kind === "episode" ? (
                <p className="mt-1 text-sm text-muted">
                  {scope.episode.title}
                </p>
              ) : null}
            </div>
            <p className="mt-5 text-sm leading-6 text-muted">
              Reel will request it and add it to your library when ready.
            </p>
            <button
              type="button"
              className={`${primaryButtonClass} mt-6 w-full`}
              onClick={onBack}
            >
              <DownloadIcon /> Download
            </button>
          </>
        )}
        <p className="mt-4 text-center text-xs text-muted">
          Request preview only. Nothing will be downloaded yet.
        </p>
      </main>
    </FullScreenShell>
  );
}
