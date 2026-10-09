import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen } from "@testing-library/react";
import { StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { router as appRouter } from "../../router.tsx";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { AppShell } from "./app-shell.tsx";
import { ROUTE_PENDING_DELAY_MS, ROUTE_PENDING_MIN_MS, RoutePending } from "./route-pending.tsx";

/** The shell around a route whose chunk never arrives, starting on a screen that is already up. */
function renderHeldRoute() {
  const root = createRootRoute({ component: Outlet });

  const shell = createRoute({
    getParentRoute: () => root,
    id: "shell",
    component: AppShell,
  });

  const home = createRoute({
    getParentRoute: () => shell,
    path: "/",
    component: () => <h1>Home</h1>,
  });

  const held = createRoute({
    getParentRoute: () => shell,
    path: "held",
    loader: () => new Promise<void>(() => {}),
    component: () => <h1>Held</h1>,
  });

  const router = createRouter({
    routeTree: root.addChildren([shell.addChildren([home, held])]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
    defaultPendingComponent: RoutePending,
    defaultPendingMs: ROUTE_PENDING_DELAY_MS,
    defaultPreload: false,
  });

  render(
    <StrictMode>
      <RouterProvider router={router} />
    </StrictMode>,
  );

  return router;
}

describe("a lazy route's chunk", () => {
  beforeEach(() => {
    // jsdom has no matchMedia; the motion helpers ask it about reduced motion.
    window.matchMedia = (query: string) =>
      Object.assign(new EventTarget(), {
        matches: false,
        media: query,
        onchange: null,
        addListener: () => undefined,
        removeListener: () => undefined,
      });
  });

  afterEach(() => {
    cleanup();
    delete document.documentElement.dataset.motion;
    mutations.reset();
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("keeps hover preload off and waits before hiding the current screen", () => {
    expect(appRouter.options.defaultPreload).toBe(false);
    expect(appRouter.options.defaultPendingComponent).toBe(RoutePending);
    expect(appRouter.options.defaultPendingMs).toBe(ROUTE_PENDING_DELAY_MS);
    expect(appRouter.options.defaultPendingMinMs).toBe(ROUTE_PENDING_MIN_MS);
  });

  it("keeps the shell up and shows the pane placeholder only after a short wait", async () => {
    vi.useFakeTimers();
    vi.spyOn(actions, "start").mockResolvedValue(undefined);
    const router = renderHeldRoute();

    await act(() => router.load());

    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Home" })).toBeTruthy();
    expect(screen.queryByRole("status", { name: "Loading page" })).toBeNull();

    await act(() => {
      void router.history.push("/held");
    });

    await act(() => vi.advanceTimersByTimeAsync(ROUTE_PENDING_DELAY_MS - 1));

    expect(screen.queryByRole("status", { name: "Loading page" })).toBeNull();
    expect(screen.getByRole("heading", { name: "Home" })).toBeTruthy();

    await act(() => vi.advanceTimersByTimeAsync(1));

    expect(screen.getByRole("status", { name: "Loading page" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Destinations" })).toBeTruthy();
  });

  it("shows the placeholder immediately when motion is reduced", async () => {
    document.documentElement.dataset.motion = "reduce";
    vi.spyOn(actions, "start").mockResolvedValue(undefined);
    const router = renderHeldRoute();

    await act(() => router.load());

    await act(() => {
      void router.history.push("/held");
    });

    expect(screen.getByRole("status", { name: "Loading page" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
  });
});
