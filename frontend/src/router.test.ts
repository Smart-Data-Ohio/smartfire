import { describe, expect, it } from "vitest";
import { SCREENS } from "./lib/screens.ts";
import { router } from "./router.tsx";

/** `pattern` with its parameters filled with 7, 8, 9 in order. */
function sample(pattern: string): string {
  let next = 7;

  return pattern.replace(/:[a-z_]+/g, () => String(next++));
}

/** Whether the router has a route for the SPA URL `path` (a leaf route, not a not-found). */
function routed(path: string): boolean {
  // The router matches below its `/app/` basepath.
  const matches = router.matchRoutes(path.replace(/^\/app/, ""), {});

  const leaf = matches.at(-1)?.routeId;

  // Unmatched paths stop at the root, or at the pathless shell (its not-found fallback).
  return leaf !== undefined && leaf !== "__root__" && leaf !== "/shell";
}

describe("the screen map and the router", () => {
  it("routes every ported screen, and none the SPA forwards to classic", () => {
    for (const screen of SCREENS) {
      expect(routed(sample(screen.spa)), screen.spa).toBe(screen.ported);
    }
  });
});
