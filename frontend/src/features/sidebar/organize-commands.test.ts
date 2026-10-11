import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { meFixture, sidebarFixture, sidebarRowFixture } from "../../api/testing.ts";
import { mutations, sidebarRowClock } from "../../store/store.ts";
import { ActionError } from "../../sync/run.ts";
import { actions } from "../../sync/runtime.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { moveRoom, moveSharedRoom } from "./organize-commands.ts";

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

beforeEach(() => {
  mutations.setMe(meFixture);
  mutations.loadSidebar(sidebarFixture([starred]), sidebarRowClock());
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

  it("leaves the room where a personal move made while the shared one was out put it", async () => {
    let land = () => {};

    vi.spyOn(actions.workspaceLayout, "moveRoom").mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          land = resolve;
        }),
    );

    const personal = vi.spyOn(actions.organize, "moveRoom").mockResolvedValue(undefined);
    const shared = moveSharedRoom(starred, 30, null);

    moveRoom(starred, { kind: "category", categoryId: 7 });
    land();

    expect(await shared).toBe(false);
    expect(personal.mock.calls).toEqual([[1, { kind: "category", categoryId: 7 }]]);
  });

  it("leaves the cleanup to the latest of two drops of the same room", async () => {
    const landings: (() => void)[] = [];

    vi.spyOn(actions.workspaceLayout, "moveRoom").mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          landings.push(resolve);
        }),
    );

    const personal = vi.spyOn(actions.organize, "moveRoom").mockResolvedValue(undefined);
    const first = moveSharedRoom(starred, 30, null);
    const second = moveSharedRoom(starred, 30, null);

    for (const land of landings) {
      land();
    }

    expect(await first).toBe(false);
    expect(await second).toBe(true);
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
