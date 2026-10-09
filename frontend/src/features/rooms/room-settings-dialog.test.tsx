import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { GithubSubscriptionList } from "../../gen/GithubSubscriptionList.ts";
import type { InboundEmail } from "../../gen/InboundEmail.ts";
import type { RoomForm } from "../../gen/RoomForm.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { resetIntegrationSessions } from "./integration-session.ts";
import RoomSettingsDialog from "./room-settings-dialog.tsx";

const BOARD = 900;

const CLOSED = 7;

const BOT = 99;

function boardForm(patch: Partial<RoomForm> = {}): RoomForm {
  return {
    type: "board",
    roomId: BOARD,
    name: "Roadmap",
    iconName: null,
    displayName: "Roadmap",
    userIds: [1, 2],
    memberIds: [1, 2],
    candidateIds: [1, 2, 3],
    displayMemberIds: [],
    users: [],
    allowedTypes: ["open", "closed", "direct", "voice", "stage", "board"],
    conversionTypes: [],
    canSubmit: true,
    canDelete: true,
    canLeave: false,
    groupCapable: false,
    defaultInvolvement: "mentions",
    stageRoles: [],
    ...patch,
  };
}

function closedForm(patch: Partial<RoomForm> = {}): RoomForm {
  return {
    ...boardForm(),
    type: "closed",
    roomId: CLOSED,
    name: "launch-planning",
    displayName: "launch-planning",
    userIds: [1, 2],
    memberIds: [1, 2],
    conversionTypes: ["open", "closed"],
    ...patch,
  };
}

const EVENTS: GithubSubscriptionList["events"] = [
  { key: "opened", label: "Opened", selectedByDefault: true },
  { key: "merged", label: "Merged", selectedByDefault: true },
  { key: "closed", label: "Closed", selectedByDefault: false },
];

function githubList(
  subscriptions: GithubSubscriptionList["subscriptions"] = [],
): GithubSubscriptionList {
  return {
    subscriptions,
    administrator: true,
    connectPath: "/github/app/connect",
    events: EVENTS,
  };
}

function held<T>() {
  let release: (value: T) => void = () => undefined;

  const promise = new Promise<T>((resolve) => {
    release = resolve;
  });

  return { promise, release };
}

async function mount(roomId = BOARD) {
  const root = createRootRoute({
    component: () => (
      <>
        <RoomSettingsDialog roomId={roomId} open onOpenChange={() => undefined} />
        <Outlet />
      </>
    ),
  });

  const room = createRoute({
    getParentRoute: () => root,
    path: "/r/$roomId",
    component: Outlet,
  });

  const settings = createRoute({
    getParentRoute: () => room,
    path: "settings",
    component: () => null,
  });

  const automations = createRoute({
    getParentRoute: () => room,
    path: "automations",
    component: () => <p>Automations pane</p>,
  });

  const router = createRouter({
    routeTree: root.addChildren([room.addChildren([settings, automations])]),
    history: createMemoryHistory({ initialEntries: [`/r/${roomId}/settings`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => {
  resetIntegrationSessions();
  vi.restoreAllMocks();
  store.setState(initialState, true);
});

describe("board settings", () => {
  it("edits the board's name and members, and opens its automations", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(boardForm());

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue({
      room: {
        id: BOARD,
        kind: "board",
        name: "Shipped",
        iconName: null,
        creatorId: 1,
        createdAt: "2026-03-02T16:00:00Z",
        updatedAt: "2026-03-02T16:00:00Z",
      },
      detail: null,
      row: null,
    });

    const user = userEvent.setup();
    const router = await mount();
    const dialog = await screen.findByRole("dialog", { name: "Board settings" });

    expect(dialog.querySelector('[role="switch"]')).toBeNull();
    expect(screen.getByRole("tab", { name: "Members · 2" })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Board automations" }));
    expect(router.state.location.pathname).toBe(`/r/${BOARD}/automations`);

    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "Shipped");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(BOARD, {
        type: "board",
        name: "Shipped",
        userIds: [1, 2],
      }),
    );
  });

  it("refuses changes from a member who cannot administer", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(
      boardForm({ canSubmit: false, canDelete: false }),
    );

    await mount();
    const dialog = await screen.findByRole("dialog", { name: "Board settings" });

    expect(screen.getByLabelText("Name")).toHaveProperty("disabled", true);
    expect(
      screen.getByText("Only the person who made this board and administrators can change it."),
    ).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Save changes" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Board automations" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Delete…" })).toBeNull();
    expect(dialog.querySelector('[role="switch"]')).toBeNull();
    expect(screen.getAllByRole("button", { name: "Close" }).length).toBeGreaterThan(0);
  });

  it("keeps the dialog open when the server refuses the icon", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(boardForm());
    vi.spyOn(actions.rooms, "update").mockRejectedValue(
      new ActionError("Validation", "Icon name is not a known icon", {
        iconName: ["is not a known icon"],
      }),
    );

    const user = userEvent.setup();

    await mount();
    await screen.findByRole("dialog", { name: "Board settings" });
    await user.type(screen.getByLabelText("Name"), "!");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    expect(await screen.findByText("That icon is not a known icon.")).toBeTruthy();
    expect(screen.getByRole("dialog", { name: "Board settings" })).toBeTruthy();
  });
});

const RAILS = {
  id: 4,
  fullName: "rails/rails",
  events: ["opened", "merged"],
};

function savedRoom(name: string) {
  return {
    room: {
      id: CLOSED,
      kind: "closed" as const,
      name,
      iconName: null,
      creatorId: 1,
      createdAt: "2026-03-02T16:00:00Z",
      updatedAt: "2026-03-02T16:00:00Z",
    },
    detail: null,
    row: null,
  };
}

describe("room integration membership", () => {
  it("keeps the GitHub bot when a closed room is renamed after subscribing", async () => {
    vi.spyOn(actions.rooms, "editForm")
      .mockResolvedValueOnce(closedForm())
      .mockResolvedValue(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }));
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(githubList());
    vi.spyOn(actions.rooms, "subscribeRepository").mockResolvedValue(RAILS);

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("rails/rails")).toBeTruthy();
    expect(await screen.findByRole("tab", { name: "Members · 3" })).toBeTruthy();

    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "renamed");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(CLOSED, {
        type: "closed",
        name: "renamed",
        userIds: [1, 2, BOT],
      }),
    );
  });

  it("drops the GitHub bot after the last unsubscribe, and keeps it while one remains", async () => {
    const edit = vi
      .spyOn(actions.rooms, "editForm")
      .mockResolvedValueOnce(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }))
      .mockResolvedValueOnce(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }))
      .mockResolvedValue(closedForm({ userIds: [1, 2], memberIds: [1, 2] }));

    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(
      githubList([RAILS, { id: 5, fullName: "campfire/campfire", events: ["opened"] }]),
    );
    vi.spyOn(actions.rooms, "unsubscribeRepository").mockResolvedValue(RAILS);

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    await user.click(await screen.findByRole("button", { name: "Remove rails/rails" }));
    await user.click(
      within(screen.getByRole("alertdialog", { name: "Unsubscribe rails/rails?" })).getByRole(
        "button",
        { name: "Remove" },
      ),
    );
    expect(await screen.findByText("campfire/campfire")).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Members · 3" })).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Remove campfire/campfire" }));
    await user.click(
      within(screen.getByRole("alertdialog", { name: "Unsubscribe campfire/campfire?" })).getByRole(
        "button",
        { name: "Remove" },
      ),
    );
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
    expect(await screen.findByRole("tab", { name: "Members · 2" })).toBeTruthy();
    expect(edit).toHaveBeenCalledTimes(3);

    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "renamed");
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(CLOSED, {
        type: "closed",
        name: "renamed",
        userIds: [1, 2],
      }),
    );
  });

  it("shows a subscription that finished while the GitHub tab was unmounted", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm());

    const reads = held<GithubSubscriptionList>();
    let fetches = 0;

    vi.spyOn(actions.rooms, "githubSubscriptions").mockImplementation(() => {
      fetches += 1;

      if (fetches === 1) return Promise.resolve(githubList());

      return reads.promise.then(() => githubList());
    });

    const write = held<typeof RAILS>();

    vi.spyOn(actions.rooms, "subscribeRepository").mockImplementation(() => write.promise);

    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    expect(await screen.findByText("No repositories subscribed yet.")).toBeTruthy();
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    expect(fetches).toBe(2);
    expect(screen.getByRole("button", { name: "Subscribe" })).toHaveProperty("disabled", true);
    expect(screen.getByText("No repositories subscribed yet.")).toBeTruthy();

    write.release(RAILS);
    expect(await screen.findByText("rails/rails")).toBeTruthy();

    reads.release(githubList());
    await act(async () => {
      await reads.promise;
    });
    expect(screen.getByText("rails/rails")).toBeTruthy();
    expect(screen.queryByText("No repositories subscribed yet.")).toBeNull();
  });

  it("shows an email address that finished rotating while the Email tab was unmounted", async () => {
    const created: InboundEmail = {
      enabled: true,
      address: "room-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb@mail.campfire.test",
    };

    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm());

    const reads = held<InboundEmail>();
    let fetches = 0;

    vi.spyOn(actions.rooms, "inboundEmail").mockImplementation(() => {
      fetches += 1;

      if (fetches === 1) return Promise.resolve({ enabled: true, address: null });

      return reads.promise.then(() => ({ enabled: true, address: null }));
    });

    const write = held<InboundEmail>();

    vi.spyOn(actions.rooms, "rotateInboundEmail").mockImplementation(() => write.promise);

    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "Email" }));
    expect(await screen.findByText(/No email address yet/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Create email address" }));
    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.click(screen.getByRole("tab", { name: "Email" }));
    expect(fetches).toBe(2);
    expect(
      screen.getByRole("button", { name: "Create email address" }).getAttribute("aria-busy"),
    ).toBe("true");
    expect(screen.getByText(/No email address yet/)).toBeTruthy();

    write.release(created);
    expect(await screen.findByText(/room-bbbb/)).toBeTruthy();

    reads.release({ enabled: true, address: null });
    await act(async () => {
      await reads.promise;
    });
    expect(screen.getByText(/room-bbbb/)).toBeTruthy();
    expect(screen.queryByText(/No email address yet/)).toBeNull();
  });
});
