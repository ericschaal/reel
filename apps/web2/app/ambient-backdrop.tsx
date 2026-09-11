"use client";

import Image from "next/image";
import { AnimatePresence, motion, MotionConfig } from "motion/react";
import {
  type CSSProperties,
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import {
  createDebouncedPublisher,
  type DebouncedPublisher,
} from "./debounced-publisher";

const LIGHT_OVERSCAN = 240;
const AMBIENT_SETTLE_DELAY_MS = 120;

type BackdropLayer = {
  fromX: number;
  fromY: number;
  revision: number;
  src: string;
  x: number;
  y: number;
};

type AmbientBackdropContextValue = {
  clear: (src: string) => void;
  show: (src: string, origin: { x: number; y: number }) => void;
};

type BackdropUpdate =
  | { type: "clear"; src: string }
  | { type: "show"; src: string; origin: { x: number; y: number } };

const AmbientBackdropContext = createContext<AmbientBackdropContextValue>({
  clear: () => undefined,
  show: () => undefined,
});

export function AmbientBackdrop({ children }: { children: ReactNode }) {
  const [backdrop, setBackdrop] = useState<BackdropLayer | null>(null);
  const lastOrigin = useRef<{ x: number; y: number } | null>(null);
  const scheduler = useRef<DebouncedPublisher<BackdropUpdate> | null>(null);

  const publishUpdate = useCallback((update: BackdropUpdate) => {
    if (update.type === "clear") {
      setBackdrop((current) =>
        current?.src === update.src ? null : current,
      );
      return;
    }

    const previousOrigin = lastOrigin.current;
    lastOrigin.current = update.origin;
    setBackdrop((current) => {
      if (
        current?.src === update.src &&
        Math.abs(current.x - update.origin.x) < 1 &&
        Math.abs(current.y - update.origin.y) < 1
      )
        return current;

      return {
        src: update.src,
        ...update.origin,
        fromX: clampTravel(
          (current?.x ?? previousOrigin?.x ?? update.origin.x) -
            update.origin.x,
          240,
        ),
        fromY: clampTravel(
          (current?.y ?? previousOrigin?.y ?? update.origin.y) -
            update.origin.y,
          140,
        ),
        revision: (current?.revision ?? 0) + 1,
      };
    });
  }, []);

  useEffect(() => {
    const activeScheduler = createDebouncedPublisher(
      AMBIENT_SETTLE_DELAY_MS,
      publishUpdate,
    );
    scheduler.current = activeScheduler;
    return () => {
      activeScheduler.dispose();
      scheduler.current = null;
    };
  }, [publishUpdate]);

  const show = useCallback(
    (src: string, origin: { x: number; y: number }) => {
      scheduler.current?.schedule({ type: "show", src, origin });
    },
    [],
  );
  const clear = useCallback(
    (src: string) => scheduler.current?.schedule({ type: "clear", src }),
    [],
  );
  const contextValue = useMemo(() => ({ clear, show }), [clear, show]);

  return (
    <AmbientBackdropContext value={contextValue}>
      <div className="relative isolate min-h-dvh bg-background">
        <div
          className="pointer-events-none fixed inset-0 -z-10 overflow-hidden"
          aria-hidden="true"
        >
          <MotionConfig reducedMotion="user">
            <AnimatePresence initial={false}>
              {backdrop ? (
                <AmbientArtworkLayer
                  key={`${backdrop.src}-${backdrop.revision}`}
                  layer={backdrop}
                />
              ) : null}
            </AnimatePresence>
          </MotionConfig>
          <div className="ambient-vignette" />
        </div>
        {children}
      </div>
    </AmbientBackdropContext>
  );
}

export function useAmbientBackdrop() {
  return useContext(AmbientBackdropContext);
}

function clampTravel(value: number, limit: number) {
  return Math.max(-limit, Math.min(limit, value));
}

function AmbientArtworkLayer({ layer }: { layer: BackdropLayer }) {
  const initialOrigin = lightOrigin(
    layer.x + layer.fromX,
    layer.y + layer.fromY,
  );
  const activeOrigin = lightOrigin(layer.x, layer.y);

  return (
    <motion.div
      className="ambient-artwork"
      style={activeOrigin as CSSProperties}
      initial={{ opacity: 0, ...initialOrigin }}
      animate={{ opacity: 1, ...activeOrigin }}
      exit={{ opacity: 0 }}
      transition={{
        opacity: { duration: 0.28, ease: "easeOut" },
        "--ambient-light-x": {
          type: "spring",
          stiffness: 105,
          damping: 24,
          mass: 0.72,
        },
        "--ambient-light-y": {
          type: "spring",
          stiffness: 105,
          damping: 24,
          mass: 0.72,
        },
      }}
    >
      <div className="ambient-light-surface ambient-light-bloom">
        <AmbientLightImage src={layer.src} />
      </div>
      <div className="ambient-light-surface ambient-light-core">
        <AmbientLightImage src={layer.src} />
      </div>
    </motion.div>
  );
}

function lightOrigin(x: number, y: number) {
  return {
    "--ambient-light-x": `${x + LIGHT_OVERSCAN}px`,
    "--ambient-light-y": `${y + LIGHT_OVERSCAN}px`,
  };
}

function AmbientLightImage({ src }: { src: string }) {
  return (
    <Image
      src={src}
      alt=""
      fill
      sizes="100vw"
      unoptimized
      decoding="async"
      className="scale-115 object-cover"
    />
  );
}
