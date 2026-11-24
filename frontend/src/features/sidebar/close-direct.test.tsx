import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { get } from "../../../mock/s2/testing.ts";
import { SEED_IDS } from "../../../mock/server.ts";
import { sidebarRowFixture } from "../../api/testing.ts";
import type { Me } from "../../gen/Me.ts";
import type { RoomDetail } from "../../gen/RoomDetail.ts";
import type { Sidebar } from "../../gen/Sidebar.ts";
import { mutations, sidebarRowClock, store } from "../../store/store.ts";
import { actions } from "../../sync/runtime.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { RoomRoute } from "../room/room-route.tsx";
import { AppShell } from "../shell/app-shell.tsx";
import { HomeView } from "../shell/home-view.tsx";

const DM = SEED_IDS.rooms.dmMaya;

const CLOSED_ENDPOINT = `/api/v1/rooms/${DM}/closed`;

let network: MockNetwork;

let sidebar: Sidebar;

let csrf: HTMLMetaElement;

beforeEach(async () => {
  vi.stubGlobal("matchMedia", (media: string) =>
    Object.assign(new EventTarget(), { media, matches: false, onchange: null }),
  );
  vi.stubGlobal(
    "ResizeObserver",
    class implements ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  Object.defineProperty(document, "fonts", { configurable: true, value: new EventTarget() });
  Object.defineProperty(HTMLElement.prototype, "scrollTo", {
    configurable: true,
    value: () => undefined,
  });
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
  network = installMockNetwork();
  csrf = document.createElement("meta");
  csrf.name = "csrf-token";
  csrf.content = network.server.csrfToken();
  document.head.append(csrf);

  const me = await get<Me>(network.server, "/api/v1/me");

  sidebar = await get<Sidebar>(network.server, "/api/v1/sidebar");

  mutations.reset();
  mutations.setMe({ ...me, lastRoomId: DM });
  mutations.loadSidebar(sidebar, sidebarRowClock());
  vi.spyOn(actions, "start").mockResolvedValue(undefined);
});

afterEach(() => {
  cleanup();
  csrf.remove();
  Reflect.deleteProperty(document, "fonts");
  Reflect.deleteProperty(HTMLElement.prototype, "scrollTo");
  network.restore();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function directRow() {
  const row = sidebar.rows.find((entry) => entry.room.id === DM);

  if (row === undefined) throw new Error("The test needs a seeded direct row");

  return row;
}

async function mount(path: string, openRealRoom = false) {
  const root = createRootRoute({ component: Outlet });
  const shell = createRoute({ getParentRoute: () => root, id: "shell", component: AppShell });
  const home = createRoute({ getParentRoute: () => shell, path: "/", component: HomeView });

  const room = createRoute({
    getParentRoute: () => shell,
    path: "r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: `${roomId}` }),
    },
    component: openRealRoom ? RoomRoute : () => <p>Room destination</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([shell.addChildren([home, room])]),
    history: createMemoryHistory({ initialEntries: [path] }),
    scrollRestoration: false,
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

function interceptClose() {
  const handle = network.server.handle.bind(network.server);

  return vi.spyOn(network.server, "handle").mockImplementation((request) => {
    if (request.path === CLOSED_ENDPOINT && request.method === "PUT") {
      return Promise.resolve({ status: 200, json: { row: null } });
    }

    return handle(request);
  });
}

it("the direct close button follows keyboard focus, closes the row and leaves its active route", async () => {
  const row = directRow();
  const request = interceptClose();
  const router = await mount(`/r/${DM}`);
  const user = userEvent.setup();
  const sidebarView = within(screen.getByRole("complementary", { name: "Conversations" }));
  const link = sidebarView.getByRole("link", { name: new RegExp(row.sidebarDisplayName) });

  const close = sidebarView.getByRole("button", {
    name: `Close conversation with ${row.sidebarDisplayName}`,
  });

  await user.hover(link);
  act(() => link.focus());
  await user.tab();
  expect(document.activeElement).toBe(close);
  await user.keyboard("{Enter}");

  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  expect(request).toHaveBeenCalledWith(
    expect.objectContaining({ method: "PUT", path: CLOSED_ENDPOINT, body: { closed: true } }),
  );
  expect(sidebarView.queryByRole("link", { name: new RegExp(row.sidebarDisplayName) })).toBeNull();
  expect(store.getState().sidebar.rows[DM]).toBeUndefined();
  expect(store.getState().me?.lastRoomId).toBeNull();
});

it("only direct rows and their options menus offer close conversation", async () => {
  const row = directRow();
  const request = interceptClose();
  await mount(`/r/${SEED_IDS.rooms.general}`);
  const user = userEvent.setup();
  const sidebarView = within(screen.getByRole("complementary", { name: "Conversations" }));
  const channel = sidebar.rows.find((entry) => entry.room.id === SEED_IDS.rooms.general);

  if (channel === undefined) throw new Error("The test needs a seeded channel");

  expect(
    sidebarView.queryByRole("button", {
      name: `Close conversation with ${channel.sidebarDisplayName}`,
    }),
  ).toBeNull();
  await user.click(
    sidebarView.getByRole("button", { name: `${channel.sidebarDisplayName} options` }),
  );
  expect(screen.queryByRole("menuitem", { name: "Close conversation" })).toBeNull();
  await user.keyboard("{Escape}");
  await user.click(sidebarView.getByRole("button", { name: `${row.sidebarDisplayName} options` }));
  await user.click(screen.getByRole("menuitem", { name: "Close conversation" }));
  await waitFor(() => expect(store.getState().sidebar.rows[DM]).toBeUndefined());
  expect(request).toHaveBeenCalledWith(
    expect.objectContaining({ method: "PUT", path: CLOSED_ENDPOINT, body: { closed: true } }),
  );
});

it("a link to a closed direct reopens the room and restores its sidebar row", async () => {
  const row = directRow();
  const detail = await get<RoomDetail>(network.server, `/api/v1/rooms/${DM}`);
  const handle = network.server.handle.bind(network.server);

  const request = vi.spyOn(network.server, "handle").mockImplementation((input) => {
    if (input.path === CLOSED_ENDPOINT && input.method === "PUT") {
      return Promise.resolve({ status: 200, json: { row } });
    }

    if (input.path === `/api/v1/rooms/${DM}` && input.method === "GET") {
      return Promise.resolve({
        status: 200,
        json: { ...detail, membership: { ...row.membership, closed: true } },
      });
    }

    return handle(input);
  });

  mutations.loadSidebar(
    { ...sidebar, rows: sidebar.rows.filter((entry) => entry.room.id !== DM) },
    sidebarRowClock(),
  );
  const router = await mount(`/r/${DM}`, true);

  await waitFor(() => expect(store.getState().sidebar.rows[DM]).toBeDefined());
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
  expect(request).toHaveBeenCalledWith(
    expect.objectContaining({ method: "PUT", path: CLOSED_ENDPOINT, body: { closed: false } }),
  );
});

it("group directs and the note-to-self have their own labelled close buttons", async () => {
  const group = sidebarRowFixture(200, "Project group", "direct", [2, 3]);
  const notes = { ...sidebarRowFixture(201, "Notes", "direct", [1]), noteToSelf: true };

  mutations.loadSidebar(
    { ...sidebar, rows: [...sidebar.rows, group, notes], noteToSelfRoomId: notes.room.id },
    sidebarRowClock(),
  );

  await mount(`/r/${DM}`);
  const sidebarView = within(screen.getByRole("complementary", { name: "Conversations" }));

  expect(
    sidebarView.getByRole("button", { name: "Close conversation with Project group" }),
  ).toBeTruthy();
  expect(
    sidebarView.getByRole("button", { name: /Close conversation with .+ \(you\)/ }),
  ).toBeTruthy();
});

it("a close from another tab leaves the active direct", async () => {
  const router = await mount(`/r/${DM}`);

  act(() =>
    mutations.applyEvents(
      [{ seq: 1, topic: "user", type: "sidebar.row.removed", data: { roomId: DM, closed: true } }],
      0,
    ),
  );

  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  expect(store.getState().sidebar.rows[DM]).toBeUndefined();
});
