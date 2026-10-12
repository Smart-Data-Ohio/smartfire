import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
  useParams,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, roomDetailFixture, userFixture } from "../../api/testing.ts";
import type { RoomKind } from "../../gen/RoomKind.ts";
import type { ThreadAccessMember } from "../../gen/ThreadAccessMember.ts";
import type { ThreadPermissions } from "../../gen/ThreadPermissions.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { THREAD_ACCESS_LOST } from "../../store/threads.ts";
import { panes } from "../../sync/panes.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { threadFixture } from "./test-fixtures.ts";
import { ThreadPane } from "./thread-pane.tsx";

/** The thread route. A thread that belongs to another room leaves this component. */
function ThreadRoute() {
  const { roomId, threadId } = useParams({ strict: false });

  if (roomId !== 4) {
    return <p>Opened in its room</p>;
  }

  return <ThreadPane roomId={roomId ?? 0} threadId={threadId ?? 0} />;
}

async function mount(path: string) {
  const root = createRootRoute({ component: Outlet });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    params: {
      parse: ({ roomId }) => ({ roomId: Number(roomId) }),
      stringify: ({ roomId }) => ({ roomId: String(roomId) }),
    },
    component: Outlet,
  });

  const thread = createRoute({
    getParentRoute: () => room,
    path: "t/$threadId",
    params: {
      parse: ({ threadId }) => ({ threadId: Number(threadId) }),
      stringify: ({ threadId }) => ({ threadId: String(threadId) }),
    },
    validateSearch: (search: { m?: unknown }) => {
      const m = Number(search.m);

      return Number.isSafeInteger(m) && m > 0 ? { m } : {};
    },
    component: ThreadRoute,
  });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([thread])]),
    basepath: "/app",
    history: createMemoryHistory({ initialEntries: [path] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

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

  // jsdom has no ResizeObserver; the composer's send button watches its size.
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

afterEach(() => {
  store.setState(initialState, true);
  vi.restoreAllMocks();
});

/** The pane opens itself here instead of fetching; only its controls matter. */
function stubPane(): void {
  vi.spyOn(actions.threads, "open").mockResolvedValue(undefined);
  vi.spyOn(actions.threads, "close").mockImplementation(() => undefined);
}

/** The thread the route names, with its pane loaded and the viewer's say over it. */
function readyPane(threadId: number, canRename: boolean, autoArchiveAfterMinutes = 1440): void {
  store.setState({
    ...initialState,
    threads: { [threadId]: threadFixture(threadId, { autoArchiveAfterMinutes }) },
    threadPanes: {
      [threadId]: {
        status: "ready",
        error: null,
        permissions: {
          canRename,
          canClose: canRename,
          canReopen: false,
          canLock: canRename,
          canUnlock: false,
          canDelete: canRename,
          canConvertWork: false,
          canManageWork: false,
          canUpdateWorkStatus: false,
          canAssignWork: false,
          canRemoveWork: false,
        } satisfies ThreadPermissions,
        work: null,
        workFacts: null,
      },
    },
  });
}

/** Room 4 loaded as a room of `kind`, so the pane knows whether its threads are board posts. */
function roomOfKind(roomId: number, kind: RoomKind): void {
  const detail = roomDetailFixture(roomId);

  store.setState({
    rooms: {
      [roomId]: {
        detail: { ...detail, room: { ...detail.room, kind } },
        status: "ready",
        error: null,
        preview: null,
      },
    },
  });
}

describe("ThreadPane", () => {
  it("sends a thread from another room to the room it belongs to", async () => {
    vi.spyOn(actions.threads, "open").mockResolvedValue(undefined);
    vi.spyOn(actions.threads, "close").mockImplementation(() => undefined);
    store.setState({
      ...initialState,
      threads: { 9: threadFixture(9, { roomId: 8 }) },
    });

    const router = await mount("/app/r/4/t/9?m=3");

    await waitFor(() => expect(screen.getByText("Opened in its room")).toBeTruthy());
    expect(router.history.location.href).toBe("/app/r/8/t/9?m=3");
  });

  it("changes how long an idle thread stays open", async () => {
    stubPane();
    readyPane(9, true);
    const update = vi.spyOn(actions.threads, "update").mockResolvedValue(undefined);
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));
    await user.click(screen.getByRole("menuitem", { name: /Auto-archive after/ }));

    const dialog = await screen.findByRole("dialog");
    // SAFETY: the field labelled Auto-archive after is the <select>.
    const picker = within(dialog).getByLabelText("Auto-archive after") as HTMLSelectElement;

    expect(picker.value).toBe("1440");

    await user.selectOptions(picker, "10080");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(9, {
        name: null,
        autoArchiveAfterMinutes: 10080,
        status: null,
      }),
    );
    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());
  });

  it("keeps the picker open and says why when the server refuses the duration", async () => {
    stubPane();
    readyPane(9, true);
    const message = "Auto archive after minutes must be one of 60, 1440, 4320 or 10080";

    vi.spyOn(actions.threads, "update").mockRejectedValue(
      new ActionError("Validation", message, { autoArchiveAfterMinutes: [message] }),
    );
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));
    await user.click(screen.getByRole("menuitem", { name: /Auto-archive after/ }));

    const dialog = await screen.findByRole("dialog");
    const picker = within(dialog).getByLabelText("Auto-archive after");

    await user.selectOptions(picker, "60");
    await user.click(within(dialog).getByRole("button", { name: "Save" }));

    expect(await screen.findByText(message)).toBeTruthy();
    expect(picker.getAttribute("aria-invalid")).toBe("true");
    expect(screen.getByRole("dialog")).toBeTruthy();
  });

  it("offers no auto-archive change on a board post", async () => {
    stubPane();
    readyPane(9, true);
    roomOfKind(4, "board");
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Post actions" }));

    expect(screen.getByRole("menuitem", { name: /Rename post/ })).toBeTruthy();
    expect(screen.queryByRole("menuitem", { name: /Auto-archive after/ })).toBeNull();
  });

  it("offers the auto-archive change on an ordinary thread", async () => {
    stubPane();
    readyPane(9, true);
    roomOfKind(4, "open");
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));

    expect(screen.getByRole("menuitem", { name: /Auto-archive after/ })).toBeTruthy();
  });

  it("offers no auto-archive change to a viewer who may only reply", async () => {
    stubPane();
    readyPane(9, false);
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(screen.getByRole("button", { name: "Thread actions" }));

    expect(screen.queryByRole("menuitem", { name: /Auto-archive after/ })).toBeNull();
  });
});

const VIEWER = meFixture.user.id;

function member(userId: number): ThreadAccessMember {
  return { userId, addedById: 2, createdAt: "2026-10-06T08:00:00.000Z" };
}

/** A private thread `creatorId` started, its pane ready, with the viewer and people loaded. */
function privatePane(creatorId: number, administrator = false): void {
  readyPane(9, false);
  store.setState((state) => ({
    me: administrator
      ? { ...meFixture, user: { ...meFixture.user, role: "administrator" as const } }
      : meFixture,
    threads: {
      9: { ...state.threads[9], ...threadFixture(9, { creatorId, visibility: "private" }) },
    },
    users: {
      [VIEWER]: meFixture.user,
      2: userFixture(2, "Grace Hopper"),
      3: userFixture(3, "Katherine Johnson"),
    },
  }));
}

describe("ThreadPane, private", () => {
  it("marks a private thread and lists who can see it", async () => {
    stubPane();
    privatePane(2);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(2), member(VIEWER)],
      users: [],
    });
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");

    expect(screen.getByText("Private")).toBeTruthy();

    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });
    const rows = within(dialog).getAllByRole("listitem");

    // The viewer first, then the creator.
    expect(rows[0]?.textContent).toContain("Ada Lovelace (you)");
    expect(rows[1]?.textContent).toContain("Grace Hopper");
    expect(rows[1]?.querySelector(".thread-access-tag")?.textContent).toBe("Creator");
    // Neither the creator nor an admin: nobody to remove, but the viewer may leave.
    expect(within(dialog).queryByRole("button", { name: /^Remove/ })).toBeNull();
    expect(within(dialog).getByRole("button", { name: "Leave" })).toBeTruthy();
  });

  it("shows no private mark or members on a public thread", async () => {
    stubPane();
    readyPane(9, false);
    const members = vi.spyOn(actions.threads, "accessMembers");

    await mount("/app/r/4/t/9");

    expect(screen.queryByText("Private")).toBeNull();
    expect(screen.queryByRole("button", { name: /^Members/ })).toBeNull();
    expect(members).not.toHaveBeenCalled();
  });

  it("adds someone from the room at once", async () => {
    stubPane();
    privatePane(2);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(2), member(VIEWER)],
      users: [],
    });
    vi.spyOn(panes, "members").mockResolvedValue({
      members: [VIEWER, 2, 3].map((userId) => ({
        userId,
        presence: "online" as const,
        statusText: null,
        starred: false,
      })),
      users: [],
    });

    const invite = vi.spyOn(actions.threads, "invite").mockResolvedValue({
      members: [member(2), member(VIEWER), member(3)],
      users: [],
    });

    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    await user.type(within(dialog).getByRole("combobox", { name: "Add people" }), "kath");
    // People already in it aren't offered.
    expect(within(dialog).queryByRole("option", { name: /Grace/ })).toBeNull();
    await user.click(await within(dialog).findByRole("option", { name: /Katherine Johnson/ }));

    expect(invite).toHaveBeenCalledWith(9, 3);
    await waitFor(() => expect(within(dialog).getAllByRole("listitem")).toHaveLength(3));
    expect(screen.getByRole("button", { name: "Members: 3 people" })).toBeTruthy();
  });

  it("follows people others add or remove while the list is open", async () => {
    stubPane();
    privatePane(2);
    let roster = [member(2), member(VIEWER)];

    vi.spyOn(actions.threads, "accessMembers").mockImplementation(() =>
      Promise.resolve({ members: roster, users: [] }),
    );
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    await waitFor(() => expect(within(dialog).getAllByRole("listitem")).toHaveLength(2));

    // Grace added Katherine: every reader gets the thread's record again.
    roster = [member(2), member(VIEWER), member(3)];
    act(() => {
      store.setState((state) => ({
        threads: {
          ...state.threads,
          9: { ...threadFixture(9, { creatorId: 2, visibility: "private" }) },
        },
      }));
    });

    await waitFor(() => expect(within(dialog).getAllByRole("listitem")).toHaveLength(3));
    expect(screen.getByRole("button", { name: "Members: 3 people" })).toBeTruthy();
  });

  it("keeps an invite when a list read sent before it answers after it", async () => {
    stubPane();
    privatePane(2);
    vi.spyOn(panes, "members").mockResolvedValue({
      members: [VIEWER, 2, 3].map((userId) => ({
        userId,
        presence: "online" as const,
        statusText: null,
        starred: false,
      })),
      users: [],
    });

    const before = { members: [member(2), member(VIEWER)], users: [] };
    let answerLate: (list: typeof before) => void = () => undefined;

    vi.spyOn(actions.threads, "accessMembers")
      .mockResolvedValueOnce(before)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            answerLate = resolve;
          }),
      );

    vi.spyOn(actions.threads, "invite").mockResolvedValue({
      members: [member(2), member(VIEWER), member(3)],
      users: [],
    });

    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    // Opening the list reads it again; that read is held back.
    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    await user.type(within(dialog).getByRole("combobox", { name: "Add people" }), "kath");
    await user.click(await within(dialog).findByRole("option", { name: /Katherine Johnson/ }));
    await waitFor(() => expect(within(dialog).getAllByRole("listitem")).toHaveLength(3));

    await act(async () => {
      answerLate(before);
      await Promise.resolve();
    });

    expect(within(dialog).getAllByRole("listitem")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "Members: 3 people" })).toBeTruthy();
  });

  it("says why an invite was refused", async () => {
    stubPane();
    privatePane(2);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(VIEWER)],
      users: [],
    });
    vi.spyOn(panes, "members").mockResolvedValue({
      members: [VIEWER, 3].map((userId) => ({
        userId,
        presence: "online" as const,
        statusText: null,
        starred: false,
      })),
      users: [],
    });
    const refusal = "Only active members of this room can be added to the thread.";

    vi.spyOn(actions.threads, "invite").mockRejectedValue(new ActionError("ThreadAccess", refusal));
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(await screen.findByRole("button", { name: "Members: 1 person" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    await user.type(within(dialog).getByRole("combobox", { name: "Add people" }), "kath{Enter}");

    expect((await within(dialog).findByRole("alert")).textContent).toBe(refusal);
    expect(within(dialog).getAllByRole("listitem")).toHaveLength(1);
  });

  it("lets the creator remove someone", async () => {
    stubPane();
    privatePane(VIEWER);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(VIEWER), member(2)],
      users: [],
    });
    const remove = vi.spyOn(actions.threads, "removeMember").mockResolvedValue(undefined);
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    await user.click(within(dialog).getByRole("button", { name: "Remove Grace Hopper" }));

    expect(remove).toHaveBeenCalledWith(9, 2);
    await waitFor(() => expect(within(dialog).getAllByRole("listitem")).toHaveLength(1));
    expect(screen.getByRole("button", { name: "Members: 1 person" })).toBeTruthy();
  });

  it("lets an admin who wasn't added see the members and remove them, but not leave", async () => {
    stubPane();
    privatePane(2, true);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(2), member(3)],
      users: [],
    });
    const user = userEvent.setup();

    await mount("/app/r/4/t/9");
    await user.click(await screen.findByRole("button", { name: "Members: 2 people" }));

    const dialog = await screen.findByRole("dialog", { name: "Private thread members" });

    expect(within(dialog).getByText(/because you're an admin/)).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Remove Grace Hopper" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Remove Katherine Johnson" })).toBeTruthy();
    expect(within(dialog).queryByRole("button", { name: "Leave" })).toBeNull();
  });

  it("leaves a private thread after confirming, then goes back to the room", async () => {
    stubPane();
    privatePane(2);
    vi.spyOn(actions.threads, "accessMembers").mockResolvedValue({
      members: [member(2), member(VIEWER)],
      users: [],
    });
    const remove = vi.spyOn(actions.threads, "removeMember").mockResolvedValue(undefined);
    const user = userEvent.setup();

    const router = await mount("/app/r/4/t/9");

    await screen.findByRole("button", { name: "Members: 2 people" });
    await user.click(screen.getByRole("button", { name: "Thread actions" }));
    await user.click(screen.getByRole("menuitem", { name: /Leave thread/ }));

    const confirm = await screen.findByRole("alertdialog", { name: "Leave private thread" });

    await user.click(within(confirm).getByRole("button", { name: "Leave thread" }));

    expect(remove).toHaveBeenCalledWith(9, VIEWER);
    await waitFor(() => expect(router.history.location.pathname).toBe("/app/r/4"));
  });

  it("says when the viewer no longer has access, with no retry", async () => {
    stubPane();
    store.setState({
      ...initialState,
      threadPanes: {
        9: {
          status: "error",
          error: THREAD_ACCESS_LOST,
          permissions: null,
          work: null,
          workFacts: null,
        },
      },
    });

    await mount("/app/r/4/t/9");

    expect(screen.getByText(THREAD_ACCESS_LOST)).toBeTruthy();
    expect(screen.getByText("Someone who can still see it can add you back.")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  });
});
