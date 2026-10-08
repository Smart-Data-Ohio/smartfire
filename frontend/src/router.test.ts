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

/** Parameter names and an index route's trailing slash do not distinguish screen patterns. */
function routePattern(pattern: string): string {
  return pattern.replace(/[$:][A-Za-z_][A-Za-z_0-9]*/g, ":id").replace(/\/$/, "");
}

describe("the screen map and the router", () => {
  it("routes every ported screen, and none the SPA forwards to classic", () => {
    for (const screen of SCREENS) {
      expect(routed(sample(screen.spa)), screen.spa).toBe(screen.ported);
    }
  });

  it("maps every public router route back to a ported classic page", () => {
    const mapped = new Set(
      SCREENS.filter((screen) => screen.ported).map((screen) => routePattern(screen.spa)),
    );

    // These are SPA-only tools, without a classic page of their own. Classic has no agent
    // profile page (only /agents/:id/approvals and /agents/:id/events), so the profile is one.
    const internal = new Set(["/app/_kitchen-sink", "/app/r/:id/t/new", "/app/agents/:id"]);

    for (const route of Object.values(router.routesById)) {
      if (route.id === "__root__" || route.id === "/shell") {
        continue;
      }

      const path = routePattern(`/app${route.fullPath}`);

      expect(mapped.has(path) || internal.has(path), path).toBe(true);
    }
  });
});
