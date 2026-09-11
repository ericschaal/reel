"use client";

import { useEffect } from "react";

type Direction = "up" | "down" | "left" | "right";

const focusableSelector = [
  "a[href]",
  "button:not(:disabled)",
  "input:not(:disabled)",
  "select:not(:disabled)",
  "textarea:not(:disabled)",
  "summary",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

const directionForKey: Partial<Record<string, Direction>> = {
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
};

const pointerTakeoverDistance = 6;

export function KeyboardNavigation() {
  useEffect(() => {
    let inputModality = document.documentElement.dataset.inputModality;
    let pointerOrigin: HTMLElement | null = null;
    let pendingNavigationOrigin: HTMLElement | null = null;
    let lastPointerPosition: { x: number; y: number } | null = null;
    let keyboardPointerAnchor: { x: number; y: number } | null = null;
    let lastPointerTarget: HTMLElement | null = null;

    function onKeyboardIntent(event: KeyboardEvent) {
      if (["Alt", "Control", "Meta", "Shift"].includes(event.key)) return;
      if (
        directionForKey[event.key] &&
        inputModality === "pointer" &&
        pointerOrigin?.isConnected
      ) {
        pendingNavigationOrigin = pointerOrigin;
      }
      if (inputModality !== "keyboard") {
        keyboardPointerAnchor = lastPointerPosition;
      }
      inputModality = "keyboard";
      document.documentElement.dataset.inputModality = "keyboard";
    }

    function onPointerIntent(event: PointerEvent) {
      const position = { x: event.clientX, y: event.clientY };
      if (
        event.type === "pointermove" &&
        inputModality === "keyboard" &&
        keyboardPointerAnchor &&
        Math.hypot(
          position.x - keyboardPointerAnchor.x,
          position.y - keyboardPointerAnchor.y,
        ) < pointerTakeoverDistance
      ) {
        return;
      }

      lastPointerPosition = position;
      keyboardPointerAnchor = null;
      inputModality = "pointer";
      document.documentElement.dataset.inputModality = "pointer";
      const target =
        event.target instanceof Element
          ? event.target.closest<HTMLElement>(focusableSelector)
          : null;
      if (target === lastPointerTarget) return;
      lastPointerTarget = target;
      if (target && isFocusable(target)) {
        pointerOrigin = usesArrowKeys(target) ? null : target;
      }
    }

    function onKeyDown(event: KeyboardEvent) {
      const direction = directionForKey[event.key];
      const handoffOrigin = pendingNavigationOrigin;
      pendingNavigationOrigin = null;
      if (
        !direction ||
        event.defaultPrevented ||
        event.altKey ||
        event.ctrlKey ||
        event.metaKey ||
        event.shiftKey ||
        document.querySelector('[data-keyboard-navigation="managed"]')
      ) {
        return;
      }

      const eventTarget = event.target;
      if (eventTarget instanceof HTMLElement && usesArrowKeys(eventTarget)) {
        return;
      }

      const root =
        document.querySelector<HTMLDialogElement>("dialog[open]") ?? document;
      const requestedOrigin =
        handoffOrigin?.isConnected
          ? handoffOrigin
          : document.activeElement instanceof HTMLElement
            ? document.activeElement
            : null;
      const firstRail = root.querySelector<HTMLElement>(
        "[data-keyboard-rail]",
      );
      const currentMenuItem = root.querySelector<HTMLElement>(
        '[data-keyboard-menu] [aria-current="page"]',
      );
      const menuEntryTarget =
        direction === "down" &&
        requestedOrigin?.closest("[data-keyboard-menu]")
          ? firstRail?.querySelector<HTMLElement>(focusableSelector)
          : null;
      const rail =
        requestedOrigin &&
        root.contains(requestedOrigin) &&
        (direction === "left" || direction === "right")
          ? requestedOrigin.closest(
              "[data-keyboard-horizontal-group], [data-keyboard-rail]",
            )
          : null;
      const candidates = Array.from(
        (rail ?? root).querySelectorAll<HTMLElement>(focusableSelector),
      ).filter(isFocusable);
      if (!candidates.length) return;

      const activeElement =
        requestedOrigin && candidates.includes(requestedOrigin)
          ? requestedOrigin
          : null;
      const directionalTarget = activeElement
        ? findDirectionalTarget(activeElement, candidates, direction)
        : initialTarget(candidates);
      const menuReturnTarget =
        direction === "up" &&
        requestedOrigin &&
        currentMenuItem &&
        (firstRail?.contains(requestedOrigin) ||
          directionalTarget?.closest("[data-keyboard-menu]"))
          ? currentMenuItem
          : null;
      const next =
        menuReturnTarget && isFocusable(menuReturnTarget)
          ? menuReturnTarget
          : menuEntryTarget && isFocusable(menuEntryTarget)
          ? menuEntryTarget
          : directionalTarget;
      if (!next) return;

      event.preventDefault();
      next.dataset.keyboardFocusDirection = direction;
      try {
        next.focus({ preventScroll: true });
      } finally {
        delete next.dataset.keyboardFocusDirection;
      }
      revealFocusedElement(next, event.repeat);
    }

    document.addEventListener("keydown", onKeyboardIntent, true);
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("pointermove", onPointerIntent, {
      capture: true,
      passive: true,
    });
    document.addEventListener("pointerdown", onPointerIntent, {
      capture: true,
      passive: true,
    });
    return () => {
      document.removeEventListener("keydown", onKeyboardIntent, true);
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("pointermove", onPointerIntent, true);
      document.removeEventListener("pointerdown", onPointerIntent, true);
    };
  }, []);

  return null;
}

function revealFocusedElement(element: HTMLElement, keyIsRepeating: boolean) {
  const rail = element.closest("[data-keyboard-rail]");
  if (!rail) {
    element.scrollIntoView?.({ block: "nearest", inline: "nearest" });
    return;
  }

  const reducedMotion = window.matchMedia?.(
    "(prefers-reduced-motion: reduce)",
  ).matches;
  element.scrollIntoView?.({
    behavior: reducedMotion || keyIsRepeating ? "auto" : "smooth",
    block: "nearest",
    inline: "center",
  });
}

export function findDirectionalTarget(
  current: HTMLElement,
  candidates: HTMLElement[],
  direction: Direction,
) {
  const origin = current.getBoundingClientRect();
  const originX = origin.left + origin.width / 2;
  const originY = origin.top + origin.height / 2;
  let best: { element: HTMLElement; score: number } | null = null;

  for (const candidate of candidates) {
    if (candidate === current) continue;
    const bounds = candidate.getBoundingClientRect();
    const x = bounds.left + bounds.width / 2;
    const y = bounds.top + bounds.height / 2;
    const horizontal = direction === "left" || direction === "right";
    const primary = horizontal
      ? direction === "right"
        ? x - originX
        : originX - x
      : direction === "down"
        ? y - originY
        : originY - y;
    if (primary <= 0) continue;

    const secondary = horizontal ? Math.abs(y - originY) : Math.abs(x - originX);
    const aligned = horizontal
      ? rangesOverlap(origin.top, origin.bottom, bounds.top, bounds.bottom)
      : rangesOverlap(origin.left, origin.right, bounds.left, bounds.right);
    // Prefer the current row or column, then the nearest target in that lane.
    const score = primary + secondary * 2 + (aligned ? 0 : 1000);
    if (!best || score < best.score) best = { element: candidate, score };
  }

  return best?.element ?? null;
}

function initialTarget(candidates: HTMLElement[]) {
  return (
    candidates.find((candidate) => candidate.matches('[aria-current="page"]')) ??
    candidates[0]
  );
}

function isFocusable(element: HTMLElement) {
  if (element.closest("[data-keyboard-navigation-ignore]")) return false;
  const bounds = element.getBoundingClientRect();
  if (!bounds.width || !bounds.height) return false;
  if (element.closest("[inert], [aria-hidden='true']")) return false;
  const style = window.getComputedStyle(element);
  return style.display !== "none" && style.visibility !== "hidden";
}

function usesArrowKeys(element: HTMLElement) {
  if (element.isContentEditable) return true;
  if (element.matches("select, textarea")) return true;
  if (!(element instanceof HTMLInputElement)) return false;
  return !["button", "checkbox", "reset", "submit"].includes(
    element.type,
  );
}

function rangesOverlap(
  firstStart: number,
  firstEnd: number,
  secondStart: number,
  secondEnd: number,
) {
  return firstStart < secondEnd && secondStart < firstEnd;
}
