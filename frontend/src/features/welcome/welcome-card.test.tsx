import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { WorkspaceWelcome } from "../../gen/WorkspaceWelcome.ts";
import { actions } from "../../sync/runtime.ts";
import { settings } from "../../sync/settings.ts";
import { WelcomeCard } from "./welcome-card.tsx";

const welcome: WorkspaceWelcome = {
  name: "Smart Data",
  description: "Where the team talks",
  messageHtml: '<div class="markdown-body"><p>Say <strong>hi</strong></p></div>',
  rooms: [
    { roomId: 1, name: "general", emoji: "👋", description: "Everyone", joined: true },
    { roomId: 90, name: "campfire", emoji: null, description: "", joined: false },
  ],
  guide: [{ title: "Hours", emoji: null, bodyHtml: "<p>9 to 5</p>", room: null }],
  canAdminister: false,
};

/** Renders `ui` at `/app/` with the routes its links go to; answers the current path. */
async function renderAtHome(ui: ReactNode): Promise<() => string> {
  const root = createRootRoute();
  const home = createRoute({ getParentRoute: () => root, path: "/", component: () => ui });
  const room = createRoute({ getParentRoute: () => root, path: "r/$roomId" });
  const guide = createRoute({ getParentRoute: () => root, path: "guide" });

  const router = createRouter({
    routeTree: root.addChildren([home, room, guide]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: ["/app/"] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return () => router.state.location.pathname;
}

beforeEach(() => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

it("introduces the workspace and its suggested rooms, with Open or Join for each", async () => {
  await renderAtHome(<WelcomeCard welcome={welcome} />);

  const card = screen.getByRole("region", { name: "Welcome to Smart Data" });
  expect(within(card).getByText("Where the team talks")).toBeTruthy();
  expect(within(card).getByText("hi").tagName).toBe("STRONG");

  const rooms = within(card).getByRole("list", { name: "Suggested channels" });
  expect(within(rooms).getAllByRole("listitem")).toHaveLength(2);
  expect(within(card).getByRole("link", { name: "Open #general" }).getAttribute("href")).toBe(
    "/app/r/1",
  );
  expect(within(card).getByRole("button", { name: "Join #campfire" })).toBeTruthy();
  expect(within(card).getByRole("link", { name: "Server guide" })).toBeTruthy();
  // Members don't get the edit link.
  expect(within(card).queryByRole("link", { name: "Edit welcome" })).toBeNull();
});

it("joins a suggested room and goes into it", async () => {
  const join = vi.spyOn(actions, "joinOpenRoom").mockResolvedValue();
  vi.spyOn(actions, "welcome").mockResolvedValue(welcome);
  const path = await renderAtHome(<WelcomeCard welcome={welcome} />);

  fireEvent.click(screen.getByRole("button", { name: "Join #campfire" }));

  await waitFor(() => expect(path()).toBe("/r/90"));
  expect(join).toHaveBeenCalledWith(90);
});

it("says why a join failed beside its button", async () => {
  vi.spyOn(actions, "joinOpenRoom").mockRejectedValue(new Error("Room not found"));
  await renderAtHome(<WelcomeCard welcome={welcome} />);

  fireEvent.click(screen.getByRole("button", { name: "Join #campfire" }));

  expect((await screen.findByRole("alert")).textContent).toBe("Room not found");
});

it("is dismissed for the person through their appearance preferences", async () => {
  const update = vi
    .spyOn(settings, "updateAppearance")
    .mockImplementation(() => new Promise(() => undefined));

  await renderAtHome(<WelcomeCard welcome={{ ...welcome, canAdminister: true }} />);

  expect(screen.getByRole("link", { name: "Edit welcome" }).getAttribute("href")).toBe(
    "/app/admin/workspace#welcome",
  );
  fireEvent.click(screen.getByRole("button", { name: "Dismiss welcome" }));

  await waitFor(() =>
    expect(update).toHaveBeenCalledWith({ appearancePreferences: { welcome: "dismissed" } }),
  );
});
