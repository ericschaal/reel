"use client";

import Image from "next/image";
import {
  createContext,
  type ReactNode,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";

type LayerIndex = 0 | 1;
type BackdropState = {
  active: LayerIndex | null;
  layers: [string | null, string | null];
};

type AmbientBackdropContextValue = {
  clear: (src: string) => void;
  show: (src: string) => void;
};

const AmbientBackdropContext = createContext<AmbientBackdropContextValue>({
  clear: () => undefined,
  show: () => undefined,
});

export function AmbientBackdrop({ children }: { children: ReactNode }) {
  const [backdrop, setBackdrop] = useState<BackdropState>({
    active: null,
    layers: [null, null],
  });

  const show = useCallback((src: string) => {
    setBackdrop((current) => {
      const existing = current.layers.indexOf(src);
      if (existing !== -1) {
        return { ...current, active: existing as LayerIndex };
      }

      const next: LayerIndex = current.active === 0 ? 1 : 0;
      const layers: BackdropState["layers"] = [...current.layers];
      layers[next] = src;
      return { active: next, layers };
    });
  }, []);

  const clear = useCallback((src: string) => {
    setBackdrop((current) =>
      current.active != null && current.layers[current.active] === src
        ? { ...current, active: null }
        : current,
    );
  }, []);
  const contextValue = useMemo(() => ({ clear, show }), [clear, show]);

  return (
    <AmbientBackdropContext value={contextValue}>
      <div className="relative isolate min-h-dvh bg-background">
        <div
          className="pointer-events-none fixed inset-0 -z-10 overflow-hidden"
          aria-hidden="true"
        >
          {backdrop.layers.map((src, index) =>
            src ? (
              <div
                key={`${index}-${src}`}
                className={`ambient-artwork ${backdrop.active === index ? "ambient-artwork-active" : ""}`}
              >
                <AmbientLavaPool src={src} variant="primary" />
                <AmbientLavaPool src={src} variant="secondary" />
                <AmbientLavaPool src={src} variant="accent" />
              </div>
            ) : null,
          )}
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

function AmbientLavaPool({
  src,
  variant,
}: {
  src: string;
  variant: "primary" | "secondary" | "accent";
}) {
  return (
    <div className={`ambient-lava-pool ambient-lava-pool-${variant}`}>
      <Image
        src={src}
        alt=""
        fill
        sizes="100vw"
        unoptimized
        decoding="async"
        className="scale-125 object-cover"
      />
    </div>
  );
}
