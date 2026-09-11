import { createElement } from "react";

export function useRouter() {
  return {
    replace(href) {
      globalThis.__replacedHref = href;
    },
  };
}

export async function activatePlayback(
  _media,
  _episode,
  _resumeSeconds,
  _signal,
  _trackSelection,
  sourceSelection,
) {
  globalThis.__activationCalls += 1;
  globalThis.__lastSourceSelection = sourceSelection;
  return {
    sessionId: `session-${globalThis.__activationCalls}`,
    source: "jellyfin",
    delivery: "direct",
    mediaUrl: "/video",
    container: "mp4",
    durationSeconds: 60,
    audioTracks: [],
    subtitleTracks: [],
    selectedAudioIndex: null,
    selectedSubtitleIndex: null,
  };
}

export function PlayerView({ playback }) {
  return createElement("div", { "data-status": playback.status });
}
