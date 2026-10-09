import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { RoomForm } from "../../gen/RoomForm.ts";
import { initialState } from "../../store/state.ts";
import { store } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import RoomSettingsDialog from "./room-settings-dialog.tsx";

const BOARD = 900;

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

async function mount() {
  const root = createRootRoute({
    component: () => (
      <>
        <RoomSettingsDialog roomId={BOARD} open onOpenChange={() => undefined} />
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
    history: createMemoryHistory({ initialEntries: [`/r/${BOARD}/settings`] }),
  });

  render(<RouterProvider router={router} />);
  await act(() => router.load());

  return router;
}

afterEach(() => {
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
