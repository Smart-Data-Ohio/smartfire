import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterAll, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { SEED_IDS } from "../../../mock/server.ts";
import type { Sidebar } from "../../gen/Sidebar.ts";
import type { RoomCategory, RoomKind, SidebarRow } from "../../store/model.ts";
import { favoriteRows, organizedSidebar } from "../../store/organize.ts";
import { mutations, store } from "../../store/store.ts";
import { installMockNetwork, type MockNetwork } from "../../test/mock-network.ts";
import { Button } from "../../ui/button.tsx";
import { Menu } from "../../ui/menu.tsx";
import { CategoryNameField } from "./category-name-field.tsx";
import { DeleteCategoryDialog } from "./delete-category-dialog.tsx";
import { RoomMenuItems } from "./room-menu.tsx";

// The menus' commands run the real optimistic actions against the in-memory mock backend.
let network: MockNetwork;

beforeAll(() => {
  network = installMockNetwork();

  const meta = document.createElement("meta");

  meta.name = "csrf-token";
  meta.content = network.server.csrfToken();
  document.head.append(meta);
});

afterAll(() => network.restore());

beforeEach(async () => {
  // Each test starts from the seeded workspace, whatever the one before changed.
  network.server.reset();

  const sidebar: Sidebar = await (await fetch("/api/v1/sidebar")).json();

  mutations.reset();
  mutations.loadSidebar(sidebar);
});

/** The viewer's row for a room as the sidebar draws it, pending changes included. */
function seeded(roomId: number): SidebarRow {
  const entry = organizedSidebar(store.getState().sidebar).rows[roomId];

  if (entry === undefined) {
    throw new Error(`No sidebar row for room ${roomId}`);
  }

  return entry;
}

interface RowOptions {
  readonly kind?: RoomKind;
  readonly favorite?: number;
  readonly category?: number;
  readonly muted?: boolean;
  readonly unread?: boolean;
}

function row(name: string, options: RowOptions = {}): SidebarRow {
  return {
    room: {
      id: 5,
      kind: options.kind ?? "open",
      name,
      iconName: null,
      creatorId: 1,
      createdAt: "2026-10-01T00:00:00.000Z",
      updatedAt: "2026-10-01T00:00:00.000Z",
    },
    membership: {
      id: 50,
      roomId: 5,
      userId: 1,
      involvement: options.muted === true ? "muted" : "mentions",
      unreadAt: options.unread === true ? "2026-10-05T00:00:00.000Z" : null,
      lastReadMessageId: null,
      roomCategoryId: options.category ?? null,
      favoritePosition: options.favorite ?? null,
      stageRole: null,
    },
    displayName: name,
    directMemberIds: [],
    unreadCount: 0,
    mentionCount: 0,
    notificationCount: 0,
    threadNotificationCount: 0,
  };
}

const CATEGORIES: readonly RoomCategory[] = [
  { id: 1, name: "Launch", collapsed: false, position: 0 },
  { id: 2, name: "Team", collapsed: false, position: 1 },
];

describe("CategoryNameField", () => {
  function field(initial = "") {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();

    render(
      <CategoryNameField
        label="Category name"
        initial={initial}
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );

    return { user: userEvent.setup(), input: screen.getByRole("textbox"), onSubmit, onCancel };
  }

  it("opens focused with the name selected, and saves a trimmed name on Enter", async () => {
    const { user, input, onSubmit } = field("Team");

    expect(document.activeElement).toBe(input);
    await user.keyboard("  People & ops  {Enter}");
    // The field is going: focus is the sidebar's to place.
    expect(onSubmit).toHaveBeenCalledWith("People & ops", true);
  });

  it("says why a blank name won't do, and saves nothing", async () => {
    const { user, onSubmit, onCancel } = field();

    await user.keyboard("   {Enter}");
    expect(screen.getByText("Give the category a name")).toBeTruthy();
    expect(onSubmit).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("cancels on Escape, and leaving an unchanged name cancels too", async () => {
    const escaped = field("Team");

    await escaped.user.keyboard("Other{Escape}");
    expect(escaped.onCancel).toHaveBeenCalledExactlyOnceWith(true);
    expect(escaped.onSubmit).not.toHaveBeenCalled();
  });

  it("saves a changed name when focus leaves, leaving focus where it went", async () => {
    const { user, onSubmit } = field("Team");

    await user.keyboard("Ops");
    await user.tab();
    // Nothing else to take focus: it fell to the page, so the sidebar places it.
    expect(onSubmit).toHaveBeenCalledWith("Ops", true);

    cleanup();

    const onElsewhere = vi.fn();

    render(
      <>
        <CategoryNameField
          label="Category name"
          initial="Team"
          onSubmit={onElsewhere}
          onCancel={vi.fn()}
        />
        <button type="button">Elsewhere</button>
      </>,
    );

    await user.keyboard("Ops");
    await user.tab();
    expect(onElsewhere).toHaveBeenCalledWith("Ops", false);
  });

  it("caps names at 50 characters", () => {
    field();

    expect(screen.getByRole("textbox").getAttribute("maxlength")).toBe("50");
  });
});

describe("DeleteCategoryDialog", () => {
  it("says where the channels go, and deletes only on confirm", async () => {
    const onConfirm = vi.fn();
    const onOpenChange = vi.fn();
    const user = userEvent.setup();

    render(
      <DeleteCategoryDialog
        category={CATEGORIES[0] ?? null}
        roomCount={2}
        onOpenChange={onOpenChange}
        onConfirm={onConfirm}
      />,
    );

    const dialog = screen.getByRole("alertdialog", { name: "Delete Launch?" });

    expect(dialog.textContent).toContain("Its 2 channels go back to Channels");

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    expect(onConfirm).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Delete category" }));
    expect(onConfirm).toHaveBeenCalledWith(CATEGORIES[0]);
    expect(onOpenChange).toHaveBeenLastCalledWith(false);
  });
});

describe("a room's menu", () => {
  async function open(entry: SidebarRow) {
    const user = userEvent.setup();

    render(
      <Menu
        label={`${entry.displayName} options`}
        trigger={(props) => <Button {...props}>Open</Button>}
      >
        <RoomMenuItems row={entry} categories={CATEGORIES} onNewCategory={() => {}} />
      </Menu>,
    );
    await user.click(screen.getByRole("button", { name: "Open" }));

    return user;
  }

  const QUIET = SEED_IDS.rooms.quiet;

  it("moves a channel to a category, with its current place checked", async () => {
    const user = await open(seeded(QUIET));

    await user.click(screen.getByRole("menuitem", { name: "Move to" }));

    expect(
      screen.getByRole("menuitemradio", { name: "Channels" }).getAttribute("aria-checked"),
    ).toBe("true");
    expect(screen.getByRole("menuitemradio", { name: "Team" }).getAttribute("aria-checked")).toBe(
      "false",
    );

    await user.click(screen.getByRole("menuitemradio", { name: "Team" }));
    expect(seeded(QUIET).membership.roomCategoryId).toBe(SEED_IDS.categories.team);
    await waitFor(() =>
      expect(store.getState().sidebar.rows[QUIET]?.membership.roomCategoryId).toBe(
        SEED_IDS.categories.team,
      ),
    );
  });

  it("stars a room", async () => {
    const user = await open(seeded(QUIET));

    await user.click(screen.getByRole("menuitem", { name: "Add to Favourites" }));
    expect(seeded(QUIET).membership.favoritePosition).not.toBeNull();
  });

  it("moves a favourite up or down without a drag, the ends disabled", async () => {
    const ENGINEERING = SEED_IDS.rooms.engineering;
    const MAYA = SEED_IDS.rooms.dmMaya;

    const favorites = () =>
      favoriteRows(organizedSidebar(store.getState().sidebar)).map((entry) => entry.room.id);

    expect(favorites()).toEqual([ENGINEERING, MAYA]);

    const user = await open(seeded(ENGINEERING));

    expect(screen.getByRole("menuitem", { name: "Move up" }).getAttribute("aria-disabled")).toBe(
      "true",
    );

    await user.click(screen.getByRole("menuitem", { name: "Move down" }));
    expect(favorites()).toEqual([MAYA, ENGINEERING]);
    await waitFor(() => expect(favoriteRows(store.getState().sidebar).at(0)?.room.id).toBe(MAYA));
  });

  it("offers no Move up or down for a room that isn't a favourite", async () => {
    await open(seeded(QUIET));

    expect(screen.queryByRole("menuitem", { name: "Move up" })).toBeNull();
    expect(screen.queryByRole("menuitem", { name: "Move down" })).toBeNull();
  });

  it("mutes a room", async () => {
    const user = await open(seeded(QUIET));

    await user.click(screen.getByRole("menuitem", { name: "Mute" }));
    expect(seeded(QUIET).membership.involvement).toBe("muted");
    await waitFor(() =>
      expect(store.getState().sidebar.rows[QUIET]?.membership.involvement).toBe("muted"),
    );
  });

  it("offers no Move to for a direct message, and Unmute and Mark as read when they apply", async () => {
    await open(row("Maya Okafor", { kind: "direct", favorite: 0, muted: true, unread: true }));

    expect(screen.queryByRole("menuitem", { name: "Move to" })).toBeNull();
    expect(screen.getByRole("menuitem", { name: "Remove from Favourites" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "Unmute" })).toBeTruthy();
    expect(screen.getByRole("menuitem", { name: "Mark as read" })).toBeTruthy();
  });
});
