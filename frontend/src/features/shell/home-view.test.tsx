import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { get, harness } from "../../../mock/s2/testing.ts";
import type { Me } from "../../gen/Me.ts";
import type { WorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { applyAccountAppearance } from "../../lib/appearance.ts";
import { mutations } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { settings } from "../../sync/settings.ts";
import { useShowWelcome } from "../welcome/sidebar-welcome.tsx";
import { welcomeStore } from "../welcome/welcome-store.ts";
import { HomeView, WELCOME_WAIT_MS } from "./home-view.tsx";

const LAST_ROOM = 5;

const welcome: WorkspaceWelcome = {
  name: "Smart Data",
  description: "",
  messageHtml: '<div class="markdown-body"><p>Say hi</p></div>',
  rooms: [],
  guide: [],
  canAdminister: false,
};

const none: WorkspaceWelcome = { ...welcome, messageHtml: null };

/** A promise and the function that settles it. */
function deferred<T>() {
  let resolve: (value: T) => void = () => undefined;

  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });

  return { promise, resolve };
}

/** The last room's page, with the workspace menu's "Show welcome" when it's offered. */
function RoomPage() {
  const showWelcome = useShowWelcome();

  return (
    <div>
      <h1>Last room</h1>
      {showWelcome === null ? null : (
        <button type="button" onClick={showWelcome}>
          Show welcome
        </button>
      )}
    </div>
  );
}

/** Home and the last room under `/app`, starting at `start`; answers the current path. */
async function renderShell(start: string): Promise<() => string> {
  const root = createRootRoute({ component: Outlet });
  const home = createRoute({ getParentRoute: () => root, path: "/", component: HomeView });
  const room = createRoute({ getParentRoute: () => root, path: "r/$roomId", component: RoomPage });

  const router = createRouter({
    routeTree: root.addChildren([home, room]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [`/app${start}`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return () => router.state.location.pathname;
}

beforeEach(async () => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );

  const me = await get<Me>(harness().server, "/api/v1/me");

  mutations.reset();
  mutations.setMe({ ...me, lastRoomId: LAST_ROOM });
  welcomeStore.store.setState({ status: "idle", welcome: null });
  applyAccountAppearance({ theme: "system", textSize: "default", appearancePreferences: null });
});

afterEach(() => {
  cleanup();
  Reflect.deleteProperty(document, "startViewTransition");
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("goes on to the last room when the welcome read stalls, and stays there once it lands", async () => {
  const reply = deferred<WorkspaceWelcome>();

  vi.spyOn(actions, "welcome").mockReturnValue(reply.promise);

  const pathname = await renderShell("/");

  // Held briefly, so a welcome on its way doesn't flash past...
  expect(document.querySelector('[aria-busy="true"]')).not.toBeNull();
  expect(screen.queryByRole("heading", { name: "Last room" })).toBeNull();
  expect(pathname()).toBe("/");

  // ...but not for ever.
  await waitFor(() => expect(pathname()).toBe(`/r/${LAST_ROOM}`), {
    timeout: WELCOME_WAIT_MS + 2000,
  });

  // The welcome turning up afterwards doesn't pull them back to Home.
  await act(async () => {
    reply.resolve(welcome);
    await reply.promise;
  });
  expect(welcomeStore.store.getState().welcome).toEqual(welcome);
  expect(pathname()).toBe(`/r/${LAST_ROOM}`);
  expect(screen.getByRole("heading", { name: "Last room" })).toBeTruthy();
});

it("goes on to the last room at once when the welcome can't be read", async () => {
  vi.spyOn(actions, "welcome").mockRejectedValue(new Error("offline"));

  const pathname = await renderShell("/");

  await waitFor(() => expect(pathname()).toBe(`/r/${LAST_ROOM}`), {
    timeout: WELCOME_WAIT_MS / 2,
  });
});

it("goes on to the last room when nothing is configured", async () => {
  vi.spyOn(actions, "welcome").mockResolvedValue(none);

  const pathname = await renderShell("/");

  await waitFor(() => expect(pathname()).toBe(`/r/${LAST_ROOM}`), {
    timeout: WELCOME_WAIT_MS / 2,
  });
});

it("shows the welcome on Home after Show welcome, before the preference is saved", async () => {
  applyAccountAppearance({
    theme: "system",
    textSize: "default",
    appearancePreferences: { version: 1, welcome: "dismissed" },
  });
  vi.spyOn(actions, "welcome").mockResolvedValue(welcome);

  // As in a browser, a preference change shows a frame later (a view transition): Home must not
  // act on the old answer meanwhile. The save never answers here either.
  const transitions: (() => void)[] = [];

  Object.defineProperty(document, "startViewTransition", {
    configurable: true,
    value: (change: () => void) => {
      transitions.push(change);
    },
  });

  const save = vi
    .spyOn(settings, "updateAppearance")
    .mockReturnValue(deferred<Awaited<ReturnType<typeof settings.updateAppearance>>>().promise);

  const pathname = await renderShell(`/r/${LAST_ROOM}`);

  fireEvent.click(await screen.findByRole("button", { name: "Show welcome" }));

  expect(await screen.findByRole("region", { name: "Welcome to Smart Data" })).toBeTruthy();
  expect(pathname()).toBe("/");
  await waitFor(() =>
    expect(save).toHaveBeenCalledWith({ appearancePreferences: { welcome: null } }),
  );

  // The preference catches up; Home stays put.
  act(() => {
    for (const change of transitions.splice(0)) change();
  });
  expect(screen.getByRole("region", { name: "Welcome to Smart Data" })).toBeTruthy();
  expect(pathname()).toBe("/");
});
