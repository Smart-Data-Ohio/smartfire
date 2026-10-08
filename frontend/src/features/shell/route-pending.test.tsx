import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen } from "@testing-library/react";
import { lazy, StrictMode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { router as appRouter } from "../../router.tsx";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { AppShell } from "./app-shell.tsx";
import { ROUTE_PENDING_DELAY_MS, RoutePending } from "./route-pending.tsx";

const held = new Promise<{ default: () => null }>(() => {});

const HeldRoute = lazy(() => held);

/** The shell around a lazy route whose chunk never arrives. */
function renderHeldRoute() {
  const root = createRootRoute({ component: Outlet });

  const shell = createRoute({
    getParentRoute: () => root,
    id: "shell",
    component: AppShell,
  });

  const route = createRoute({
    getParentRoute: () => shell,
    path: "held",
    component: HeldRoute,
  });

  const router = createRouter({
    routeTree: root.addChildren([shell.addChildren([route])]),
    history: createMemoryHistory({ initialEntries: ["/held"] }),
    defaultPendingComponent: RoutePending,
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

  it("keeps the router's hover preload off and catches the chunk in the pane", () => {
    expect(appRouter.options.defaultPreload).toBe(false);
    expect(appRouter.options.defaultPendingComponent).toBe(RoutePending);
  });

  it("keeps the shell up and shows the pane placeholder only after a short wait", async () => {
    vi.useFakeTimers();
    vi.spyOn(actions, "start").mockResolvedValue(undefined);
    const router = renderHeldRoute();

    await act(() => router.load());

    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Destinations" })).toBeTruthy();
    expect(document.querySelector(".app-shell")).not.toBeNull();
    expect(screen.queryByRole("status", { name: "Loading page" })).toBeNull();

    await act(() => vi.advanceTimersByTimeAsync(ROUTE_PENDING_DELAY_MS));

    expect(screen.getByRole("status", { name: "Loading page" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
    expect(screen.getByRole("navigation", { name: "Destinations" })).toBeTruthy();
  });

  it("shows the placeholder immediately when motion is reduced", async () => {
    document.documentElement.dataset.motion = "reduce";
    vi.spyOn(actions, "start").mockResolvedValue(undefined);
    const router = renderHeldRoute();

    await act(() => router.load());

    expect(screen.getByRole("status", { name: "Loading page" })).toBeTruthy();
    expect(screen.getByRole("complementary", { name: "Conversations" })).toBeTruthy();
  });
});
