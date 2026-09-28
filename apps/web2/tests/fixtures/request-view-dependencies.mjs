import { createElement } from "react";

export const glassClass = "";
export const pageGutter = "";
export const primaryButtonClass = "";
export function Eyebrow({ children }) { return createElement("span", null, children); }
export function Artwork() { return createElement("span", { "aria-hidden": "true" }); }
export function DownloadIcon() { return createElement("span", { "aria-hidden": "true" }); }
export function FullScreenShell({ children }) { return createElement("div", null, children); }
