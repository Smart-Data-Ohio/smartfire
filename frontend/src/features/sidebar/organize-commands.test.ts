import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import type { SidebarRow } from "../../store/model.ts";
import { mutations, sidebarRowClock } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { moveSharedRoom } from "./organize-commands.ts";

afterEach(() => {
  vi.restoreAllMocks();
  mutations.reset();

  for (const shown of toastSnapshot()) {
    removeToast(shown.id);
  }
});

/** #general, starred: a drop on the shared layout takes it out of the favourites. */
const starred = (() => {
  const row = sidebarRowFixture(1, "general");

  return { ...row, membership: { ...row.membership, favoritePosition: 0 } };
})();

const team = { id: 7, name: "Team", collapsed: false, position: 0 };

let seq = 0;

/** #general's membership changing as `sidebar.row.upserted` brings it (another tab, or a reply). */
function upserted(change: Partial<SidebarRow["membership"]>): void {
  seq += 1;
  mutations.applyEvents(
    [
      {
        seq,
        topic: `user:${meFixture.user.id}`,
        type: "sidebar.row.upserted",
        data: { ...starred, membership: { ...starred.membership, ...change } },
      },
    ],
    Date.now(),
  );
}

/** A shared move that lands when the test says so. */
function heldSharedMoves(): (() => void)[] {
  const landings: (() => void)[] = [];

  vi.spyOn(actions.workspaceLayout, "moveRoom").mockImplementation(
    () =>
      new Promise<void>((resolve) => {
        landings.push(resolve);
      }),
  );

  return landings;
}

/** The personal move, landing at once in the store as its reply would. */
function personalMoves() {
  return vi.spyOn(actions.organize, "moveRoom").mockImplementation(async () => {
    upserted({ favoritePosition: null, roomCategoryId: null });
  });
}

beforeEach(() => {
  mutations.setMe(meFixture);
  mutations.loadSidebar({ ...sidebarFixture([starred]), categories: [team] }, sidebarRowClock());
});

describe("moving a room for everyone", () => {
  it("takes the room out of its personal place once the shared move has landed", async () => {
    const order: string[] = [];

    vi.spyOn(actions.workspaceLayout, "moveRoom").mockImplementation(async () => {
      order.push("shared");
    });

    const personal = vi.spyOn(actions.organize, "moveRoom").mockImplementation(async () => {
      order.push("personal");
    });

    const leaving = vi.fn(() => order.push("leaving"));

    expect(await moveSharedRoom(starred, 30, null, true, leaving)).toBe(true);
    expect(order).toEqual(["shared", "leaving", "personal"]);
    expect(personal).toHaveBeenCalledWith(1, { kind: "channels" });
  });

  it("leaves the room where another tab put it while the shared move was out", async () => {
    const landings = heldSharedMoves();
    const personal = personalMoves();
    const shared = moveSharedRoom(starred, 30, null);

    upserted({ favoritePosition: null, roomCategoryId: team.id });

    for (const land of landings) {
      land();
    }

    expect(await shared).toBe(false);
    expect(personal).not.toHaveBeenCalled();
  });

  it("leaves the room where a newer star put it while the shared move was out", async () => {
    const landings = heldSharedMoves();
    const personal = personalMoves();
    const shared = moveSharedRoom(starred, 30, null);

    upserted({ favoritePosition: 3 });

    for (const land of landings) {
      land();
    }

    expect(await shared).toBe(false);
    expect(personal).not.toHaveBeenCalled();
  });

  it("takes the room out of its personal place once for two drops of it", async () => {
    const landings = heldSharedMoves();
    const personal = personalMoves();
    const first = moveSharedRoom(starred, 30, null);
    const second = moveSharedRoom(starred, 30, null);

    for (const land of landings) {
      land();
    }

    expect([await first, await second]).toEqual([true, false]);
    expect(personal.mock.calls).toEqual([[1, { kind: "channels" }]]);
  });

  it("keeps the room's personal place when the shared move is refused", async () => {
    vi.spyOn(actions.workspaceLayout, "moveRoom").mockRejectedValue(
      new ActionError("NotFound", "Workspace category not found"),
    );

    const personal = vi.spyOn(actions.organize, "moveRoom");

    await moveSharedRoom(starred, 30, null);

    expect(personal).not.toHaveBeenCalled();
    expect(toastSnapshot().map((shown) => shown.title)).toEqual([
      "Couldn't move general for everyone",
    ]);
  });

  it("leaves the personal place alone for a move from the menu", async () => {
    vi.spyOn(actions.workspaceLayout, "moveRoom").mockResolvedValue(undefined);

    const personal = vi.spyOn(actions.organize, "moveRoom");

    await moveSharedRoom(starred, 30, null, false);

    expect(personal).not.toHaveBeenCalled();
  });
});
