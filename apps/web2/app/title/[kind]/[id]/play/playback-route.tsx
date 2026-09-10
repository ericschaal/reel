"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import type { Episode, TitleMedia } from "../../../../catalogue";
import {
  activateJellyfinPlayback,
  type ActivePlayback,
  type PlaybackTrackSelection,
  PlayerView,
} from "../playback";

export function PlaybackRoute({
  media,
  episode,
  resumeSeconds,
  backHref,
}: {
  media: TitleMedia;
  episode?: Episode;
  resumeSeconds?: number;
  backHref: string;
}) {
  const router = useRouter();
  const activation = useRef<AbortController | null>(null);
  const [playback, setPlayback] = useState<ActivePlayback>({
    status: "loading",
    resumeSeconds,
    episode,
  });

  const activate = useCallback(
    (trackSelection: PlaybackTrackSelection = {}) => {
      activation.current?.abort();
      const controller = new AbortController();
      activation.current = controller;
      void activateJellyfinPlayback(
        media,
        episode,
        undefined,
        controller.signal,
        trackSelection,
      )
        .then((descriptor) => {
          if (activation.current === controller) {
            setPlayback({
              status: "ready",
              descriptor,
              resumeSeconds,
              episode,
              ...trackSelection,
            });
          }
        })
        .catch((reason: unknown) => {
          if (controller.signal.aborted || activation.current !== controller) return;
          setPlayback({
            status: "error",
            message:
              reason instanceof Error
                ? reason.message
                : "Jellyfin playback could not be started.",
            resumeSeconds,
            episode,
            ...trackSelection,
          });
        });
    },
    [episode, media, resumeSeconds],
  );

  useEffect(() => {
    activate();
    return () => {
      activation.current?.abort();
      activation.current = null;
    };
  }, [activate]);

  async function selectPlaybackTracks(
    _positionSeconds: number,
    selection: PlaybackTrackSelection,
  ) {
    activation.current?.abort();
    const controller = new AbortController();
    activation.current = controller;
    try {
      return await activateJellyfinPlayback(
        media,
        episode,
        undefined,
        controller.signal,
        selection,
      );
    } finally {
      if (activation.current === controller) activation.current = null;
    }
  }

  function closePlayback() {
    activation.current?.abort();
    activation.current = null;
    router.replace(backHref, { scroll: false });
  }

  function retryPlayback() {
    const selection = {
      audioStreamIndex: playback.audioStreamIndex,
      subtitleStreamIndex: playback.subtitleStreamIndex,
    };
    setPlayback({
      status: "loading",
      resumeSeconds,
      episode,
      ...selection,
    });
    activate(selection);
  }

  return (
    <PlayerView
      media={media}
      playback={playback}
      onBack={closePlayback}
      onRetry={retryPlayback}
      onSelectTracks={selectPlaybackTracks}
    />
  );
}
