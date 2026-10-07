import { describe, expect, it } from "vitest";
import { parseBoardSearch } from "./lib/board-search.ts";
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

    // These are SPA-only tools, without a classic page of their own.
    const internal = new Set(["/app/_kitchen-sink", "/app/r/:id/t/new"]);

    for (const route of Object.values(router.routesById)) {
      if (route.id === "__root__" || route.id === "/shell") {
        continue;
      }

      const path = routePattern(`/app${route.fullPath}`);

      expect(mapped.has(path) || internal.has(path), path).toBe(true);
    }
  });
});

describe("board route search", () => {
  it("keeps valid board search and drops invalid values", () => {
    expect(parseBoardSearch({ view: "board", status: "all", owner: 7, tag: "api" })).toEqual({
      view: "board",
      status: "all",
      owner: "7",
      tag: "api",
    });
    expect(parseBoardSearch({ view: "other", status: "blocked", owner: [], tag: false })).toEqual(
      {},
    );
  });

  it("inherits board search on posts/new, threads and ordinary room child routes", () => {
    for (const path of ["/r/900/posts/new", "/r/900/t/42", "/r/900/files", "/r/900/m/123"]) {
      const leaf = router
        .matchRoutes(path, { view: "board", status: "all", owner: "me", tag: "api", m: 123 })
        .at(-1);

      expect(leaf?.search).toMatchObject({ view: "board", status: "all", owner: "me", tag: "api" });

      if (path === "/r/900/t/42") expect(leaf?.search).toHaveProperty("m", 123);
    }

    expect(
      router.matchRoutes("/r/900/t/new", { parent: 123, status: "open" }).at(-1)?.search,
    ).toMatchObject({ parent: 123, status: "open" });
  });
});
