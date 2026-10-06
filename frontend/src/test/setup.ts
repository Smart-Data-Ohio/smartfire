import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

// jsdom's built-in stylesheet hides [popover] elements, yet it has no showPopover() to reveal
// them. A browser without the Popover API doesn't know the attribute and renders the element in
// the flow; the components fall back to exactly that, so the tests run against that behaviour.
if ("document" in globalThis) {
  const popoverFallback = document.createElement("style");

  // Same selector as jsdom's own rule: its cascade ranks by specificity, not by origin.
  popoverFallback.textContent =
    "[popover]:not(:popover-open):not(dialog[open]) { display: block; }";
  document.head.append(popoverFallback);
}

// Vitest runs without globals, so Testing Library can't register its own cleanup.
afterEach(() => {
  cleanup();
});
