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
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ChannelWebhook } from "../../gen/ChannelWebhook.ts";
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
    topic: null,
    voiceBitrate: null,
    voiceUserLimit: null,
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
  let reject: (error: Error) => void = () => undefined;

  const promise = new Promise<T>((resolve, refuse) => {
    release = resolve;
    reject = refuse;
  });

  return { promise, release, reject };
}

async function mount(roomId = BOARD, closable = false) {
  function Harness() {
    const [open, setOpen] = useState(true);

    return (
      <>
        {closable ? (
          <button type="button" onClick={() => setOpen(true)}>
            Reopen settings
          </button>
        ) : null}
        <RoomSettingsDialog
          roomId={roomId}
          open={open}
          onOpenChange={closable ? setOpen : () => undefined}
        />
        <Outlet />
      </>
    );
  }

  const root = createRootRoute({
    component: Harness,
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
        topic: null,
        voiceBitrate: null,
        voiceUserLimit: null,
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
      topic: null,
      voiceBitrate: null,
      voiceUserLimit: null,
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

  it("disables Save while the member list is refreshing after a subscription", async () => {
    const refresh = held<RoomForm>();

    vi.spyOn(actions.rooms, "editForm")
      .mockResolvedValueOnce(closedForm())
      .mockImplementation(() => refresh.promise);
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(githubList());
    vi.spyOn(actions.rooms, "subscribeRepository").mockResolvedValue(RAILS);

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("rails/rails")).toBeTruthy();
    expect(await screen.findByText("Updating members…")).toBeTruthy();

    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "renamed");
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", true);
    await user.keyboard("{Enter}");
    expect(update).not.toHaveBeenCalled();

    refresh.release(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty(
        "disabled",
        false,
      ),
    );
    expect(screen.queryByText("Updating members…")).toBeNull();
    expect(screen.getByRole("tab", { name: "Members · 3" })).toBeTruthy();
  });

  it("blocks Save when the member refresh fails, and a retry keeps the GitHub bot", async () => {
    vi.spyOn(actions.rooms, "editForm")
      .mockResolvedValueOnce(closedForm())
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValue(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }));
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(githubList());
    vi.spyOn(actions.rooms, "subscribeRepository").mockResolvedValue(RAILS);

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("rails/rails")).toBeTruthy();

    expect(await screen.findByText("Couldn't update the member list.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();

    await user.click(screen.getByRole("tab", { name: "General" }));
    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "renamed");
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", true);
    await user.keyboard("{Enter}");
    expect(update).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty(
        "disabled",
        false,
      ),
    );
    expect(screen.queryByText("Couldn't update the member list.")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(CLOSED, {
        type: "closed",
        name: "renamed",
        userIds: [1, 2, BOT],
      }),
    );
  });

  it("clears a failed member refresh when the dialog closes and reopens", async () => {
    const refresh = held<RoomForm>();
    let edits = 0;

    vi.spyOn(actions.rooms, "editForm").mockImplementation(() => {
      edits += 1;

      if (edits === 1) return Promise.resolve(closedForm());

      if (edits === 2) return refresh.promise;

      return Promise.resolve(closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT] }));
    });
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(githubList());
    vi.spyOn(actions.rooms, "subscribeRepository").mockResolvedValue(RAILS);

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED, true);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("Updating members…")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Channel settings" })).toBeNull(),
    );

    await user.click(screen.getByRole("button", { name: "Reopen settings" }));
    await screen.findByRole("dialog", { name: "Channel settings" });
    expect(await screen.findByLabelText("Name")).toHaveProperty("value", "launch-planning");
    refresh.reject(new Error("offline"));
    await act(async () => {
      await refresh.promise.catch(() => undefined);
    });
    expect(screen.queryByText("Couldn't update the member list.")).toBeNull();
    expect(screen.queryByText("Updating members…")).toBeNull();

    await user.clear(screen.getByLabelText("Name"));
    await user.type(screen.getByLabelText("Name"), "renamed");
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", false);
    await user.click(screen.getByRole("button", { name: "Save changes" }));

    await waitFor(() =>
      expect(update).toHaveBeenCalledWith(CLOSED, {
        type: "closed",
        name: "renamed",
        userIds: [1, 2, BOT],
      }),
    );
  });

  it("keeps the newer member refresh when the older one lands after it", async () => {
    const stale = held<RoomForm>();
    const fresh = held<RoomForm>();

    vi.spyOn(actions.rooms, "editForm")
      .mockResolvedValueOnce(closedForm())
      .mockImplementationOnce(() => stale.promise)
      .mockImplementationOnce(() => fresh.promise);
    vi.spyOn(actions.rooms, "githubSubscriptions").mockResolvedValue(githubList());
    vi.spyOn(actions.rooms, "subscribeRepository")
      .mockResolvedValueOnce(RAILS)
      .mockResolvedValue({ id: 5, fullName: "campfire/campfire", events: ["opened"] });

    const update = vi.spyOn(actions.rooms, "update").mockResolvedValue(savedRoom("renamed"));
    const user = userEvent.setup();

    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "GitHub" }));

    // The second subscribe starts its refresh while the first one is still in flight.
    await user.type(screen.getByLabelText("Repository"), "rails/rails");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("Updating members…")).toBeTruthy();

    await user.type(screen.getByLabelText("Repository"), "campfire/campfire");
    await user.click(screen.getByRole("button", { name: "Subscribe" }));
    expect(await screen.findByText("Updating members…")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", true);

    // The newer response lands first: the bot the second subscribe added is saved.
    fresh.release(
      closedForm({ userIds: [1, 2, BOT], memberIds: [1, 2, BOT], candidateIds: [1, 2, 3, BOT] }),
    );
    await waitFor(() => expect(screen.getByRole("tab", { name: "Members · 3" })).toBeTruthy());
    expect(screen.queryByText("Updating members…")).toBeNull();
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", true);

    // The older response predates the bot. Applying it would unsave the bot it never saw, so
    // the member list and the save gate stay as the newer refresh left them.
    stale.release(closedForm());
    await act(async () => {
      await stale.promise;
    });
    expect(screen.getByRole("tab", { name: "Members · 3" })).toBeTruthy();
    expect(screen.queryByText("Unsaved changes")).toBeNull();
    expect(screen.queryByText("Couldn't update the member list.")).toBeNull();
    expect(screen.getByRole("button", { name: "Save changes" })).toHaveProperty("disabled", true);

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

describe("channel topic settings", () => {
  it("loads a textarea, counts Unicode characters, and saves a topic-only edit", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm({ topic: "Current topic" }));

    const update = vi
      .spyOn(actions.rooms, "update")
      .mockRejectedValue(new ActionError("ServerError", "held"));

    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("textbox", { name: "Topic" });
    expect(field.tagName).toBe("TEXTAREA");
    expect(field.getAttribute("value") ?? field.textContent).toBe("Current topic");
    expect(screen.getByText("13 / 1024")).toBeTruthy();
    await user.clear(field);
    await user.type(field, "  🦀 Planning  ");
    expect(screen.getByText("14 / 1024")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(update).toHaveBeenCalledWith(CLOSED, {
      type: "closed",
      userIds: [1, 2],
      topic: "🦀 Planning",
    });
  });

  it("allows 1024 Unicode characters and blocks a topic beyond the limit", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm());
    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("textbox", { name: "Topic" });
    await user.click(field);
    await user.paste("🦀".repeat(1024));
    expect(screen.getByText("1024 / 1024")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save changes" }).hasAttribute("disabled")).toBe(
      false,
    );
    await user.paste("x");
    expect(screen.getByText("1025 / 1024")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save changes" }).hasAttribute("disabled")).toBe(
      true,
    );
  });

  it("keeps the topic read-only for someone who cannot edit the name", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(
      closedForm({ canSubmit: false, topic: "Read me" }),
    );
    await mount(CLOSED);
    const field = await screen.findByRole("textbox", { name: "Topic" });
    expect(field.hasAttribute("disabled")).toBe(true);
  });
});

describe("voice bitrate settings", () => {
  it("shows the default, and saves a bitrate-only change", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm());

    const update = vi
      .spyOn(actions.rooms, "update")
      .mockRejectedValue(new ActionError("ServerError", "held"));

    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("combobox", { name: "Voice bitrate" });
    expect(field).toHaveProperty("value", "");
    expect(within(field).getByRole("option", { name: "Default (48 kbps)" })).toBeTruthy();

    expect(
      within(field)
        .getAllByRole("option")
        .map((option) => option.textContent),
    ).toEqual([
      "Default (48 kbps)",
      "48 kbps",
      "64 kbps",
      "96 kbps",
      "128 kbps",
      "256 kbps",
      "384 kbps",
    ]);

    expect(screen.getByRole("button", { name: "Save changes" }).hasAttribute("disabled")).toBe(
      true,
    );
    await user.selectOptions(field, "384 kbps");
    expect(screen.getByText("Unsaved changes")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(update).toHaveBeenCalledWith(CLOSED, {
      type: "closed",
      userIds: [1, 2],
      voiceBitrate: 384_000,
    });
  });

  it("resets a room's bitrate to the default", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm({ voiceBitrate: 96_000 }));

    const update = vi
      .spyOn(actions.rooms, "update")
      .mockRejectedValue(new ActionError("ServerError", "held"));

    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("combobox", { name: "Voice bitrate" });
    expect(field).toHaveProperty("value", "96000");
    await user.selectOptions(field, "Default (48 kbps)");
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(update).toHaveBeenCalledWith(CLOSED, {
      type: "closed",
      userIds: [1, 2],
      voiceBitrate: null,
    });
  });

  it("is only offered to those who may change the room, and never on a board", async () => {
    const edit = vi.spyOn(actions.rooms, "editForm");

    edit.mockResolvedValue(closedForm({ canSubmit: false, voiceBitrate: 128_000 }));
    await mount(CLOSED);
    await screen.findByRole("textbox", { name: "Topic" });
    expect(screen.queryByRole("combobox", { name: "Voice bitrate" })).toBeNull();
    cleanup();

    edit.mockResolvedValue(boardForm());
    await mount(BOARD);
    await screen.findByRole("dialog", { name: "Board settings" });
    await screen.findByRole("textbox", { name: "Topic" });
    expect(screen.queryByRole("combobox", { name: "Voice bitrate" })).toBeNull();
    cleanup();

    edit.mockResolvedValue(closedForm({ type: "voice", voiceBitrate: 256_000 }));
    await mount(CLOSED);
    const field = await screen.findByRole("combobox", { name: "Voice bitrate" });
    expect(field).toHaveProperty("value", "256000");
  });
});

describe("webhook settings", () => {
  const WEBHOOK: ChannelWebhook = {
    id: 21,
    roomId: CLOSED,
    creatorId: 1,
    name: "Deployments",
    avatarUrl: null,
    lastUsedAt: null,
    createdAt: "2026-10-03T15:00:00.000Z",
    updatedAt: "2026-10-03T15:00:00.000Z",
  };

  it("offers Webhooks only to those the API lets manage them, and never on a board", async () => {
    const list = vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [] });
    const edit = vi.spyOn(actions.rooms, "editForm");

    edit.mockResolvedValue(closedForm({ canSubmit: false, canDelete: false }));
    await mount(CLOSED);

    await screen.findByRole("dialog", { name: "Channel settings" });
    expect(screen.getByRole("tab", { name: "General" })).toBeTruthy();
    expect(screen.queryByRole("tab", { name: "Webhooks" })).toBeNull();
    cleanup();

    edit.mockResolvedValue(boardForm());
    await mount(BOARD);
    await screen.findByRole("dialog", { name: "Board settings" });
    expect(screen.queryByRole("tab", { name: "Webhooks" })).toBeNull();
    cleanup();

    edit.mockResolvedValue(closedForm());
    await mount(CLOSED);
    await screen.findByRole("dialog", { name: "Channel settings" });
    expect(screen.getByRole("tab", { name: "Webhooks" })).toBeTruthy();
    expect(list).not.toHaveBeenCalled();
  });

  it("forgets a webhook URL on show when the settings close", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm());
    vi.spyOn(actions.rooms, "channelWebhooks").mockResolvedValue({ webhooks: [] });
    vi.spyOn(actions.rooms, "createChannelWebhook").mockResolvedValue({
      webhook: WEBHOOK,
      token: "one-time-token",
      url: "https://chat.example.test/api/webhooks/21/one-time-token",
    });

    const user = userEvent.setup();

    await mount(CLOSED, true);
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "Webhooks" }));
    await user.type(await screen.findByLabelText("Name"), "Deployments");
    await user.click(screen.getByRole("button", { name: "Create webhook" }));
    expect(await screen.findByText(/one-time-token/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(document.body.textContent).not.toContain("one-time-token");
    await waitFor(() =>
      expect(screen.queryByRole("dialog", { name: "Channel settings" })).toBeNull(),
    );

    await user.click(screen.getByRole("button", { name: "Reopen settings" }));
    await screen.findByRole("dialog", { name: "Channel settings" });
    await user.click(screen.getByRole("tab", { name: "Webhooks" }));
    expect(await screen.findByText("No webhooks yet.")).toBeTruthy();
    expect(document.body.textContent).not.toContain("one-time-token");
    expect(JSON.stringify(store.getState())).not.toContain("one-time-token");
  });
});

describe("voice user limit settings", () => {
  it.each([1, 5, 99])("saves a limit of %s without changing other settings", async (limit) => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(
      closedForm({ type: "voice", conversionTypes: [] }),
    );

    const update = vi
      .spyOn(actions.rooms, "update")
      .mockRejectedValue(new ActionError("ServerError", "held"));

    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("combobox", { name: "Voice user limit" });
    expect(field).toHaveProperty("value", "");
    expect(screen.getByRole("combobox", { name: "Voice bitrate" })).toHaveProperty("value", "");
    await user.selectOptions(field, String(limit));
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(update).toHaveBeenCalledWith(CLOSED, {
      type: "voice",
      userIds: [1, 2],
      voiceUserLimit: limit,
    });
  });

  it("loads a limit and clears it with No limit", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(closedForm({ voiceUserLimit: 5 }));

    const update = vi
      .spyOn(actions.rooms, "update")
      .mockRejectedValue(new ActionError("ServerError", "held"));

    const user = userEvent.setup();
    await mount(CLOSED);
    const field = await screen.findByRole("combobox", { name: "Voice user limit" });
    expect(field).toHaveProperty("value", "5");
    expect(within(field).getAllByRole("option")).toHaveLength(100);
    await user.selectOptions(field, "");
    await user.click(screen.getByRole("button", { name: "Save changes" }));
    expect(update).toHaveBeenCalledWith(CLOSED, {
      type: "closed",
      userIds: [1, 2],
      voiceUserLimit: null,
    });
  });

  it("hides voice controls for a viewer who cannot administer the room", async () => {
    vi.spyOn(actions.rooms, "editForm").mockResolvedValue(
      closedForm({ canSubmit: false, voiceUserLimit: 5 }),
    );
    await mount(CLOSED);
    await screen.findByRole("textbox", { name: "Name" });
    expect(screen.queryByRole("combobox", { name: "Voice user limit" })).toBeNull();
    expect(screen.queryByRole("combobox", { name: "Voice bitrate" })).toBeNull();
  });
});
