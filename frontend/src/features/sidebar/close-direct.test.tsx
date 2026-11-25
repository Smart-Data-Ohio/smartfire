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
import { beginRoomRequest } from "../../store/join-state.ts";
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

it("the keyboard close button leaves the active direct and focuses its next row", async () => {
  const row = directRow();

  mutations.loadSidebar(
    {
      ...sidebar,
      rows: sidebar.rows.map((entry) =>
        entry.room.id === DM
          ? { ...row, membership: { ...row.membership, favoritePosition: null } }
          : entry,
      ),
    },
    sidebarRowClock(),
  );

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
    expect.objectContaining({
      method: "PUT",
      path: CLOSED_ENDPOINT,
      body: { closed: true, lastMessageId: row.lastMessage?.messageId },
    }),
  );
  expect(sidebarView.queryByRole("link", { name: new RegExp(row.sidebarDisplayName) })).toBeNull();
  expect(store.getState().sidebar.rows[DM]).toBeUndefined();
  expect(store.getState().me?.lastRoomId).toBeNull();
  expect(document.activeElement).toBe(sidebarView.getByRole("link", { name: "Ember" }));
});

it("closing a direct with a legacy preview omits the message precondition", async () => {
  const row = directRow();
  const request = interceptClose();

  const legacyRow = {
    ...row,
    lastMessage: { creatorId: 1, excerpt: "Earlier sidebar payload", createdAt: row.evaluatedAt },
  };

  mutations.loadSidebar(
    { ...sidebar, rows: sidebar.rows.map((entry) => (entry.room.id === DM ? legacyRow : entry)) },
    sidebarRowClock(),
  );

  await mount(`/r/${DM}`);

  const close = screen.getByRole("button", {
    name: `Close conversation with ${row.sidebarDisplayName}`,
  });

  await userEvent.click(close);

  await waitFor(() =>
    expect(request).toHaveBeenCalledWith(
      expect.objectContaining({ method: "PUT", path: CLOSED_ENDPOINT, body: { closed: true } }),
    ),
  );
});

it("closing the only direct from its menu focuses the Direct messages heading", async () => {
  const row = directRow();

  mutations.loadSidebar(
    {
      ...sidebar,
      rows: sidebar.rows.flatMap((entry) => {
        if (entry.room.id === DM) {
          return [{ ...row, membership: { ...row.membership, favoritePosition: null } }];
        }

        return entry.room.kind === "direct" ? [] : [entry];
      }),
    },
    sidebarRowClock(),
  );

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
  act(() => sidebarView.getByRole("link", { name: new RegExp(row.sidebarDisplayName) }).focus());
  await user.keyboard("{Shift>}{F10}{/Shift}");
  act(() => screen.getByRole("menuitem", { name: "Close conversation" }).focus());
  await user.keyboard("{Enter}");
  await waitFor(() => expect(store.getState().sidebar.rows[DM]).toBeUndefined());
  await waitFor(() =>
    expect(document.activeElement).toBe(
      sidebarView.getByRole("button", { name: "Direct messages" }),
    ),
  );
  expect(request).toHaveBeenCalledWith(
    expect.objectContaining({
      method: "PUT",
      path: CLOSED_ENDPOINT,
      body: { closed: true, lastMessageId: row.lastMessage?.messageId },
    }),
  );
});

it("a closed direct loads its timeline during a reopen write and retries a refusal", async () => {
  const row = directRow();
  const detail = await get<RoomDetail>(network.server, `/api/v1/rooms/${DM}`);
  const handle = network.server.handle.bind(network.server);
  const reopen = Promise.withResolvers<Awaited<ReturnType<typeof handle>>>();
  let reopenAttempts = 0;

  const request = vi.spyOn(network.server, "handle").mockImplementation((input) => {
    if (input.path === CLOSED_ENDPOINT && input.method === "PUT") {
      reopenAttempts += 1;

      return reopenAttempts === 1
        ? reopen.promise
        : Promise.resolve({ status: 200, json: { row } });
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

  expect(router.state.location.pathname).toBe(`/r/${DM}`);

  try {
    await waitFor(() => expect(store.getState().timelines[DM]?.ids.length).toBeGreaterThan(0));
    expect(request).toHaveBeenCalledWith(
      expect.objectContaining({ method: "PUT", path: CLOSED_ENDPOINT, body: { closed: false } }),
    );
    expect(store.getState().sidebar.rows[DM]).toBeUndefined();
  } finally {
    await act(async () => reopen.resolve({ status: 500, json: null }));
  }

  await act(() => actions.reloadRoom(DM, null));
  await waitFor(() => expect(store.getState().sidebar.rows[DM]).toBeDefined());
  expect(reopenAttempts).toBe(2);
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
  expect(store.getState().me?.lastRoomId).toBeNull();
});

it("losing access to a group direct keeps its unavailable room on screen", async () => {
  const roomId = SEED_IDS.rooms.groupDm;
  const me = await get<Me>(network.server, "/api/v1/me");
  mutations.setMe({ ...me, lastRoomId: roomId });
  const router = await mount(`/r/${roomId}`, true);
  const navigate = vi.spyOn(router, "navigate");
  await waitFor(() => expect(store.getState().timelines[roomId]?.ids.length).toBeGreaterThan(0));

  act(() => {
    mutations.applyEvents(
      [
        {
          seq: 1,
          topic: "user",
          type: "sidebar.row.removed",
          data: { roomId, refreshRoom: true },
        },
      ],
      0,
    );
    mutations.setRoomUnavailable(roomId, beginRoomRequest(), sidebarRowClock());
  });

  expect(await screen.findByRole("region", { name: "Room unavailable" })).toBeTruthy();
  expect(navigate).not.toHaveBeenCalledWith(expect.objectContaining({ to: "/" }));
  expect(router.state.location.pathname).toBe(`/r/${roomId}`);
  expect(store.getState().me?.lastRoomId).toBe(roomId);
});

it("a sidebar snapshot without the active direct does not imply a close", async () => {
  const router = await mount(`/r/${DM}`);

  act(() =>
    mutations.loadSidebar(
      { ...sidebar, rows: sidebar.rows.filter((entry) => entry.room.id !== DM) },
      sidebarRowClock(),
    ),
  );

  expect(router.state.location.pathname).toBe(`/r/${DM}`);
  expect(store.getState().me?.lastRoomId).toBe(DM);
});

it("a late plain removal after reopening leaves the direct route in place", async () => {
  mutations.loadSidebar(
    {
      ...sidebar,
      rows: sidebar.rows.filter((row) => row.room.id !== DM),
      closedDirectRoomIds: [DM],
    },
    sidebarRowClock(),
  );
  const router = await mount(`/r/${DM}`);
  const navigate = vi.spyOn(router, "navigate");

  act(() =>
    mutations.applyEvents(
      [
        { seq: 1, topic: "user", type: "sidebar.row.upserted", data: directRow() },
        { seq: 2, topic: "user", type: "sidebar.row.removed", data: { roomId: DM } },
      ],
      0,
    ),
  );

  expect(navigate).not.toHaveBeenCalled();
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
});

it("the first sidebar snapshot leaves a closed direct link open", async () => {
  mutations.reset();

  const router = await mount(`/r/${DM}`);
  const navigate = vi.spyOn(router, "navigate");

  act(() =>
    mutations.loadSidebar(
      {
        ...sidebar,
        rows: sidebar.rows.filter((row) => row.room.id !== DM),
        closedDirectRoomIds: [DM],
      },
      sidebarRowClock(),
    ),
  );

  expect(navigate).not.toHaveBeenCalled();
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
});

it("a cold-load reopen outranks an older sidebar snapshot listing the direct closed", async () => {
  const row = directRow();
  const detail = await get<RoomDetail>(network.server, `/api/v1/rooms/${DM}`);
  const handle = network.server.handle.bind(network.server);

  const closedSidebar = {
    ...sidebar,
    rows: sidebar.rows.filter((entry) => entry.room.id !== DM),
    closedDirectRoomIds: [DM],
  };

  vi.spyOn(network.server, "handle").mockImplementation((request) => {
    if (request.path === CLOSED_ENDPOINT && request.method === "PUT") {
      return Promise.resolve({ status: 200, json: { row } });
    }

    if (request.path === `/api/v1/rooms/${DM}` && request.method === "GET") {
      return Promise.resolve({
        status: 200,
        json: { ...detail, membership: { ...row.membership, closed: true } },
      });
    }

    return handle(request);
  });
  mutations.loadSidebar({ ...closedSidebar, closedDirectRoomIds: [] }, sidebarRowClock());
  const since = mutations.openRowTicket();
  const router = await mount(`/r/${DM}`, true);

  await waitFor(() => expect(store.getState().sidebar.rows[DM]).toEqual(row));
  const navigate = vi.spyOn(router, "navigate");

  await act(async () => {
    mutations.loadSidebar(closedSidebar, since);
    mutations.closeRowTicket(since);
  });

  expect(store.getState().sidebar.rows[DM]).toEqual(row);
  expect(store.getState().directClosures.has(DM)).toBe(false);
  expect(navigate).not.toHaveBeenCalledWith(expect.objectContaining({ to: "/" }));
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
});

it("a local mark-read during resync does not hide a concurrent direct closure", async () => {
  const row = directRow();

  const unread = {
    ...row,
    unreadCount: 3,
    membership: { ...row.membership, unreadAt: row.evaluatedAt },
  };

  mutations.loadSidebar(
    { ...sidebar, rows: sidebar.rows.map((entry) => (entry.room.id === DM ? unread : entry)) },
    sidebarRowClock(),
  );
  const router = await mount(`/r/${DM}`);

  act(() => {
    const since = mutations.openRowTicket();
    mutations.markRoomRead(DM);
    expect(store.getState().sidebar.rows[DM]?.unreadCount).toBe(0);
    mutations.resyncSidebar(
      {
        ...sidebar,
        rows: sidebar.rows.filter((entry) => entry.room.id !== DM),
        closedDirectRoomIds: [DM],
      },
      since,
    );
    mutations.closeRowTicket(since);
  });

  expect(store.getState().sidebar.rows[DM]).toBeUndefined();
  expect(store.getState().directClosures.has(DM)).toBe(true);
  await waitFor(() => expect(router.state.location.pathname).toBe("/"));

  act(() => {
    const since = mutations.openRowTicket();
    const reopened = { ...row, unreadCount: 0, membership: { ...row.membership, unreadAt: null } };

    mutations.applyEvents(
      [{ seq: 1, topic: "user", type: "sidebar.row.upserted", data: reopened }],
      0,
    );
    mutations.loadSidebar(
      { ...sidebar, rows: sidebar.rows.map((entry) => (entry.room.id === DM ? unread : entry)) },
      since,
    );
    mutations.closeRowTicket(since);
  });

  expect(store.getState().sidebar.rows[DM]?.unreadCount).toBe(0);
  expect(store.getState().directClosures.has(DM)).toBe(false);
});

it("a sidebar resync reconciles a missed close and clears the last room", async () => {
  const router = await mount(`/r/${DM}`);

  act(() =>
    mutations.resyncSidebar(
      {
        ...sidebar,
        rows: sidebar.rows.filter((entry) => entry.room.id !== DM),
        closedDirectRoomIds: [DM],
      },
      sidebarRowClock(),
    ),
  );

  await waitFor(() => expect(router.state.location.pathname).toBe("/"));
  expect(store.getState().me?.lastRoomId).toBeNull();
});

it("a sidebar resync preserves the unavailable screen for a revoked direct", async () => {
  const router = await mount(`/r/${DM}`, true);
  const navigate = vi.spyOn(router, "navigate");
  await waitFor(() => expect(store.getState().rooms[DM]?.detail).toBeTruthy());

  act(() => {
    mutations.setRoomUnavailable(DM, beginRoomRequest(), sidebarRowClock());
    mutations.resyncSidebar(
      {
        ...sidebar,
        rows: sidebar.rows.filter((entry) => entry.room.id !== DM),
        closedDirectRoomIds: [],
      },
      sidebarRowClock(),
    );
  });

  expect(await screen.findByRole("region", { name: "Room unavailable" })).toBeTruthy();
  expect(navigate).not.toHaveBeenCalled();
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
});

it("a stale me reply cannot restore a last room that was closed", () => {
  const stale = store.getState().me;

  if (stale === null) throw new Error("The test needs a signed-in viewer");
  const since = mutations.openRowTicket();

  mutations.landReplyRows(
    [{ seq: 1, topic: "user", type: "sidebar.row.removed", data: { roomId: DM, closed: true } }],
    0,
    since,
  );
  expect(store.getState().me?.lastRoomId).toBeNull();

  mutations.loadSidebar(sidebar, since);
  mutations.closeRowTicket(since);
  mutations.setMe(stale);

  expect(store.getState().me?.lastRoomId).toBeNull();
  expect(store.getState().sidebar.rows[DM]).toBeUndefined();
});

it("a refused close shows the current unread row and keeps the room open", async () => {
  const row = directRow();
  const handle = network.server.handle.bind(network.server);

  vi.spyOn(network.server, "handle").mockImplementation((request) =>
    request.path === CLOSED_ENDPOINT && request.method === "PUT"
      ? Promise.resolve({
          status: 200,
          json: { row: { ...row, unreadCount: row.unreadCount + 1 } },
        })
      : handle(request),
  );

  const router = await mount(`/r/${DM}`);
  const user = userEvent.setup();
  const sidebarView = within(screen.getByRole("complementary", { name: "Conversations" }));

  await user.click(
    sidebarView.getByRole("button", {
      name: `Close conversation with ${row.sidebarDisplayName}`,
    }),
  );

  await waitFor(() =>
    expect(store.getState().sidebar.rows[DM]?.unreadCount).toBe(row.unreadCount + 1),
  );
  expect(router.state.location.pathname).toBe(`/r/${DM}`);
  expect(store.getState().me?.lastRoomId).toBe(DM);
});

it("a successful close retains focus handoff until its sync removal arrives", async () => {
  const row = directRow();
  const handle = network.server.handle.bind(network.server);
  const reply = Promise.withResolvers<Awaited<ReturnType<typeof handle>>>();
  vi.spyOn(network.server, "handle").mockImplementation((request) =>
    request.path === CLOSED_ENDPOINT && request.method === "PUT" ? reply.promise : handle(request),
  );
  const close = vi.spyOn(actions.organize, "closeDirect");

  await mount(`/r/${SEED_IDS.rooms.general}`);
  const user = userEvent.setup();
  const sidebarView = within(screen.getByRole("complementary", { name: "Conversations" }));
  const link = sidebarView.getByRole("link", { name: new RegExp(row.sidebarDisplayName) });

  const button = sidebarView.getByRole("button", {
    name: `Close conversation with ${row.sidebarDisplayName}`,
  });

  await user.hover(link);
  act(() => link.focus());
  await user.tab();
  await user.keyboard("{Enter}");
  expect(document.activeElement).toBe(button);

  await act(async () => {
    // A fresher sync row supersedes the HTTP reply, while the live removal is still on its way.
    mutations.applyEvents([{ seq: 1, topic: "user", type: "sidebar.row.upserted", data: row }], 0);
    reply.resolve({ status: 200, json: { row: null } });
    await close.mock.results[0]?.value;
  });
  expect(store.getState().sidebar.rows[DM]).toBeDefined();

  act(() =>
    mutations.applyEvents(
      [{ seq: 2, topic: "user", type: "sidebar.row.removed", data: { roomId: DM, closed: true } }],
      0,
    ),
  );
  expect(document.activeElement).toBe(sidebarView.getByRole("link", { name: "engineering" }));
});
