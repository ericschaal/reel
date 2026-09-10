import type {
  Episode,
  TitleMedia,
  PlaybackProgress,
  SeasonDetails,
  SeriesDetails,
} from "../../../catalogue";
import { Artwork } from "../../../media-card";
import { buttonClass, Eyebrow, glassClass, pageGutter } from "../../../ui";
import { EpisodeMetadata, EpisodeRail, SeasonSelector } from "./episodes";
import {
  DownloadIcon,
  DownloadedStatus,
  FullScreenShell,
  PlaybackControl,
  PlaybackHint,
  ProgressBar,
} from "./playback";

export function EpisodeDetailView({
  media,
  episode,
  series,
  season,
  seasonNumber,
  seasonLoading,
  seasonError,
  progress,
  onBack,
  onSelectSeason,
  onOpenEpisode,
  onPlayLocal,
  onDownload,
}: {
  media: TitleMedia;
  episode: Episode;
  series: SeriesDetails | null;
  season: SeasonDetails | null;
  seasonNumber: number | null;
  seasonLoading: boolean;
  seasonError: string | null;
  progress: PlaybackProgress | null;
  onBack: () => void;
  onSelectSeason: (seasonNumber: number) => void;
  onOpenEpisode: (episode: Episode) => void;
  onPlayLocal: (resumeSeconds?: number) => void;
  onDownload: () => void;
}) {
  const localCopy = episode.availability === "local";

  return (
    <FullScreenShell onBack={onBack} backLabel="Back to episodes">
      <main className={`mx-auto max-w-[1400px] py-10 sm:py-16 ${pageGutter}`}>
        <section className="grid items-center gap-8 lg:grid-cols-[minmax(0,1.3fr)_minmax(360px,0.7fr)] lg:gap-14">
          <div className="relative aspect-video overflow-hidden rounded-2xl border border-line bg-panel shadow-2xl">
            <Artwork
              src={episode.still}
              sizes="(max-width: 1024px) 100vw, 65vw"
              priority
            />
            {progress ? <ProgressBar progress={progress} /> : null}
          </div>
          <div className="min-w-0">
            <Eyebrow>
              Season {episode.seasonNumber} · Episode {episode.episodeNumber}
            </Eyebrow>
            <p className="mb-3 text-sm font-semibold text-muted">
              {media.title}
            </p>
            <h1 className="text-4xl leading-tight font-semibold tracking-[-0.04em] sm:text-5xl">
              {episode.title}
            </h1>
            <div className="mt-5">
              <EpisodeMetadata episode={episode} />
            </div>
            <p className="mt-5 text-base leading-7 text-ink/75">
              {episode.overview || `An episode of ${media.title}.`}
            </p>
            <div className="mt-7 flex flex-wrap items-stretch gap-3">
              <PlaybackControl
                progress={progress}
                disabled={!localCopy}
                onPlay={onPlayLocal}
              />
              {episode.availability === "local" ? (
                <DownloadedStatus />
              ) : (
                <button
                  type="button"
                  className={buttonClass}
                  onClick={onDownload}
                >
                  <DownloadIcon /> Download
                </button>
              )}
            </div>
            <PlaybackHint progress={progress} />
          </div>
        </section>
        <section
          className="mt-14 border-t border-white/10 pt-10 sm:mt-20 sm:pt-12"
          aria-labelledby="season-episodes-heading"
        >
          <div className="flex flex-wrap items-end justify-between gap-4">
            <div>
              <Eyebrow>Keep watching</Eyebrow>
              <h2
                id="season-episodes-heading"
                className="text-2xl font-semibold tracking-tight sm:text-3xl"
              >
                Episodes
              </h2>
            </div>
            <SeasonSelector
              series={series}
              seasonNumber={seasonNumber}
              loading={seasonLoading}
              onSelect={onSelectSeason}
            />
          </div>
          {seasonError ? (
            <div
              className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}
            >
              {seasonError}
            </div>
          ) : seasonLoading ? (
            <div
              className="mt-6 flex gap-4 overflow-hidden"
              role="status"
              aria-label="Loading episodes"
            >
              {[0, 1, 2].map((item) => (
                <div
                  key={item}
                  className="aspect-video w-[78vw] max-w-xs shrink-0 rounded-xl bg-white/5 motion-safe:animate-pulse"
                />
              ))}
            </div>
          ) : season?.episodes.length ? (
            <EpisodeRail
              episodes={season.episodes}
              currentEpisodeId={episode.id}
              onOpenEpisode={onOpenEpisode}
            />
          ) : (
            <div
              className={`mt-6 rounded-2xl p-5 text-sm text-muted ${glassClass}`}
            >
              No episodes are available for this season yet.
            </div>
          )}
        </section>
      </main>
    </FullScreenShell>
  );
}
