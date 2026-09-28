"use client";

import {
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type MouseEvent,
} from "react";
import type { TitleMedia } from "@/app/catalogue";
import type {
  ActivePlayback,
  PlaybackDescriptor,
  PlaybackTrack,
  PlaybackTrackSelection,
} from "./playback";
import {
  createTimelineStore,
  initialPlayerState,
  playerReducer,
} from "./video-player-state";
import { PlayerTimeline, PlayerTimeSummary } from "./video-player-timeline";
import {
  PlayerSettings,
  type PlayerMenu,
  type TrackChoice,
} from "./video-player-settings";

type ReadyPlayback = Extract<ActivePlayback, { status: "ready" }>;

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
  const volumeDraggingRef = useRef(false);
  const lastAudibleVolumeRef = useRef(1);
  const playerRef = useRef<HTMLDivElement>(null);
  const videoRef = useRef<HTMLVideoElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const controlsTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const noticeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hlsRef = useRef<import("hls.js").default | null>(null);
  const resumePositionRef = useRef(playback.resumeSeconds ?? 0);
  const resumeAfterSwitchRef = useRef<boolean | null>(true);
  const frameCallbackRef = useRef<number | null>(null);
  const switchInFlightRef = useRef(false);
  const trackSwitchPhaseRef = useRef<"activating" | "loading" | null>(null);
  const [timelineStore] = useState(() =>
    createTimelineStore(playback.descriptor.durationSeconds ?? 0),
  );
  const [descriptor, setDescriptor] = useState(playback.descriptor);
  const [playerState, dispatchPlayer] = useReducer(
    playerReducer,
    initialPlayerState,
  );
  const { isPlaying, playerError, trackSwitchError } = playerState;
  const isSwitchingTracks =
    playerState.phase === "activatingTrack" ||
    playerState.phase === "loadingTrack";
  const isLoadingTrackMedia = playerState.phase === "loadingTrack";
  const isBuffering =
    playerState.phase === "loading" || playerState.phase === "buffering";
  const [volume, setVolume] = useState(1);
  const [isMuted, setIsMuted] = useState(false);
  const [playbackRate, setPlaybackRate] = useState(1);
  const [controlsVisible, setControlsVisible] = useState(true);
  const [controlNotice, setControlNotice] = useState<string | null>(null);
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
  const sourceName = descriptor.source === "aioStreams" ? "AIOStreams" : "Jellyfin";

  const revealControls = useCallback((keepOpen = false) => {
    setControlsVisible(true);
    if (controlsTimer.current) clearTimeout(controlsTimer.current);
    if (!keepOpen) {
      controlsTimer.current = setTimeout(() => {
        const video = videoRef.current;
        if (video && !video.paused && !volumeDraggingRef.current) setControlsVisible(false);
      }, 3200);
    }
  }, []);

  const showControlNotice = useCallback((message: string) => {
    if (noticeTimer.current) clearTimeout(noticeTimer.current);
    setControlNotice(message);
    noticeTimer.current = setTimeout(() => setControlNotice(null), 4000);
  }, []);
  const closeMenu = useCallback(() => setMenu(null), []);

  const togglePlay = useCallback(() => {
    const video = videoRef.current;
    if (!video) return;
    if (video.paused) {
      void video.play().catch(() => {
        dispatchPlayer({
          type: "mediaFailed",
          message: "Playback could not be resumed.",
        });
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
    timelineStore.update({ currentTime: video.currentTime });
  }, [timelineStore]);

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
    if (video.muted || video.volume === 0) {
      if (video.volume === 0) video.volume = lastAudibleVolumeRef.current;
      video.muted = false;
    } else {
      lastAudibleVolumeRef.current = video.volume;
      video.muted = true;
    }
    setVolume(video.volume);
    setIsMuted(video.muted);
    revealControls();
  }, [revealControls]);

  const toggleFullscreen = useCallback(async () => {
    const player = playerRef.current;
    if (!player) return;
    try {
      if (document.fullscreenElement) {
        await document.exitFullscreen();
      } else {
        await player.requestFullscreen();
      }
    } catch {
      showControlNotice("Fullscreen is not available in this browser.");
    }
  }, [showControlNotice]);

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
    if (
      trackSwitchPhaseRef.current === "activating" ||
      video.seeking ||
      video.readyState < HTMLMediaElement.HAVE_CURRENT_DATA
    ) return;
    const reveal = () => {
      if (trackSwitchPhaseRef.current === "activating") return;
      frameCallbackRef.current = null;
      trackSwitchPhaseRef.current = null;
      switchInFlightRef.current = false;
      hideFrozenFrame();
      dispatchPlayer({ type: "trackPresented" });
    };
    if (frameCallbackRef.current !== null) {
      video.cancelVideoFrameCallback(frameCallbackRef.current);
    }
    // loadeddata/seeked already supplies the paused frame. No further frame
    // callback is guaranteed until playback resumes.
    if (
      (!video.paused || resumeAfterSwitchRef.current === true) &&
      "requestVideoFrameCallback" in video
    ) {
      frameCallbackRef.current = video.requestVideoFrameCallback(reveal);
    } else {
      reveal();
    }
  }, [hideFrozenFrame]);

  const switchDescriptorTracks = useCallback(
    async (selection: PlaybackTrackSelection) => {
      const video = videoRef.current;
      if (!video || switchInFlightRef.current) return;
      const activationPosition = video.currentTime;
      switchInFlightRef.current = true;
      trackSwitchPhaseRef.current = "activating";
      if (frameCallbackRef.current !== null) {
        video.cancelVideoFrameCallback(frameCallbackRef.current);
        frameCallbackRef.current = null;
      }
      dispatchPlayer({ type: "activateTrack" });
      try {
        const nextDescriptor = await onSelectTracks(activationPosition, selection);
        if (videoRef.current !== video) return;
        const handoffPosition = video.currentTime;
        const shouldResume = !video.paused;
        captureCurrentFrame();
        video.pause();
        dispatchPlayer({ type: "loadTrack" });
        resumePositionRef.current = handoffPosition;
        resumeAfterSwitchRef.current = shouldResume;
        setDescriptor(nextDescriptor);
        timelineStore.update({
          duration:
            nextDescriptor.durationSeconds ?? timelineStore.getSnapshot().duration,
        });
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
        if (videoRef.current !== video) return;
        switchInFlightRef.current = false;
        trackSwitchPhaseRef.current = null;
        hideFrozenFrame();
        dispatchPlayer({
          type: "trackFailed",
          message:
            reason instanceof Error
              ? reason.message
              : "The track could not be changed.",
        });
      }
    },
    [
      captureCurrentFrame,
      hideFrozenFrame,
      onSelectTracks,
      selectedAudio,
      timelineStore,
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
    if (trackSwitchPhaseRef.current === "activating") {
      trackSwitchPhaseRef.current = "loading";
    }

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
        const selected = descriptorSubtitleTracks.find(
          (track) => track.index === descriptor.selectedSubtitleIndex,
        );
        const selectedIndex = findSubtitleTrackIndex(Array.from(video.textTracks), selected);
        Array.from(video.textTracks).forEach((track, index) => {
          track.mode = index === selectedIndex ? "showing" : "disabled";
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

    const loadNativeVideo = () => {
      video.textTracks.addEventListener("addtrack", syncNativeTextTracks);
      video.addEventListener("loadedmetadata", syncNativeTextTracks);
      video.src = descriptor.mediaUrl;
      syncNativeTextTracks();
    };

    if (descriptor.delivery === "direct") {
      loadNativeVideo();
    } else {
      // Prefer MSE for consistent subtitle rendition support. Some browsers
      // advertise native HLS but fail to demux playlists containing subtitles.
      void import("hls.js")
        .then(({ default: Hls }) => {
          if (cancelled) return;
          if (!Hls.isSupported()) {
            if (video.canPlayType("application/vnd.apple.mpegurl")) {
              loadNativeVideo();
              return;
            }
            throw new Error("This browser cannot play HLS video.");
          }
          const hls = new Hls({
            capLevelToPlayerSize: true,
            startPosition: resumePositionRef.current || -1,
          });
          hlsRef.current = hls;
          destroyHls = () => hls.destroy();
          let networkRecoveryAttempts = 0;
          let mediaRecoveryAttempts = 0;

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
            } else {
              const selected = descriptorSubtitleTracks.find(
                (track) => track.index === descriptor.selectedSubtitleIndex,
              );
              const selectedIndex = findSubtitleTrackIndex(
                hls.subtitleTracks.map((track) => ({ label: track.name, language: track.lang })),
                selected,
              );
              hls.subtitleDisplay = selectedIndex !== -1;
              hls.subtitleTrack = selectedIndex;
            }
          };

          hls.on(Hls.Events.MANIFEST_PARSED, () => {
            networkRecoveryAttempts = 0;
            syncHlsTracks();
          });
          hls.on(Hls.Events.SUBTITLE_TRACKS_UPDATED, () => {
            // hls.js applies its default after this event; restore our selection
            // after that step, including when a new rendition group appears.
            queueMicrotask(() => {
              if (!cancelled) syncHlsTracks();
            });
          });
          hls.on(Hls.Events.AUDIO_TRACK_SWITCHED, (_event, data) => {
            if (!descriptorAudioTracks.length) setSelectedAudio(data.id);
          });
          hls.on(Hls.Events.SUBTITLE_TRACK_SWITCH, (_event, data) => {
            if (!descriptorSubtitleTracks.length) setSelectedSubtitle(data.id);
          });
          hls.on(Hls.Events.ERROR, (_event, data) => {
            if (!data.fatal) return;
            if (
              data.type === Hls.ErrorTypes.NETWORK_ERROR &&
              networkRecoveryAttempts < 2
            ) {
              networkRecoveryAttempts += 1;
              hls.startLoad();
              return;
            }
            if (
              data.type === Hls.ErrorTypes.MEDIA_ERROR &&
              mediaRecoveryAttempts < 1
            ) {
              mediaRecoveryAttempts += 1;
              hls.recoverMediaError();
              return;
            }
            switchInFlightRef.current = false;
            dispatchPlayer({
              type: "mediaFailed",
              message: `${sourceName} playback stopped unexpectedly.`,
            });
          });
          hls.loadSource(descriptor.mediaUrl);
          hls.attachMedia(video);
        })
        .catch((reason: unknown) => {
          if (!cancelled) {
            switchInFlightRef.current = false;
            dispatchPlayer({
              type: "mediaFailed",
              message:
                reason instanceof Error
                  ? reason.message
                  : "The video player could not start.",
            });
          }
        });
    }

    return () => {
      cancelled = true;
      video.removeEventListener("loadedmetadata", seekToResumePosition);
      video.removeEventListener("loadedmetadata", syncNativeTextTracks);
      video.textTracks.removeEventListener("addtrack", syncNativeTextTracks);
      if (frameCallbackRef.current !== null) {
        video.cancelVideoFrameCallback(frameCallbackRef.current);
        frameCallbackRef.current = null;
      }
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
    descriptorSubtitleTracks,
    sourceName,
  ]);

  useEffect(() => {
    const onFullscreenChange = () => setIsFullscreen(Boolean(document.fullscreenElement));
    document.addEventListener("fullscreenchange", onFullscreenChange);
    return () => document.removeEventListener("fullscreenchange", onFullscreenChange);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (event.key === "Escape" && menu) {
        event.preventDefault();
        setMenu(null);
        return;
      }
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
      if (noticeTimer.current) clearTimeout(noticeTimer.current);
    };
  }, []);

  useEffect(() => {
    const finishVolumeDrag = () => {
      if (!volumeDraggingRef.current) return;
      volumeDraggingRef.current = false;
      revealControls(Boolean(menu));
    };
    window.addEventListener("pointerup", finishVolumeDrag);
    window.addEventListener("pointercancel", finishVolumeDrag);
    return () => {
      window.removeEventListener("pointerup", finishVolumeDrag);
      window.removeEventListener("pointercancel", finishVolumeDrag);
    };
  }, [menu, revealControls]);

  function updateVolume(nextVolume: number) {
    const video = videoRef.current;
    if (!video) return;
    nextVolume = Math.min(1, Math.max(0, nextVolume));
    video.volume = nextVolume;
    if (nextVolume > 0) lastAudibleVolumeRef.current = nextVolume;
    video.muted = nextVolume === 0;
    setVolume(nextVolume);
    setIsMuted(video.muted);
  }

  function chooseAudio(id: number) {
    if (id === selectedAudio || isSwitchingTracks) return;
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
    if (id === selectedSubtitle || isSwitchingTracks) return;
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
    video.defaultPlaybackRate = rate;
    video.playbackRate = rate;
    setPlaybackRate(rate);
    setMenu(null);
  }

  async function togglePictureInPicture() {
    const video = videoRef.current;
    if (!video || !("pictureInPictureEnabled" in document)) return;
    try {
      if (document.pictureInPictureElement) {
        await document.exitPictureInPicture();
      } else if ("requestPictureInPicture" in video) {
        await video.requestPictureInPicture();
      }
    } catch {
      showControlNotice("Picture in picture is not available in this browser.");
    }
  }

  function onPlayerClick(event: MouseEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget) togglePlay();
  }

  const volumeProgress = isMuted ? 0 : volume * 100;
  const hasPictureInPicture =
    typeof document !== "undefined" && "pictureInPictureEnabled" in document;

  return (
    <div
      ref={playerRef}
      data-keyboard-navigation="managed"
      className={`group relative h-dvh w-full overflow-hidden bg-black text-white ${controlsVisible || menu ? "cursor-default" : "cursor-none"}`}
      onMouseMove={() => revealControls(Boolean(menu))}
      onMouseLeave={() => {
        if (isPlaying && !menu && !volumeDraggingRef.current) setControlsVisible(false);
      }}
      onClick={onPlayerClick}
    >
      <video
        ref={videoRef}
        className="h-full w-full object-contain"
        playsInline
        onClick={() => menu ? setMenu(null) : togglePlay()}
        onDoubleClick={() => void toggleFullscreen()}
        onPlay={() => {
          dispatchPlayer({ type: "play" });
          revealControls();
        }}
        onPause={() => {
          dispatchPlayer({ type: "pause" });
          setControlsVisible(true);
        }}
        onWaiting={() => dispatchPlayer({ type: "waiting" })}
        onPlaying={() => dispatchPlayer({ type: "canPlay" })}
        onLoadedData={(event) => revealDecodedFrame(event.currentTarget)}
        onSeeked={(event) => revealDecodedFrame(event.currentTarget)}
        onCanPlay={(event) => {
          if (trackSwitchPhaseRef.current === "activating") return;
          dispatchPlayer({ type: "canPlay" });
          const shouldResume = resumeAfterSwitchRef.current;
          resumeAfterSwitchRef.current = null;
          if (shouldResume) {
            void event.currentTarget.play().catch(() => setControlsVisible(true));
          } else if (shouldResume === false) {
            event.currentTarget.pause();
          }
        }}
        onRateChange={(event) => {
          const video = event.currentTarget;
          if (video.defaultPlaybackRate !== video.playbackRate) {
            video.defaultPlaybackRate = video.playbackRate;
          }
          setPlaybackRate(video.playbackRate);
        }}
        onDurationChange={(event) =>
          timelineStore.update({ duration: event.currentTarget.duration || 0 })
        }
        onTimeUpdate={(event) =>
          timelineStore.update({ currentTime: event.currentTarget.currentTime })
        }
        onVolumeChange={(event) => {
          setVolume(event.currentTarget.volume);
          setIsMuted(event.currentTarget.muted);
        }}
        onEnded={() => dispatchPlayer({ type: "pause" })}
        onError={() => {
          switchInFlightRef.current = false;
          dispatchPlayer({
            type: "mediaFailed",
            message: `The browser could not play this ${sourceName} stream.`,
          });
        }}
      />

      <canvas
        ref={canvasRef}
        className="pointer-events-none absolute inset-0 h-full w-full object-contain opacity-0"
        aria-hidden="true"
      />

      {(isBuffering || isLoadingTrackMedia) && !playerError ? (
        <div
          className="pointer-events-none absolute inset-0 grid place-items-center"
          role="status"
          aria-label={isLoadingTrackMedia ? "Switching track" : "Buffering"}
        >
          <span className="size-14 animate-spin rounded-full border-2 border-white/25 border-t-accent" />
        </div>
      ) : null}

      {!isPlaying && !isBuffering && !isSwitchingTracks && !playerError ? (
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

      {controlNotice && !playerError ? (
        <p
          className="absolute top-24 left-1/2 z-20 -translate-x-1/2 rounded-full bg-black/75 px-4 py-2 text-sm text-white shadow-xl"
          role="status"
        >
          {controlNotice}
        </p>
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
            onClose={closeMenu}
            error={trackSwitchError}
            busy={isSwitchingTracks}
          />
        ) : null}

        <PlayerTimeline store={timelineStore} onSeek={seekTo} />

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

          <div className="flex shrink-0 items-center gap-1">
            <button type="button" className={iconButtonClass} onClick={toggleMute} aria-label={isMuted ? "Unmute" : "Mute"}>
              <VolumeIcon className="size-6" muted={isMuted || volume === 0} />
            </button>
            <input
              type="range"
              className="player-range player-volume-range w-14 shrink-0 touch-none sm:w-24"
              min="0"
              max="1"
              step="0.05"
              value={isMuted ? 0 : volume}
              style={{ "--range-progress": `${volumeProgress}%` } as CSSProperties}
              aria-label="Volume"
              aria-valuetext={`${Math.round(volumeProgress)}%`}
              onPointerDown={() => {
                volumeDraggingRef.current = true;
                revealControls(true);
              }}
              onChange={(event) => updateVolume(Number(event.currentTarget.value))}
            />
          </div>

          <PlayerTimeSummary store={timelineStore} />

          <div className="ml-auto flex items-center gap-1 sm:gap-2">
            <button
              type="button"
              className={textButtonClass}
              onClick={() => setMenu(menu ? null : "settings")}
              aria-label="Playback settings"
              aria-expanded={Boolean(menu)}
            >
              <SettingsIcon className="size-5" />
              <span className="hidden lg:inline">Settings</span>
            </button>
            {hasPictureInPicture ? (
              <button type="button" className={`${iconButtonClass} hidden sm:grid`} onClick={() => void togglePictureInPicture()} aria-label="Picture in picture">
                <PictureInPictureIcon className="size-6" />
              </button>
            ) : null}
            <button type="button" className={iconButtonClass} onClick={() => void toggleFullscreen()} aria-label={isFullscreen ? "Exit fullscreen" : "Enter fullscreen"}>
              <FullscreenIcon className="size-6" active={isFullscreen} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function findSubtitleTrackIndex(
  tracks: { label: string; language?: string | null }[],
  selected: PlaybackTrack | undefined,
) {
  if (!selected || selected.index < 0) return -1;
  const normalize = (value: string | null | undefined) => value?.trim().toLowerCase();
  const label = normalize(selected.label);
  const language = normalize(selected.language);
  const labeled = tracks.flatMap((track, index) =>
    normalize(track.label) === label ? [index] : [],
  );
  if (labeled.length === 1) return labeled[0];
  const candidates = labeled.length ? labeled : tracks.map((_, index) => index);
  const matching = language
    ? candidates.filter((index) => normalize(tracks[index].language) === language)
    : [];
  // Browser track indexes differ from Jellyfin stream indexes. Match the
  // manifest's display title/language, and never guess between ambiguous tracks.
  return matching.length === 1 ? matching[0] : -1;
}

const iconButtonClass = "grid size-11 shrink-0 place-items-center rounded-full text-white transition hover:bg-white/12 active:scale-95";
const textButtonClass = "inline-flex min-h-11 items-center gap-2 rounded-full px-3 text-sm font-semibold text-white transition hover:bg-white/12 active:scale-95";
type IconProps = { className?: string };

function PlayIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-current ${className ?? ""}`} viewBox="0 0 24 24"><path d="M7 4.8a1.2 1.2 0 0 1 1.84-1.01l11.18 7.2a1.2 1.2 0 0 1 0 2.02l-11.18 7.2A1.2 1.2 0 0 1 7 19.2V4.8Z" /></svg>; }
function PauseIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-current ${className ?? ""}`} viewBox="0 0 24 24"><path d="M6.5 4h3v16h-3zM14.5 4h3v16h-3z" /></svg>; }
function BackIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8"><path d="m14.5 5-7 7 7 7M8 12h11" /></svg>; }
function AlertIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d="M12 3 2.8 20h18.4L12 3Z" /><path d="M12 9v5m0 3v.1" /></svg>; }
function VolumeIcon({ className, muted }: IconProps & { muted: boolean }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8"><path d="M5 9h4l4-4v14l-4-4H5V9Z" />{muted ? <path d="m17 9 4 6m0-6-4 6" /> : <><path d="M16 9.5a4 4 0 0 1 0 5" /><path d="M18.5 7a7.5 7.5 0 0 1 0 10" /></>}</svg>; }
function ReplayIcon({ className, direction }: IconProps & { direction: "back" | "forward" }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d={direction === "back" ? "M5 8V4m0 4h4M5.5 8A8 8 0 1 1 4 15" : "M19 8V4m0 4h-4m3.5 0A8 8 0 1 0 20 15"} /><text x="12" y="15" textAnchor="middle" className="fill-current stroke-none text-[7px] font-bold">10</text></svg>; }
function PictureInPictureIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><rect x="3" y="5" width="18" height="14" rx="2" /><rect x="12" y="11" width="7" height="5" rx=".5" /></svg>; }
function FullscreenIcon({ className, active }: IconProps & { active: boolean }) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.8">{active ? <path d="M9 4v5H4m16 0h-5V4M4 15h5v5m6 0v-5h5" /> : <path d="M9 4H4v5m16 0V4h-5M4 15v5h5m6 0h5v-5" />}</svg>; }

function SettingsIcon({ className }: IconProps) { return <svg aria-hidden="true" className={`fill-none stroke-current ${className ?? ""}`} viewBox="0 0 24 24" strokeWidth="1.7"><path d="M4 7h7m4 0h5M4 17h3m4 0h9"/><circle cx="13" cy="7" r="2"/><circle cx="9" cy="17" r="2"/></svg>; }
