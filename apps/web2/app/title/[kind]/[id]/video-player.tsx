"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from "react";
import type { TitleMedia } from "../../../catalogue";
import type {
  ActivePlayback,
  PlaybackDescriptor,
  PlaybackTrackSelection,
} from "./playback";

type ReadyPlayback = Extract<ActivePlayback, { status: "ready" }>;
type PlayerMenu = "audio" | "subtitles" | "speed" | null;
type TrackChoice = { id: number; label: string; language?: string };

const PLAYBACK_RATES = [0.5, 0.75, 1, 1.25, 1.5, 2];

export function ReelVideoPlayer({
  media,
  playback,
  onBack,
  onSelectTracks,
}: {
  media: TitleMedia;
  playback: ReadyPlayback;
  onBack: () => void;
  onSelectTracks: (
    resumeSeconds: number,
    selection: PlaybackTrackSelection,
  ) => Promise<PlaybackDescriptor>;
}) {
  const playerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const controlsTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hlsRef = useRef<import("hls.js").default | null>(null);
  const resumePositionRef = useRef(playback.resumeSeconds ?? 0);
  const resumeAfterSwitchRef = useRef(false);
  const [descriptor, setDescriptor] = useState(playback.descriptor);
  const [playerError, setPlayerError] = useState<string | null>(null);
  const [trackSwitchError, setTrackSwitchError] = useState<string | null>(null);
  const [isSwitchingTracks, setIsSwitchingTracks] = useState(false);
  const [isPlaying, setIsPlaying] = useState(false);
  const [isBuffering, setIsBuffering] = useState(true);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(
    descriptor.durationSeconds ?? 0,
  );
  const [volume, setVolume] = useState(1);
  const [isMuted, setIsMuted] = useState(false);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [menu, setMenu] = useState<PlayerMenu>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const descriptorAudioTracks = descriptor.audioTracks;
  const descriptorSubtitleTracks = descriptor.subtitleTracks;
  const [audioTracks, setAudioTracks] = useState<TrackChoice[]>(() =>
    descriptorAudioTracks.map((track) => ({
      id: track.index,
      label: track.label,
      language: track.language ?? undefined,
    })),
  );
  const [subtitleTracks, setSubtitleTracks] = useState<TrackChoice[]>(() =>
    descriptorSubtitleTracks.map((track) => ({
      id: track.index,
      label: track.label,
      language: track.language ?? undefined,
    })),
  );
  const [selectedAudio, setSelectedAudio] = useState(
    descriptor.selectedAudioIndex ??
      descriptorAudioTracks[0]?.index ??
      0,
  );
  const [selectedSubtitle, setSelectedSubtitle] = useState(
    descriptor.selectedSubtitleIndex ?? -1,
  );

  const episode = playback.episode;
  const title = episode ? episode.title : media.title;
  const context = episode
    ? `${media.title} · S${episode.seasonNumber} E${episode.episodeNumber}`
    : null;

  const revealControls = useCallback((keepOpen = false) => {
    setControlsVisible(true);
    if (controlsTimer.current) clearTimeout(controlsTimer.current);
    if (!keepOpen) {
      controlsTimer.current = setTimeout(() => {
        const video = videoRef.current;
        if (video && !video.paused) setControlsVisible(false);
      }, 3200);
    }
  }, []);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      void video.play().catch(() => {
        setPlayerError("Playback could not be resumed.");
      });
    } else {
      video.pause();
    }
    revealControls();
  }, [revealControls]);

  const seekTo = useCallback((seconds: number) => {
    const video = videoRef.current;
    if (!video || !Number.isFinite(seconds)) return;
    video.currentTime = Math.min(Math.max(seconds, 0), video.duration || seconds);
    setCurrentTime(video.currentTime);
  }, []);

  const seekBy = useCallback(
    (seconds: number) => {
      const video = videoRef.current;
      if (!video) return;
      seekTo(video.currentTime + seconds);
      revealControls();
    },
    [revealControls, seekTo],
  );

  const toggleMute = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    video.muted = !video.muted;
    setIsMuted(video.muted);
    revealControls();
  }, [revealControls]);

  const toggleFullscreen = useCallback(() => {
    const player = playerRef.current;
    if (!player) return;
    if (document.fullscreenElement) {
      void document.exitFullscreen();
    } else {
      void player.requestFullscreen();
    }
  }, []);

  const captureCurrentFrame = useCallback(() => {
    const video = videoRef.current;
    const canvas = canvasRef.current;
    if (!video || !canvas || video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA) {
      return;
    }
    const sourceWidth = video.videoWidth;
    const sourceHeight = video.videoHeight;
    if (!sourceWidth || !sourceHeight) return;
    const scale = Math.min(1, 1920 / sourceWidth, 1080 / sourceHeight);
    canvas.width = Math.round(sourceWidth * scale);
    canvas.height = Math.round(sourceHeight * scale);
    const context = canvas.getContext("2d");
    if (!context) return;
    context.drawImage(video, 0, 0, canvas.width, canvas.height);
    canvas.style.transition = "none";
    canvas.style.opacity = "1";
  }, []);

  const hideFrozenFrame = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    canvas.style.transition = "opacity 150ms ease";
    canvas.style.opacity = "0";
  }, []);

  const revealDecodedFrame = useCallback((video: HTMLVideoElement) => {
    const reveal = () => {
      hideFrozenFrame();
      setIsSwitchingTracks(false);
    };
    if ("requestVideoFrameCallback" in video) {
      video.requestVideoFrameCallback(reveal);
    } else {
      requestAnimationFrame(reveal);
    }
  }, [hideFrozenFrame]);

  const switchDescriptorTracks = useCallback(
    async (selection: PlaybackTrackSelection) => {
      const video = videoRef.current;
      if (!video || isSwitchingTracks) return;
      const position = video.currentTime;
      const shouldResume = !video.paused;
      captureCurrentFrame();
      setIsSwitchingTracks(true);
      setTrackSwitchError(null);
      try {
        const nextDescriptor = await onSelectTracks(position, selection);
        resumePositionRef.current = position;
        resumeAfterSwitchRef.current = shouldResume;
        setDescriptor(nextDescriptor);
        setDuration(nextDescriptor.durationSeconds ?? duration);
        setAudioTracks(
          nextDescriptor.audioTracks.map((track) => ({
            id: track.index,
            label: track.label,
            language: track.language ?? undefined,
          })),
        );
        setSubtitleTracks(
          nextDescriptor.subtitleTracks.map((track) => ({
            id: track.index,
            label: track.label,
            language: track.language ?? undefined,
          })),
        );
        setSelectedAudio(nextDescriptor.selectedAudioIndex ?? selectedAudio);
        setSelectedSubtitle(nextDescriptor.selectedSubtitleIndex ?? -1);
        setMenu(null);
      } catch (reason) {
        hideFrozenFrame();
        setIsSwitchingTracks(false);
        setTrackSwitchError(
          reason instanceof Error
            ? reason.message
            : "The track could not be changed.",
        );
      }
    },
    [
      captureCurrentFrame,
      duration,
      hideFrozenFrame,
      isSwitchingTracks,
      onSelectTracks,
      selectedAudio,
    ],
  );

  const toggleSubtitles = useCallback(() => {
    if (!subtitleTracks.length) return;
    const next = selectedSubtitle === -1 ? subtitleTracks[0].id : -1;
    if (descriptorSubtitleTracks.length) {
      void switchDescriptorTracks({
        audioStreamIndex: selectedAudio,
        subtitleStreamIndex: next,
      });
      return;
    }
    const hls = hlsRef.current;
    if (hls) {
      hls.subtitleDisplay = next !== -1;
      hls.subtitleTrack = next;
    }
    const video = videoRef.current;
    if (video && !hls) {
      Array.from(video.textTracks).forEach((track, index) => {
        track.mode = index === next ? "showing" : "disabled";
      });
    }
    setSelectedSubtitle(next);
    revealControls();
  }, [
    descriptorSubtitleTracks.length,
    revealControls,
    selectedAudio,
    selectedSubtitle,
    subtitleTracks,
    switchDescriptorTracks,
  ]);

  useEffect(() => {
    const video = videoRef.current;
    if (!video) return;
    let cancelled = false;
    let destroyHls: (() => void) | undefined;

    const seekToResumePosition = () => {
      if (
        resumePositionRef.current > 0 &&
        Number.isFinite(video.duration)
      ) {
        video.currentTime = Math.min(
          resumePositionRef.current,
          Math.max(0, video.duration - 1),
        );
      }
    };
    const syncNativeTextTracks = () => {
      if (descriptorSubtitleTracks.length) {
        Array.from(video.textTracks).forEach((track, index) => {
          track.mode =
            descriptor.selectedSubtitleIndex !== null && index === 0
              ? "showing"
              : "disabled";
        });
        return;
      }
      const tracks = Array.from(video.textTracks).map((track, index) => ({
        id: index,
        label: track.label || track.language || `Subtitle ${index + 1}`,
        language: track.language || undefined,
      }));
      setSubtitleTracks(tracks);
      const showing = Array.from(video.textTracks).findIndex(
        (track) => track.mode === "showing",
      );
      setSelectedSubtitle(showing);
    };

    video.addEventListener("loadedmetadata", seekToResumePosition, { once: true });
    video.textTracks.addEventListener("addtrack", syncNativeTextTracks);

    if (
      descriptor.delivery === "direct" ||
      video.canPlayType("application/vnd.apple.mpegurl")
    ) {
      video.src = descriptor.mediaUrl;
      syncNativeTextTracks();
    } else {
      void import("hls.js")
        .then(({ default: Hls }) => {
          if (cancelled) return;
          if (!Hls.isSupported()) {
            throw new Error("This browser cannot play HLS video.");
          }
          const hls = new Hls({
            capLevelToPlayerSize: true,
            startPosition: resumePositionRef.current || -1,
          });
          hlsRef.current = hls;
          destroyHls = () => hls.destroy();

          const syncHlsTracks = () => {
            if (!descriptorAudioTracks.length) {
              setAudioTracks(hls.audioTracks.map((track, index) => ({
                id: index,
                label: track.name || track.lang || `Audio ${index + 1}`,
                language: track.lang || undefined,
              })));
              setSelectedAudio(Math.max(0, hls.audioTrack));
            }
            if (!descriptorSubtitleTracks.length) {
              setSubtitleTracks(hls.subtitleTracks.map((track, index) => ({
                id: index,
                label: track.name || track.lang || `Subtitle ${index + 1}`,
                language: track.lang || undefined,
              })));
              setSelectedSubtitle(hls.subtitleTrack);
            } else if (descriptor.selectedSubtitleIndex === -1) {
              hls.subtitleDisplay = false;
              hls.subtitleTrack = -1;
            } else if (
              descriptor.selectedSubtitleIndex !== null &&
              hls.subtitleTracks.length
            ) {
              hls.subtitleDisplay = true;
              hls.subtitleTrack = 0;
            }
          };

          hls.on(Hls.Events.MANIFEST_PARSED, syncHlsTracks);
          hls.on(Hls.Events.AUDIO_TRACK_SWITCHED, (_event, data) => {
            if (!descriptorAudioTracks.length) setSelectedAudio(data.id);
          });
          hls.on(Hls.Events.SUBTITLE_TRACK_SWITCH, (_event, data) => {
            if (!descriptorSubtitleTracks.length) setSelectedSubtitle(data.id);
          });
          hls.on(Hls.Events.ERROR, (_event, data) => {
            if (data.fatal) {
              setPlayerError("The Jellyfin stream stopped unexpectedly.");
            }
          });
          hls.loadSource(descriptor.mediaUrl);
          hls.attachMedia(video);
        })
        .catch((reason: unknown) => {
          if (!cancelled) {
            setPlayerError(
              reason instanceof Error
                ? reason.message
                : "The video player could not start.",
            );
          }
        });
    }

    return () => {
      cancelled = true;
      video.removeEventListener("loadedmetadata", seekToResumePosition);
      video.textTracks.removeEventListener("addtrack", syncNativeTextTracks);
      destroyHls?.();
      hlsRef.current = null;
      video.removeAttribute("src");
      video.load();
    };
  }, [
    descriptor.delivery,
    descriptor.mediaUrl,
    descriptor.selectedSubtitleIndex,
    descriptorAudioTracks.length,
    descriptorSubtitleTracks.length,
  ]);

  useEffect(() => {
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.matches("input, button, select, textarea")) return;
      switch (event.key.toLowerCase()) {
        case " ":
        case "k":
          event.preventDefault();
          togglePlay();
          break;
        case "arrowleft":
        case "j":
          event.preventDefault();
          seekBy(-10);
          break;
        case "arrowright":
        case "l":
          event.preventDefault();
          seekBy(10);
          break;
        case "m":
          toggleMute();
          break;
        case "f":
          toggleFullscreen();
          break;
        case "c":
          toggleSubtitles();
          break;
        case "escape":
          if (menu) setMenu(null);
          break;
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [menu, seekBy, toggleFullscreen, toggleMute, togglePlay, toggleSubtitles]);

  useEffect(() => {
    return () => {
      if (controlsTimer.current) clearTimeout(controlsTimer.current);
    };
  }, []);

  function updateVolume(nextVolume: number) {
    const video = videoRef.current;
    if (!video) return;
    video.volume = nextVolume;
    video.muted = nextVolume === 0;
    setVolume(nextVolume);
    setIsMuted(video.muted);
  }

  function chooseAudio(id: number) {
    if (descriptorAudioTracks.length) {
      void switchDescriptorTracks({
        audioStreamIndex: id,
        subtitleStreamIndex: selectedSubtitle,
      });
      return;
    }
    const hls = hlsRef.current;
    if (hls) hls.audioTrack = id;
    setSelectedAudio(id);
    setMenu(null);
  }

  function chooseSubtitle(id: number) {
    if (descriptorSubtitleTracks.length) {
      void switchDescriptorTracks({
        audioStreamIndex: selectedAudio,
        subtitleStreamIndex: id,
      });
      return;
    }
    const hls = hlsRef.current;
    if (hls) {
      hls.subtitleDisplay = id !== -1;
      hls.subtitleTrack = id;
    } else if (videoRef.current) {
      Array.from(videoRef.current.textTracks).forEach((track, index) => {
        track.mode = index === id ? "showing" : "disabled";
      });
    }
    setSelectedSubtitle(id);
    setMenu(null);
  }

  function choosePlaybackRate(rate: number) {
    const video = videoRef.current;
    if (!video) return;
    video.playbackRate = rate;
    setPlaybackRate(rate);
    setMenu(null);
  }

  async function togglePictureInPicture() {
    const video = videoRef.current;
    if (!video || !("pictureInPictureEnabled" in document)) return;
    if (document.pictureInPictureElement) {
      await document.exitPictureInPicture();
    } else if ("requestPictureInPicture" in video) {
      await video.requestPictureInPicture();
    }
  }

  function onPlayerClick(event: MouseEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget) togglePlay();
  }

  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  const volumeProgress = isMuted ? 0 : volume * 100;
  const hasPictureInPicture =
    typeof document !== "undefined" && "pictureInPictureEnabled" in document;

  return (
    <div
      ref={playerRef}
      className={`group relative h-dvh w-full overflow-hidden bg-black text-white ${controlsVisible || menu ? "cursor-default" : "cursor-none"}`}
      onMouseMove={() => revealControls(Boolean(menu))}
      onMouseLeave={() => {
        if (isPlaying && !menu) setControlsVisible(false);
      }}
      onClick={onPlayerClick}
    >
      <video
        ref={videoRef}
        className="h-full w-full object-contain"
        autoPlay
        playsInline
        onClick={togglePlay}
        onDoubleClick={toggleFullscreen}
        onPlay={() => {
          setIsPlaying(true);
          setIsBuffering(false);
          revealControls();
        }}
        onPause={() => {
          setIsPlaying(false);
          setControlsVisible(true);
        }}
        onWaiting={() => setIsBuffering(true)}
        onPlaying={() => setIsBuffering(false)}
        onLoadedData={(event) => revealDecodedFrame(event.currentTarget)}
        onCanPlay={() => {
          setIsBuffering(false);
          if (resumeAfterSwitchRef.current) {
            resumeAfterSwitchRef.current = false;
            void videoRef.current?.play();
          }
        }}
        onDurationChange={(event) => setDuration(event.currentTarget.duration || 0)}
        onTimeUpdate={(event) => setCurrentTime(event.currentTarget.currentTime)}
        onVolumeChange={(event) => {
          setVolume(event.currentTarget.volume);
          setIsMuted(event.currentTarget.muted);
        }}
        onEnded={() => setIsPlaying(false)}
        onError={() => {
          setIsSwitchingTracks(false);
          setPlayerError("The browser could not play this Jellyfin stream.");
        }}
      />

      <canvas
        ref={canvasRef}
        className="pointer-events-none absolute inset-0 h-full w-full object-contain opacity-0"
        aria-hidden="true"
      />

      {(isBuffering || isSwitchingTracks) && !playerError ? (
        <div
          className="pointer-events-none absolute inset-0 grid place-items-center"
          role="status"
          aria-label={isSwitchingTracks ? "Switching track" : "Buffering"}
        >
          <span className="size-14 animate-spin rounded-full border-2 border-white/25 border-t-accent" />
        </div>
      ) : null}

      {!isPlaying && !isBuffering && !playerError ? (
        <button
          type="button"
          className="absolute top-1/2 left-1/2 grid size-20 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full border border-white/20 bg-black/35 shadow-2xl backdrop-blur-xl transition hover:scale-105 hover:bg-black/55"
          aria-label="Play"
          onClick={togglePlay}
        >
          <PlayIcon className="ml-1 size-8" />
        </button>
      ) : null}

      {playerError ? (
        <div className="absolute inset-0 grid place-items-center bg-black/75 px-6 text-center">
          <div className="max-w-md">
            <AlertIcon className="mx-auto size-9 text-accent" />
            <h2 className="mt-4 text-2xl font-semibold">Playback interrupted</h2>
            <p className="mt-3 text-sm leading-6 text-white/65">{playerError}</p>
            <button
              type="button"
              className="mt-6 rounded-full bg-white px-5 py-2.5 text-sm font-semibold text-black hover:bg-white/85"
              onClick={onBack}
            >
              Back to details
            </button>
          </div>
        </div>
      ) : null}

      <div
        className={`pointer-events-none absolute inset-0 bg-linear-to-b from-black/70 via-transparent via-45% to-black/90 transition-opacity duration-300 ${controlsVisible || menu ? "opacity-100" : "opacity-0"}`}
        aria-hidden="true"
      />

      <header
        className={`absolute inset-x-0 top-0 flex items-center gap-4 px-4 py-5 transition-all duration-300 sm:px-7 lg:px-10 ${controlsVisible || menu ? "translate-y-0 opacity-100" : "pointer-events-none -translate-y-4 opacity-0"}`}
      >
        <button type="button" className={iconButtonClass} onClick={onBack} aria-label="Back to details">
          <BackIcon className="size-6" />
        </button>
        <div className="min-w-0">
          <p className="truncate text-base font-semibold text-white sm:text-lg">{title}</p>
          {context ? <p className="mt-0.5 truncate text-xs text-white/55 sm:text-sm">{context}</p> : null}
        </div>
        <span className="ml-auto hidden font-mono text-[11px] font-semibold tracking-[0.22em] text-accent sm:block">REEL</span>
      </header>

      <div
        className={`absolute inset-x-0 bottom-0 px-4 pb-[max(1rem,env(safe-area-inset-bottom))] transition-all duration-300 sm:px-7 sm:pb-7 lg:px-10 ${controlsVisible || menu ? "translate-y-0 opacity-100" : "pointer-events-none translate-y-5 opacity-0"}`}
        onClick={(event) => event.stopPropagation()}
      >
        {menu ? (
          <PlayerSettings
            menu={menu}
            audioTracks={audioTracks}
            subtitleTracks={subtitleTracks}
            selectedAudio={selectedAudio}
            selectedSubtitle={selectedSubtitle}
            playbackRate={playbackRate}
            onMenuChange={setMenu}
            onAudio={chooseAudio}
            onSubtitle={chooseSubtitle}
            onPlaybackRate={choosePlaybackRate}
            onClose={() => setMenu(null)}
            error={trackSwitchError}
          />
        ) : null}

        <div className="mb-2 flex items-center justify-between text-xs font-medium text-white/70">
          <span>{formatPlayerTime(currentTime)}</span>
          <span>-{formatPlayerTime(Math.max(0, duration - currentTime))}</span>
        </div>
        <input
          type="range"
          className="player-range player-progress-range w-full"
          min="0"
          max={duration || 0}
          step="0.1"
          value={Math.min(currentTime, duration || 0)}
          style={{ "--range-progress": `${progress}%` } as CSSProperties}
          aria-label="Seek through video"
          aria-valuetext={`${formatPlayerTime(currentTime)} of ${formatPlayerTime(duration)}`}
          onChange={(event) => seekTo(Number(event.currentTarget.value))}
        />

        <div className="mt-3 flex items-center gap-1 sm:gap-2">
          <button type="button" className={iconButtonClass} onClick={togglePlay} aria-label={isPlaying ? "Pause" : "Play"}>
            {isPlaying ? <PauseIcon className="size-6" /> : <PlayIcon className="size-6" />}
          </button>
          <button type="button" className={`${iconButtonClass} hidden sm:grid`} onClick={() => seekBy(-10)} aria-label="Rewind 10 seconds">
            <ReplayIcon className="size-6" direction="back" />
          </button>
          <button type="button" className={`${iconButtonClass} hidden sm:grid`} onClick={() => seekBy(10)} aria-label="Forward 10 seconds">
            <ReplayIcon className="size-6" direction="forward" />
          </button>

          <div className="group/volume flex items-center">
            <button type="button" className={iconButtonClass} onClick={toggleMute} aria-label={isMuted ? "Unmute" : "Mute"}>
              <VolumeIcon className="size-6" muted={isMuted || volume === 0} />
            </button>
            <input
              type="range"
              className="player-range player-volume-range hidden w-0 opacity-0 transition-all duration-200 group-hover/volume:ml-1 group-hover/volume:block group-hover/volume:w-20 group-hover/volume:opacity-100 focus:ml-1 focus:block focus:w-20 focus:opacity-100 sm:block"
              min="0"
              max="1"
              step="0.05"
              value={isMuted ? 0 : volume}
              style={{ "--range-progress": `${volumeProgress}%` } as CSSProperties}
              aria-label="Volume"
              onChange={(event) => updateVolume(Number(event.currentTarget.value))}
            />
          </div>

          <span className="ml-1 hidden text-xs font-medium text-white/65 md:block">{formatPlayerTime(currentTime)} / {formatPlayerTime(duration)}</span>

          <div className="ml-auto flex items-center gap-1 sm:gap-2">
            <button
              type="button"
              className={`${textButtonClass} hidden md:inline-flex`}
              onClick={() => setMenu(menu === "speed" ? null : "speed")}
              aria-label="Playback speed"
              aria-expanded={menu === "speed"}
            >
              {playbackRate}×
            </button>
            <button
              type="button"
              className={textButtonClass}
              onClick={() => setMenu(menu === "audio" ? null : "audio")}
              aria-label="Audio and subtitles"
              aria-expanded={menu === "audio" || menu === "subtitles"}
            >
              <TracksIcon className="size-6" />
              <span className="hidden lg:inline">Audio &amp; subtitles</span>
            </button>
            {hasPictureInPicture ? (
              <button type="button" className={`${iconButtonClass} hidden sm:grid`} onClick={() => void togglePictureInPicture()} aria-label="Picture in picture">
                <PictureInPictureIcon className="size-6" />
              </button>
            ) : null}
            <button type="button" className={iconButtonClass} onClick={toggleFullscreen} aria-label={isFullscreen ? "Exit fullscreen" : "Enter fullscreen"}>
              <FullscreenIcon className="size-6" active={isFullscreen} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function PlayerSettings({
  menu,
  audioTracks,
  subtitleTracks,
  selectedAudio,
  selectedSubtitle,
  playbackRate,
  onMenuChange,
  onAudio,
  onSubtitle,
  onPlaybackRate,
  onClose,
  error,
}: {
  menu: Exclude<PlayerMenu, null>;
  audioTracks: TrackChoice[];
  subtitleTracks: TrackChoice[];
  selectedAudio: number;
  selectedSubtitle: number;
  playbackRate: number;
  onMenuChange: (menu: Exclude<PlayerMenu, null>) => void;
  onAudio: (id: number) => void;
  onSubtitle: (id: number) => void;
  onPlaybackRate: (rate: number) => void;
  onClose: () => void;
  error: string | null;
}) {
  return (
    <section className="absolute right-4 bottom-24 left-4 overflow-hidden rounded-2xl border border-white/15 bg-[#101214]/95 shadow-2xl backdrop-blur-2xl sm:right-7 sm:left-auto sm:w-[420px] lg:right-10" aria-label="Playback settings">
      <div className="flex items-center border-b border-white/10 px-2 pt-2">
        {(["audio", "subtitles", "speed"] as const).map((item) => (
          <button
            key={item}
            type="button"
            className={`relative flex-1 px-2 py-3 text-xs font-semibold capitalize transition-colors sm:text-sm ${menu === item ? "text-white" : "text-white/50 hover:text-white/80"}`}
            onClick={() => onMenuChange(item)}
          >
            {item}
            {menu === item ? <span className="absolute inset-x-3 bottom-0 h-0.5 rounded-full bg-accent" /> : null}
          </button>
        ))}
        <button type="button" className="ml-1 grid size-9 place-items-center rounded-full text-white/55 hover:bg-white/10 hover:text-white" onClick={onClose} aria-label="Close settings">
          <CloseIcon className="size-5" />
        </button>
      </div>
      <div className="max-h-64 overflow-y-auto p-2">
        {error ? (
          <p className="mx-2 mb-2 rounded-lg bg-red-400/10 px-3 py-2 text-xs leading-5 text-red-200" role="alert">
            {error}
          </p>
        ) : null}
        {menu === "audio" ? (
          audioTracks.length ? (
            audioTracks.map((track) => (
              <Choice key={track.id} selected={selectedAudio === track.id} label={track.label} detail={track.language} onClick={() => onAudio(track.id)} />
            ))
          ) : (
            <EmptyTrackState label="The stream exposes one default audio track." />
          )
        ) : null}
        {menu === "subtitles" ? (
          <>
            <Choice selected={selectedSubtitle === -1} label="Off" onClick={() => onSubtitle(-1)} />
            {subtitleTracks.length ? subtitleTracks.map((track) => (
              <Choice key={track.id} selected={selectedSubtitle === track.id} label={track.label} detail={track.language} onClick={() => onSubtitle(track.id)} />
            )) : <EmptyTrackState label="No subtitle tracks are available in this stream." />}
          </>
        ) : null}
        {menu === "speed" ? PLAYBACK_RATES.map((rate) => (
          <Choice key={rate} selected={playbackRate === rate} label={rate === 1 ? "Normal" : `${rate}×`} onClick={() => onPlaybackRate(rate)} />
        )) : null}
      </div>
    </section>
  );
}

function Choice({ selected, label, detail, onClick }: { selected: boolean; label: string; detail?: string; onClick: () => void }) {
  return (
    <button type="button" className="flex min-h-11 w-full items-center gap-3 rounded-xl px-3 text-left text-sm transition-colors hover:bg-white/8" onClick={onClick}>
      <span className={`grid size-5 place-items-center rounded-full border ${selected ? "border-accent bg-accent text-black" : "border-white/25"}`}>
        {selected ? <CheckIcon className="size-3" /> : null}
      </span>
      <span className="flex-1 font-medium">{label}</span>
      {detail && detail.toLowerCase() !== label.toLowerCase() ? <span className="text-xs uppercase text-white/40">{detail}</span> : null}
    </button>
  );
}

function EmptyTrackState({ label }: { label: string }) {
  return <p className="px-3 py-5 text-sm leading-6 text-white/45">{label}</p>;
}

function formatPlayerTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return "0:00";
  const whole = Math.floor(seconds);
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const remaining = whole % 60;
  return hours > 0
    ? `${hours}:${String(minutes).padStart(2, "0")}:${String(remaining).padStart(2, "0")}`
    : `${minutes}:${String(remaining).padStart(2, "0")}`;
}

const iconButtonClass = "grid size-11 shrink-0 place-items-center rounded-full text-white transition hover:bg-white/12 active:scale-95";
const textButtonClass = "inline-flex min-h-11 items-center gap-2 rounded-full px-3 text-sm font-semibold text-white transition hover:bg-white/12 active:scale-95";
type IconProps = { className?: string };

function PlayIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-current ${className ?? ""}`} viewBox="0 0 24 24"><path d="M7 4.8a1.2 1.2 0 0 1 1.84-1.01l11.18 7.2a1.2 1.2 0 0 1 0 2.02l-11.18 7.2A1.2 1.2 0 0 1 7 19.2V4.8Z" /></svg>; }
function PauseIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-current ${className ?? ""}`} viewBox="0 0 24 24"><path d="M6.5 4h3v16h-3zM14.5 4h3v16h-3z" /></svg>; }
function BackIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8"><path d="m14.5 5-7 7 7 7M8 12h11" /></svg>; }
function CloseIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8"><path d="m6 6 12 12M18 6 6 18" /></svg>; }
function CheckIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 16 16" strokeWidth="2.2"><path d="m3 8.5 3 3 7-7" /></svg>; }
function AlertIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d="M12 3 2.8 20h18.4L12 3Z" /><path d="M12 9v5m0 3v.1" /></svg>; }
function VolumeIcon({ className, muted }: IconProps & { muted: boolean }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8"><path d="M5 9h4l4-4v14l-4-4H5V9Z" />{muted ? <path d="m17 9 4 6m0-6-4 6" /> : <><path d="M16 9.5a4 4 0 0 1 0 5" /><path d="M18.5 7a7.5 7.5 0 0 1 0 10" /></>}</svg>; }
function ReplayIcon({ className, direction }: IconProps & { direction: "back" | "forward" }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d={direction === "back" ? "M5 8V4m0 4h4M5.5 8A8 8 0 1 1 4 15" : "M19 8V4m0 4h-4m3.5 0A8 8 0 1 0 20 15"} /><text x="12" y="15" textAnchor="middle" className="fill-current stroke-none text-[7px] font-bold">10</text></svg>; }
function TracksIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><rect x="3" y="5" width="18" height="14" rx="2" /><path d="M7 10h4m-4 4h6m3-4h1m-1 4h1" /></svg>; }
function PictureInPictureIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><rect x="3" y="5" width="18" height="14" rx="2" /><rect x="12" y="11" width="7" height="5" rx=".5" /></svg>; }
function FullscreenIcon({ className, active }: IconProps & { active: boolean }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8">{active ? <path d="M9 4v5H4m16 0h-5V4M4 15h5v5m6 0v-5h5" /> : <path d="M9 4H4v5m16 0V4h-5M4 15v5h5m6 0h5v-5" />}</svg>; }
