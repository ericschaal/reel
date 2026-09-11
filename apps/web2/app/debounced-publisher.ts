export type DebouncedPublisher<T> = {
  dispose: () => void;
  schedule: (value: T) => void;
};

export function createDebouncedPublisher<T>(
  delayMs: number,
  publish: (value: T) => void,
): DebouncedPublisher<T> {
  let timeout: ReturnType<typeof setTimeout> | null = null;
  let pending: T;
  let hasPending = false;

  function dispose() {
    if (timeout) clearTimeout(timeout);
    timeout = null;
    hasPending = false;
  }

  return {
    schedule(value: T) {
      pending = value;
      hasPending = true;
      if (timeout) clearTimeout(timeout);
      timeout = setTimeout(() => {
        timeout = null;
        if (!hasPending) return;
        hasPending = false;
        publish(pending);
      }, delayMs);
    },
    dispose,
  };
}
