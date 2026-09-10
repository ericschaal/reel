"use client";

import { useEffect, useRef, useState } from "react";

type IntersectionObserverOptions = {
  enabled?: boolean;
  once?: boolean;
  rootMargin?: string;
};

export function useIntersectionObserver<T extends Element>({
  enabled = true,
  once = false,
  rootMargin = "0px",
}: IntersectionObserverOptions = {}) {
  const ref = useRef<T>(null);
  const [isIntersecting, setIsIntersecting] = useState(false);

  useEffect(() => {
    const element = ref.current;
    if (!enabled || !element) return;

    if (!("IntersectionObserver" in window)) {
      const frame = requestAnimationFrame(() => setIsIntersecting(true));
      return () => cancelAnimationFrame(frame);
    }

    const observer = new IntersectionObserver(
      ([entry]) => {
        const nextValue = entry?.isIntersecting ?? false;
        setIsIntersecting(nextValue);
        if (nextValue && once) observer.disconnect();
      },
      { rootMargin },
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [enabled, once, rootMargin]);

  return { ref, isIntersecting };
}
