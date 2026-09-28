export type PlaybackPhase =
  | "loading"
  | "ready"
  | "buffering"
  | "activatingTrack"
  | "loadingTrack"
  | "error";

export type PlayerState = {
  phase: PlaybackPhase;
  isPlaying: boolean;
  playerError: string | null;
  trackSwitchError: string | null;
};

export type PlayerAction =
  | { type: "play" }
  | { type: "pause" }
  | { type: "waiting" }
  | { type: "canPlay" }
  | { type: "activateTrack" }
  | { type: "loadTrack" }
  | { type: "trackPresented" }
  | { type: "trackFailed"; message: string }
  | { type: "mediaFailed"; message: string };

export const initialPlayerState: PlayerState = {
  phase: "loading",
  isPlaying: false,
  playerError: null,
  trackSwitchError: null,
};

export function playerReducer(
  state: PlayerState,
  action: PlayerAction,
): PlayerState {
  switch (action.type) {
    case "play":
      return {
        ...state,
        phase:
          state.phase === "activatingTrack" || state.phase === "loadingTrack"
            ? state.phase
            : "ready",
        isPlaying: true,
      };
    case "pause":
      return { ...state, isPlaying: false };
    case "waiting":
      return state.phase === "activatingTrack" || state.phase === "loadingTrack"
        ? state
        : { ...state, phase: "buffering" };
    case "canPlay":
      return state.phase === "activatingTrack" || state.phase === "loadingTrack"
        ? state
        : { ...state, phase: "ready" };
    case "activateTrack":
      return {
        ...state,
        phase: "activatingTrack",
        trackSwitchError: null,
      };
    case "loadTrack":
      return { ...state, phase: "loadingTrack" };
    case "trackPresented":
      return { ...state, phase: "ready" };
    case "trackFailed":
      return {
        ...state,
        phase: "ready",
        trackSwitchError: action.message,
      };
    case "mediaFailed":
      return {
        ...state,
        phase: "error",
        isPlaying: false,
        playerError: action.message,
      };
  }
}

export type TimelineSnapshot = { currentTime: number; duration: number };

export function createTimelineStore(initialDuration: number) {
  let snapshot: TimelineSnapshot = { currentTime: 0, duration: initialDuration };
  const listeners = new Set<() => void>();
  return {
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    update(next: Partial<TimelineSnapshot>) {
      const currentTime = next.currentTime ?? snapshot.currentTime;
      const duration = next.duration ?? snapshot.duration;
      if (
        currentTime === snapshot.currentTime &&
        duration === snapshot.duration
      ) return;
      snapshot = { currentTime, duration };
      listeners.forEach((listener) => listener());
    },
  };
}

export type TimelineStore = ReturnType<typeof createTimelineStore>;
