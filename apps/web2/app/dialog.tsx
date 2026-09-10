"use client";

import { useEffect, useRef, type ReactNode } from "react";
import { glassClass } from "./ui";

// The native modal supplies focus containment, Escape dismissal, and an inert
// background. Mount only while open so each opening captures its own trigger.
export function Dialog({
  children,
  labelledBy,
  onClose,
  className = "",
}: {
  children: ReactNode;
  labelledBy: string;
  onClose: () => void;
  className?: string;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current!;
    const trigger =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;
    const previousOverflow = document.body.style.overflow;
    dialog.showModal();
    document.body.style.overflow = "hidden";
    return () => {
      dialog.close();
      document.body.style.overflow = previousOverflow;
      if (trigger?.isConnected) trigger.focus();
    };
  }, []);

  return (
    <dialog
      ref={ref}
      aria-labelledby={labelledBy}
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      className={`fixed inset-0 m-auto max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-2xl overflow-y-auto overscroll-contain rounded-2xl p-0 text-ink backdrop:bg-black/55 backdrop:backdrop-blur-sm ${glassClass} ${className}`}
    >
      <div className="p-5 sm:p-8">{children}</div>
    </dialog>
  );
}
